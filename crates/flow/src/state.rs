//! Durable state: config, journal (append + fsync), receipts.
//!
//! Guarantees stated by this module, and only these:
//! * an intent appended and fsynced is not forgotten;
//! * an outcome is appended after the ref update attempt;
//! * a torn journal line is refused (`InvalidData`), never silently skipped.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::gitcli::Obligation;
use crate::json::{self, Json};

pub const POLICY_VERSION: &str = "FLOW-POLICY-1";
pub const SCHEMA_CONFIG: &str = "flow-config/1";
/// Bumped whenever the receipt's semantics change. A receipt written under an
/// older schema is never silently reinterpreted: publication refuses it.
pub const SCHEMA_RECEIPT: &str = "flow-receipt/2";
pub const SCHEMA_JOURNAL: &str = "flow-journal/1";
/// A proposal record: immutable, attributed, keyed by idempotency key.
pub const SCHEMA_PROPOSAL: &str = "flow-proposal/1";
/// A candidate record: an immutable exact tree assembled from proposals.
pub const SCHEMA_CANDIDATE: &str = "flow-candidate/1";
/// The local adoption record: which policy digest this working copy approved.
pub const SCHEMA_ADOPTION: &str = "flow-adoption/1";
/// A publication plan: the full preflight of a publish, with no mutation.
pub const SCHEMA_PLAN: &str = "flow-plan/1";
/// The read-only status document: what the working copy holds and what it owes.
pub const SCHEMA_STATUS: &str = "flow-status/1";
/// The proposal listing: a list of immutable proposal records.
pub const SCHEMA_PROPOSAL_LIST: &str = "flow-proposal-list/1";
/// Identity of the way a context digest is computed. Part of the digest itself,
/// so a future codec change invalidates older receipts instead of reusing them.
pub const CONTEXT_CODEC: &str = "flow-context/2";

/// A journal read failure, kept typed so the CLI can distinguish "the file
/// could not be read" (outcome unknown) from "the file is torn" (corruption).
/// Collapsing both into an empty journal is the P0-C defect.
#[derive(Debug)]
pub enum JournalError {
    /// The journal could not be opened or read: the outcome is genuinely unknown.
    Io(String),
    /// A line was read but is not a valid record: the journal is corrupt.
    Corrupt(String),
}

impl std::fmt::Display for JournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JournalError::Io(m) => write!(f, "journal unreadable: {m}"),
            JournalError::Corrupt(m) => write!(f, "journal corrupt: {m}"),
        }
    }
}

impl From<JournalError> for String {
    fn from(e: JournalError) -> String {
        e.to_string()
    }
}

/// Machine state reported when recovery cannot decide on its own.
pub const STATE_CORRUPT_JOURNAL: &str = "CORRUPT_JOURNAL";
pub const STATE_UNKNOWN_OUTCOME: &str = "UNKNOWN_OUTCOME";
pub const STATE_NEEDS_RECONCILIATION: &str = "NEEDS_RECONCILIATION";

pub struct Config {
    pub policy_version: String,
    pub execution_boundary: String,
    pub obligations: Vec<Obligation>,
    pub disabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            policy_version: POLICY_VERSION.to_string(),
            execution_boundary: "local-trusted".to_string(),
            obligations: Vec::new(),
            disabled: false,
        }
    }
}

impl Config {
    pub fn to_json(&self) -> Json {
        let obs = self
            .obligations
            .iter()
            .map(|o| {
                Json::obj(vec![
                    ("id", Json::str(o.id.clone())),
                    ("program", Json::str(o.program.clone())),
                    (
                        "args",
                        Json::Arr(o.args.iter().map(|a| Json::str(a.clone())).collect()),
                    ),
                    ("required", Json::Bool(o.required)),
                    ("timeout_s", Json::Num(o.timeout_s as f64)),
                ])
            })
            .collect();
        Json::obj(vec![
            ("schema", Json::str(SCHEMA_CONFIG)),
            ("policy_version", Json::str(self.policy_version.clone())),
            (
                "execution_boundary",
                Json::str(self.execution_boundary.clone()),
            ),
            ("obligations", Json::Arr(obs)),
            ("disabled", Json::Bool(self.disabled)),
        ])
    }

    pub fn from_json(value: &Json) -> Result<Config, String> {
        let schema = value.get("schema").and_then(Json::as_str).unwrap_or("");
        if schema != SCHEMA_CONFIG {
            return Err(format!("unsupported config schema {schema:?}"));
        }
        let mut obligations = Vec::new();
        if let Some(items) = value.get("obligations").and_then(Json::as_arr) {
            for item in items {
                let id = item
                    .get("id")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string();
                let program = item
                    .get("program")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string();
                if id.is_empty() || program.is_empty() {
                    return Err("obligation needs a non-empty id and program".into());
                }
                let args = item
                    .get("args")
                    .and_then(Json::as_arr)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                obligations.push(Obligation {
                    id,
                    program,
                    args,
                    required: item.get("required").and_then(Json::as_bool).unwrap_or(true),
                    timeout_s: item
                        .get("timeout_s")
                        .and_then(Json::as_num)
                        .unwrap_or(300.0) as u64,
                });
            }
        }
        Ok(Config {
            policy_version: value
                .get("policy_version")
                .and_then(Json::as_str)
                .unwrap_or(POLICY_VERSION)
                .to_string(),
            execution_boundary: value
                .get("execution_boundary")
                .and_then(Json::as_str)
                .unwrap_or("local-trusted")
                .to_string(),
            obligations,
            disabled: value
                .get("disabled")
                .and_then(Json::as_bool)
                .unwrap_or(false),
        })
    }
}

pub fn flow_dir(root: &Path) -> PathBuf {
    root.join(".flow")
}

pub fn config_path(root: &Path) -> PathBuf {
    flow_dir(root).join("config.json")
}

pub fn journal_path(root: &Path) -> PathBuf {
    flow_dir(root).join("journal.jsonl")
}

pub fn receipts_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("receipts")
}

pub fn logs_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("logs")
}

pub fn proposals_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("proposals")
}

pub fn candidates_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("candidates")
}

pub fn adoption_path(root: &Path) -> PathBuf {
    flow_dir(root).join("adopted.json")
}

/// The local approval of a repository policy.
///
/// `.flow/config.json` can arrive with a clone. Nothing in this file is executed
/// until this record names its policy digest: a committed config is a claim, not
/// a consent.
pub struct Adoption {
    pub policy_digest: String,
    pub policy_version: String,
    pub approved_at: String,
    pub source: String,
}

pub fn load_adoption(root: &Path) -> Result<Option<Adoption>, String> {
    let path = adoption_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let value = json::parse(&text)?;
    let schema = value.get("schema").and_then(Json::as_str).unwrap_or("");
    if schema != SCHEMA_ADOPTION {
        return Err(format!(
            "unsupported adoption schema {schema:?} in {}: refusing to guess what was approved",
            path.display()
        ));
    }
    let get = |key: &str| {
        value
            .get(key)
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string()
    };
    Ok(Some(Adoption {
        policy_digest: get("policy_digest"),
        policy_version: get("policy_version"),
        approved_at: get("approved_at"),
        source: get("source"),
    }))
}

pub fn save_adoption(root: &Path, adoption: &Adoption) -> Result<(), String> {
    let value = Json::obj(vec![
        ("schema", Json::str(SCHEMA_ADOPTION)),
        ("policy_digest", Json::str(adoption.policy_digest.clone())),
        ("policy_version", Json::str(adoption.policy_version.clone())),
        ("approved_at", Json::str(adoption.approved_at.clone())),
        ("source", Json::str(adoption.source.clone())),
    ]);
    write_atomic(&adoption_path(root), &(value.to_string_pretty() + "\n"))
}

pub fn receipt_path(root: &Path, tree_oid: &str) -> PathBuf {
    receipts_dir(root).join(format!("{tree_oid}.json"))
}

pub fn load_config(root: &Path) -> Result<Config, String> {
    let path = config_path(root);
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("not initialized: cannot read {}: {e}", path.display()))?;
    Config::from_json(&json::parse(&text)?)
}

pub fn save_config(root: &Path, config: &Config) -> Result<(), String> {
    let path = config_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create .flow: {e}"))?;
    }
    write_atomic(&path, &(config.to_json().to_string_pretty() + "\n"))
}

/// Refuse to write through a symlink or a Windows reparse point (junction).
///
/// A repository can ship `.flow/journal.jsonl` or `.flow/receipts` as a symlink:
/// cloning with `core.symlinks=true` (the default on Linux and macOS) then makes
/// Flow append outside the workspace. Every write below `.flow/` therefore checks
/// that each existing component is a real directory and that the target is not a
/// link. This is a check-then-write, so it is not a defence against a *concurrent*
/// local attacker — that boundary is documented, and the local user is trusted.
fn guard_below_flow(path: &Path) -> Result<(), String> {
    let mut anchor: Option<&Path> = None;
    for ancestor in path.ancestors() {
        if ancestor.file_name().map(|n| n == ".flow").unwrap_or(false) {
            anchor = ancestor.parent();
            break;
        }
    }
    let Some(root) = anchor else {
        return Ok(());
    };
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("{} is not below {}", path.display(), root.display()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                let kind = meta.file_type();
                let reparse = {
                    #[cfg(windows)]
                    {
                        use std::os::windows::fs::MetadataExt;
                        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
                        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
                    }
                    #[cfg(not(windows))]
                    {
                        false
                    }
                };
                if kind.is_symlink() || reparse {
                    return Err(format!(
                        "refusing to write {}: {} is a symbolic link or reparse point, so the write would escape the workspace",
                        path.display(),
                        current.display()
                    ));
                }
            }
            Err(_) => break,
        }
    }
    // A hard link is indistinguishable from a regular file on Windows without
    // Win32, so the check exists where the metadata exposes it. It closes the
    // "journal appended through a link to an outside file" shape on Unix.
    #[cfg(unix)]
    if let Ok(meta) = fs::symlink_metadata(path) {
        use std::os::unix::fs::MetadataExt;
        if meta.is_file() && meta.nlink() > 1 {
            return Err(format!(
                "refusing to write {}: it has {} hard links, so the write could reach a file outside the workspace",
                path.display(),
                meta.nlink()
            ));
        }
    }
    Ok(())
}

pub fn write_atomic(path: &Path, data: &str) -> Result<(), String> {
    write_atomic_bytes(path, data.as_bytes())
}

/// Atomic write of arbitrary bytes: a proposal patch is not guaranteed to be
/// text and must never be round-tripped through `String`.
pub fn write_atomic_bytes(path: &Path, data: &[u8]) -> Result<(), String> {
    guard_below_flow(path)?;
    // The temporary name carries the process id: two Flow processes writing
    // different files never fight over one shared scratch name.
    let tmp = path.with_extension(format!("tmp-flow-{}", std::process::id()));
    {
        let mut file =
            File::create(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
        file.write_all(data)
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        file.sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("cannot rename into {}: {e}", path.display()))?;
    // The rename is durable only once the containing directory is flushed. A
    // journal entry whose rename is lost after a power cut, while `update-ref`
    // already moved the ref, is precisely the state recovery must never have to
    // guess about. Windows exposes no directory fsync, so the claim there is
    // "file fsynced before the rename" and the README says so.
    #[cfg(not(windows))]
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let dir = File::open(parent)
            .map_err(|e| format!("cannot open {} for fsync: {e}", parent.display()))?;
        dir.sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", parent.display()))?;
    }
    Ok(())
}

pub fn append_journal(root: &Path, record: &Json) -> Result<(), String> {
    // Every record carries its schema so a future reader can refuse a mismatch.
    let mut tagged = record.clone();
    if let Json::Obj(pairs) = &mut tagged {
        if !pairs.iter().any(|(k, _)| k == "schema") {
            pairs.insert(0, ("schema".to_string(), Json::str(SCHEMA_JOURNAL)));
        }
    }
    let path = journal_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create .flow: {e}"))?;
    }
    guard_below_flow(&path)?;
    // The journal is rewritten and renamed rather than opened with `O_APPEND`.
    // A rename replaces the directory entry, so a *hard link* to an outside file
    // can no longer be written through — the case the `nlink > 1` check cannot see
    // on Windows. It also makes every append atomic: a torn line is impossible.
    // The cost is O(journal size) per append, which is bounded in practice and
    // documented; the alternative is a write primitive with worse failure modes.
    let mut content = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("cannot read journal {}: {e}", path.display())),
    };
    // `tagged`, not `record`: the schema stamp is what lets a future reader
    // refuse a record it does not understand.
    let mut line = tagged.to_string();
    line.push('\n');
    content.extend_from_slice(line.as_bytes());
    write_atomic_bytes(&path, &content).map_err(|e| format!("cannot append journal: {e}"))
}
/// Read every journal record.
///
/// A missing journal is an empty journal (a fresh repository). Anything else
/// that prevents a faithful read is an error — never silently an empty set.
pub fn read_journal(root: &Path) -> Result<Vec<Json>, JournalError> {
    let path = journal_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file =
        File::open(&path).map_err(|e| JournalError::Io(format!("cannot open journal: {e}")))?;
    let mut records = Vec::new();
    for (lineno, line) in BufReader::new(file).lines().enumerate() {
        let line =
            line.map_err(|e| JournalError::Io(format!("cannot read line {}: {e}", lineno + 1)))?;
        if line.trim().is_empty() {
            continue;
        }
        let record = json::parse(&line).map_err(|e| {
            JournalError::Corrupt(format!("torn or invalid line {}: {e}", lineno + 1))
        })?;
        records.push(record);
    }
    Ok(records)
}

/// Epoch seconds, used for lock ages.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Nanosecond-resolution suffix for journal intent ids.
///
/// The id used to be `second + commit prefix`. Two publications of the same
/// tree to two refs within one second produce the same commit (same tree, same
/// parent, same default message), hence the same id, and one of the two intents
/// could then be closed by the other's outcome.
pub fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

pub struct Receipt {
    pub schema: String,
    pub tree_oid: String,
    pub commit_oid: String,
    pub context_id: String,
    /// Digest of the *effective verification policy* this receipt proves.
    pub policy_digest: String,
    pub policy_version: String,
    pub execution_boundary: String,
    pub status: String,
    /// `complete` or `selected`: only a complete scope is publishable.
    pub scope_mode: String,
    pub missing_required: Vec<String>,
    pub started_at: String,
    pub finished_at: String,
    pub obligations: Vec<Json>,
    pub logs: Vec<String>,
    pub worktree_diff: Vec<String>,
    pub json: Json,
}

impl Receipt {
    /// Load the **complete** receipt for `tree_oid`.
    ///
    /// Selected (partial) executions are written to a different file, so a
    /// partial run can never be mistaken for — or overwrite — a complete proof.
    pub fn load(root: &Path, tree_oid: &str) -> Result<Option<Receipt>, String> {
        // The tree id becomes a file name. It must be an object id, never a path.
        if !crate::paths::is_object_id(tree_oid) {
            return Err(format!(
                "INVALID_TREE_ID: {tree_oid:?} is not a 40- or 64-character hex object id, so it cannot name a receipt"
            ));
        }
        let path = receipt_path(root, tree_oid);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|e| format!("cannot read receipt: {e}"))?;
        let value = json::parse(&text)?;
        let get = |key: &str| {
            value
                .get(key)
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string()
        };
        let scope = value.get("verification_scope");
        let scope_mode = scope
            .and_then(|s| s.get("mode"))
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        Ok(Some(Receipt {
            schema: get("schema"),
            tree_oid: get("tree_oid"),
            commit_oid: get("commit_oid"),
            context_id: get("context_id"),
            policy_digest: get("policy_digest"),
            policy_version: get("policy_version"),
            execution_boundary: get("execution_boundary"),
            status: get("status"),
            scope_mode,
            missing_required: scope
                .and_then(|s| s.get("missing_required"))
                .and_then(Json::as_arr)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            started_at: get("started_at"),
            finished_at: get("finished_at"),
            obligations: value
                .get("obligations")
                .and_then(Json::as_arr)
                .map(<[Json]>::to_vec)
                .unwrap_or_default(),
            logs: string_list(&value, "logs"),
            worktree_diff: string_list(&value, "worktree_diff"),
            json: value,
        }))
    }
}

/// Remove one top-level field, keeping every other pair in order.
fn without_field(value: &Json, key: &str) -> Json {
    let mut out = value.clone();
    if let Json::Obj(pairs) = &mut out {
        pairs.retain(|(k, _)| k != key);
    }
    out
}

/// Bind a receipt to its own content.
///
/// This detects accidental corruption and casual editing of a receipt on disk.
/// It is **not** a signature and **not** protection against an attacker who can
/// rewrite both the config and the receipt: that would require a key the Flow
/// process does not have. The honest boundary is documented in the report.
pub fn seal_receipt(value: &Json) -> Json {
    let body = without_field(value, "integrity");
    let digest = crate::sha256::sha256_hex(body.to_string().as_bytes());
    let mut sealed = body;
    if let Json::Obj(pairs) = &mut sealed {
        pairs.push(("integrity".to_string(), Json::str(digest)));
    }
    sealed
}

pub fn verify_receipt_integrity(value: &Json) -> Result<(), String> {
    let recorded = value
        .get("integrity")
        .and_then(Json::as_str)
        .ok_or("receipt carries no integrity digest")?;
    let body = without_field(value, "integrity");
    let computed = crate::sha256::sha256_hex(body.to_string().as_bytes());
    if computed != recorded {
        return Err(format!(
            "receipt content does not match its integrity digest (recorded {recorded}, computed {computed})"
        ));
    }
    Ok(())
}

fn string_list(value: &Json, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Where a *partial* (selected) execution is recorded. Deliberately a different
/// file from the complete slot so the two can never shadow one another.
pub fn partial_receipt_path(root: &Path, tree_oid: &str, context_id: &str) -> PathBuf {
    let prefix: String = context_id.chars().take(12).collect();
    receipts_dir(root).join(format!("{tree_oid}.partial.{prefix}.json"))
}

pub fn save_receipt(root: &Path, receipt: &Json) -> Result<PathBuf, String> {
    let tree = receipt
        .get("tree_oid")
        .and_then(Json::as_str)
        .ok_or("receipt without tree_oid")?;
    // The tree id becomes a file name here too: a receipt is loaded from disk as
    // well as written, and a hand-edited or foreign one must never choose where
    // the next write lands.
    if !crate::paths::is_object_id(tree) {
        return Err(format!(
            "INVALID_TREE_ID: receipt tree_oid {tree:?} is not a 40- or 64-character hex object id"
        ));
    }
    let mode = receipt
        .get("verification_scope")
        .and_then(|s| s.get("mode"))
        .and_then(Json::as_str)
        .unwrap_or("");
    let path = match mode {
        // The complete slot is keyed by tree: a re-run on the same tree replaces
        // the previous complete proof, which is the intended behaviour.
        "complete" => receipt_path(root, tree),
        "selected" => {
            let context = receipt
                .get("context_id")
                .and_then(Json::as_str)
                .ok_or("selected receipt without context_id")?;
            partial_receipt_path(root, tree, context)
        }
        other => {
            return Err(format!(
                "receipt has unknown verification_scope.mode {other:?}: refusing to pick a slot"
            ))
        }
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create receipts dir: {e}"))?;
    }
    write_atomic(&path, &(receipt.to_string_pretty() + "\n"))?;
    Ok(path)
}

// The publication lock moved to `crate::lock` (contract `flow-lock/2`): its
// attribution rules are the whole point, and they live next to their tests.

/// Civil date/time from the Unix epoch, UTC, without a dependency.
pub fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Howard Hinnant's algorithm, adapted from days since 1970-01-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
    }

    #[test]
    fn config_round_trip() {
        let config = Config {
            obligations: vec![Obligation {
                id: "t".into(),
                program: "cargo".into(),
                args: vec!["test".into()],
                required: true,
                timeout_s: 30,
            }],
            ..Config::default()
        };
        let parsed = Config::from_json(&config.to_json()).unwrap();
        assert_eq!(parsed.obligations.len(), 1);
        assert_eq!(parsed.obligations[0].program, "cargo");
        assert_eq!(parsed.policy_version, POLICY_VERSION);
    }

    #[test]
    fn rejects_unknown_schema() {
        let mut value = Config::default().to_json();
        if let Json::Obj(pairs) = &mut value {
            pairs.retain(|(k, _)| k != "schema");
        }
        assert!(Config::from_json(&value).is_err());
    }

    // The tests below pin the local-state contracts the CLI relies on: a record
    // is never reinterpreted, a file name is never taken from a record without a
    // check, and a failure is reported rather than swallowed. They live here
    // because these are private helpers of the durable-state module.

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("flow-state-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn a_journal_error_names_its_kind_and_never_looks_like_empty_state() {
        assert_eq!(
            JournalError::Io("cannot open".into()).to_string(),
            "journal unreadable: cannot open"
        );
        assert_eq!(
            JournalError::Corrupt("torn line 2".into()).to_string(),
            "journal corrupt: torn line 2"
        );
        assert_eq!(
            String::from(JournalError::Corrupt("x".into())),
            "journal corrupt: x"
        );
    }

    #[test]
    fn an_obligation_missing_its_program_is_refused_not_dropped() {
        let torn =
            json::parse(r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":""}]}"#)
                .unwrap();
        let reason = Config::from_json(&torn)
            .err()
            .expect("a policy with an unusable obligation must be refused");
        assert!(reason.contains("non-empty id and program"), "{reason}");

        let no_id =
            json::parse(r#"{"schema":"flow-config/1","obligations":[{"program":"git"}]}"#).unwrap();
        assert!(Config::from_json(&no_id).is_err());
    }

    #[test]
    fn a_config_whose_obligations_are_not_a_list_keeps_the_declared_defaults() {
        // A shape Flow does not understand is not an obligation set: the record
        // still names its policy version and boundary, and declares nothing.
        let value = json::parse(
            r#"{"schema":"flow-config/1","obligations":"none","policy_version":"FLOW-POLICY-1"}"#,
        )
        .unwrap();
        let config = Config::from_json(&value).unwrap();
        assert_eq!(config.obligations.len(), 0);
        assert_eq!(config.policy_version, POLICY_VERSION);
        assert_eq!(config.execution_boundary, "local-trusted");
        assert!(!config.disabled);
    }

    #[test]
    fn an_adoption_record_from_another_schema_is_not_reinterpreted() {
        let root = temp_root("adoption");
        assert!(load_adoption(&root).unwrap().is_none());

        let path = adoption_path(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"schema":"flow-adoption/9","policy_digest":"x"}"#).unwrap();
        let reason = load_adoption(&root)
            .err()
            .expect("a foreign adoption schema must be refused");
        assert!(reason.contains("refusing to guess"), "{reason}");

        let adoption = Adoption {
            policy_digest: "digest".into(),
            policy_version: POLICY_VERSION.into(),
            approved_at: "2026-01-01T00:00:00Z".into(),
            source: "test".into(),
        };
        save_adoption(&root, &adoption).unwrap();
        let loaded = load_adoption(&root).unwrap().expect("a saved adoption");
        assert_eq!(loaded.policy_digest, "digest");
        assert_eq!(loaded.source, "test");
        assert_eq!(loaded.approved_at, "2026-01-01T00:00:00Z");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_journal_append_keeps_a_record_schema_and_a_missing_journal_is_empty() {
        let root = temp_root("journal");
        // A fresh repository has no journal: that is an empty journal, not an
        // unreadable one.
        assert!(read_journal(&root).unwrap().is_empty());

        // A record that already carries a schema keeps it (a future reader must
        // be able to see the version the record was written under).
        let tagged = Json::obj(vec![
            ("schema", Json::str("flow-journal/0")),
            ("event", Json::str("note")),
        ]);
        append_journal(&root, &tagged).unwrap();
        // A record that is not an object cannot be tagged; it is still appended
        // as the caller wrote it rather than being silently rewritten.
        append_journal(&root, &Json::Num(1.0)).unwrap();
        let records = read_journal(&root).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].get("schema").and_then(Json::as_str),
            Some("flow-journal/0")
        );
        assert_eq!(records[1], Json::Num(1.0));

        // Blank lines are layout, never records.
        let mut text = fs::read_to_string(journal_path(&root)).unwrap();
        text.push_str("\n \n");
        fs::write(journal_path(&root), &text).unwrap();
        assert_eq!(read_journal(&root).unwrap().len(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_receipt_can_only_be_stored_under_an_object_id_and_a_known_slot() {
        let root = temp_root("receipts");
        let tree = "a".repeat(40);
        let receipt = |mode: &str, context: Option<&str>, tree_oid: &str| {
            let mut pairs = vec![
                ("schema".to_string(), Json::str(SCHEMA_RECEIPT)),
                ("tree_oid".to_string(), Json::str(tree_oid)),
                (
                    "verification_scope".to_string(),
                    Json::obj(vec![("mode", Json::str(mode))]),
                ),
            ];
            if let Some(context) = context {
                pairs.push(("context_id".to_string(), Json::str(context)));
            }
            Json::Obj(pairs)
        };

        // The tree id becomes a file name: a path must never reach it.
        let reason = save_receipt(&root, &receipt("complete", None, "../../escape")).unwrap_err();
        assert!(reason.contains("INVALID_TREE_ID"), "{reason}");
        assert!(
            !receipts_dir(&root).exists(),
            "nothing may be written first"
        );

        // An unknown slot is refused instead of being coerced to "complete".
        let reason = save_receipt(&root, &receipt("everything", None, &tree)).unwrap_err();
        assert!(
            reason.contains("unknown verification_scope.mode"),
            "{reason}"
        );

        // A partial record needs the context that names its own file.
        let reason = save_receipt(&root, &receipt("selected", None, &tree)).unwrap_err();
        assert!(
            reason.contains("selected receipt without context_id"),
            "{reason}"
        );

        let written = save_receipt(&root, &receipt("selected", Some("ctx-123"), &tree)).unwrap();
        assert!(written.is_file(), "{}", written.display());
        assert!(
            written.to_string_lossy().contains(".partial.ctx-123"),
            "a selected receipt never shares the complete slot: {}",
            written.display()
        );
        // Loading a receipt that is not there is `None`, not an error.
        assert!(Receipt::load(&root, &tree).unwrap().is_none());
        // An id that is not an object id cannot name a receipt at all.
        assert!(Receipt::load(&root, "../config.json").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_write_outside_the_flow_directory_is_not_guarded_and_still_atomic() {
        let root = temp_root("guard");
        // No `.flow` component: the guard has no anchor and the write proceeds.
        let plain = root.join("plain.json");
        write_atomic(&plain, "content\n").unwrap();
        assert_eq!(fs::read_to_string(&plain).unwrap(), "content\n");
        // The guarded shape creates `.flow` and lands in it.
        save_config(&root, &Config::default()).unwrap();
        assert!(config_path(&root).is_file());
        assert!(load_config(&root).is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_civil_calendar_handles_days_before_the_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(-365), (1969, 1, 1));
        // Far enough back that the era itself is negative (`z < 0`): year 0,
        // 29 February, which is a valid proleptic-Gregorian leap day. The
        // alternative branch must not be dead code assumed away.
        assert_eq!(civil_from_days(-719_469), (0, 2, 29));
        // A timestamp is always formatted, never "negative days" garbage.
        let stamp = iso8601_now();
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert!(stamp.ends_with('Z'), "{stamp}");
    }

    #[test]
    fn a_receipt_helper_tolerates_a_non_object_without_panicking() {
        // The helpers are total: a caller that hands them a non-object gets the
        // value back unchanged instead of a panic inside a refusal path.
        assert_eq!(
            without_field(&Json::Bool(true), "integrity"),
            Json::Bool(true)
        );
        assert_eq!(seal_receipt(&Json::Num(1.0)), Json::Num(1.0));
        let sealed = seal_receipt(&Json::obj(vec![("status", Json::str("PASSED"))]));
        assert!(verify_receipt_integrity(&sealed).is_ok());
    }

    #[cfg(windows)]
    fn make_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        let out = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other("mklink /J failed"))
        }
    }

    #[cfg(unix)]
    fn make_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[test]
    fn a_reparse_point_below_flow_is_never_written_through() {
        let root = temp_root("guard-reparse");
        let outside = temp_root("guard-outside");
        let link = root.join(".flow");
        if let Err(e) = make_dir_link(&outside, &link) {
            eprintln!("SKIP a reparse point below .flow: cannot create a link ({e})");
            let _ = fs::remove_dir_all(&root);
            let _ = fs::remove_dir_all(&outside);
            return;
        }
        // A real file beneath the redirect: the guard walks the components and
        // must refuse at `.flow` before any write reaches the outside directory.
        fs::write(outside.join("journal.jsonl"), "").unwrap();
        let reason = guard_below_flow(&link.join("journal.jsonl"))
            .expect_err("a redirect below .flow is refused");
        assert!(
            reason.contains("symbolic link or reparse point"),
            "{reason}"
        );
        let _ = fs::remove_dir(&link);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn the_clock_helpers_return_epoch_values_not_constants() {
        // 2023-11-14 in seconds: any later build passes, a constant 0/1 does not.
        assert!(
            unix_now() > 1_700_000_000,
            "unix_now must be epoch seconds, got {}",
            unix_now()
        );
        assert!(
            unique_suffix() > 1_700_000_000_000_000_000,
            "unique_suffix must be epoch nanoseconds"
        );
    }

    #[test]
    fn iso8601_now_has_the_shape_and_the_current_year() {
        let now = iso8601_now();
        assert_eq!(now.len(), 20, "a UTC timestamp is 20 characters: {now}");
        assert!(now.ends_with('Z'), "{now}");
        assert_eq!(&now[4..5], "-", "{now}");
        assert_eq!(&now[7..8], "-", "{now}");
        assert_eq!(&now[10..11], "T", "{now}");
        assert_eq!(&now[13..14], ":", "{now}");
        assert_eq!(&now[16..17], ":", "{now}");
        let year: i64 = now[..4].parse().expect("four-digit year");
        assert!(year >= 2024, "the year must be current: {now}");
        let hour: u32 = now[11..13].parse().expect("hour");
        assert!(hour < 24, "{now}");
        let minute: u32 = now[14..16].parse().expect("minute");
        assert!(minute < 60, "{now}");
        let second: u32 = now[17..19].parse().expect("second");
        assert!(second < 60, "{now}");

        // The date and the time of day must both come from the same epoch second:
        // a wrong remainder (`%` replaced by `/`) keeps the shape and the year and
        // only moves the clock, so the shape alone is not a proof.
        let secs = unix_now();
        let got = u64::from(hour) * 3600 + u64::from(minute) * 60 + u64::from(second);
        let want = secs % 86_400;
        let delta = got.abs_diff(want);
        assert!(
            delta <= 2 || delta >= 86_398,
            "the clock time must match the epoch second ({got} vs {want}): {now}"
        );
    }

    #[test]
    fn civil_dates_exercise_the_era_and_century_corrections() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        // Day 59 is 1970-03-01; the century correction moves it to an impossible
        // 1970-02-29 when either operator is mutated, which a single date cannot see.
        assert_eq!(civil_from_days(59), (1970, 3, 1));
        // Beyond one century the `doe / 36_524` correction is non-zero, which is
        // where the mutated arithmetic differs.
        assert_eq!(civil_from_days(100_000), (2243, 10, 17));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(-719_469), (0, 2, 29));
    }

    #[test]
    fn string_list_reads_only_the_string_entries_of_an_array() {
        let value =
            json::parse(r#"{"logs":["a","b"],"mixed":["a",1],"scalar":"x"}"#).expect("json");
        assert_eq!(
            string_list(&value, "logs"),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(string_list(&value, "mixed"), vec!["a".to_string()]);
        assert!(string_list(&value, "scalar").is_empty());
        assert!(string_list(&value, "absent").is_empty());
    }
}
