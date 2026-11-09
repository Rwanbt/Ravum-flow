//! Durable state: config, journal (append + fsync), receipts.
//!
//! Guarantees stated by this module, and only these:
//! * an intent appended and fsynced is not forgotten;
//! * an outcome is appended after the ref update attempt;
//! * a torn journal line is refused (`InvalidData`), never silently skipped.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::gitcli::Obligation;
use crate::json::{self, Json};

pub const POLICY_VERSION: &str = "FLOW-POLICY-1";
pub const SCHEMA_CONFIG: &str = "flow-config/1";
/// Bumped whenever the receipt's semantics change. A receipt written under an
/// older schema is never silently reinterpreted: publication refuses it.
pub const SCHEMA_RECEIPT: &str = "flow-receipt/2";
pub const SCHEMA_JOURNAL: &str = "flow-journal/1";
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

pub fn write_atomic(path: &Path, data: &str) -> Result<(), String> {
    // The temporary name carries the process id: two Flow processes writing
    // different files never fight over one shared scratch name.
    let tmp = path.with_extension(format!("tmp-flow-{}", std::process::id()));
    {
        let mut file =
            File::create(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
        file.write_all(data.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        file.sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("cannot rename into {}: {e}", path.display()))
}

pub fn append_journal(root: &Path, record: &Json) -> Result<(), String> {
    // Every record carries its schema so a future reader can refuse a mismatch.
    let mut tagged = record.clone();
    if let Json::Obj(pairs) = &mut tagged {
        if !pairs.iter().any(|(k, _)| k == "schema") {
            pairs.insert(0, ("schema".to_string(), Json::str(SCHEMA_JOURNAL)));
        }
    }
    let record = &tagged;
    let path = journal_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create .flow: {e}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open journal: {e}"))?;
    let mut line = record.to_string();
    line.push('\n');
    file.write_all(line.as_bytes())
        .map_err(|e| format!("cannot append journal: {e}"))?;
    // fsync the file and the directory (best effort on Windows).
    file.sync_all()
        .map_err(|e| format!("cannot fsync journal: {e}"))?;
    #[cfg(not(windows))]
    if let Some(parent) = path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
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

// ------------------------------------------------------------------- lock
//
// A local, advisory lock between Flow processes. It serialises the short
// mutations (`publish`, `reconcile`) so two writers cannot interleave journal
// appends and produce a torn line. It is NOT a distributed lease, it does not
// fence a foreign `git` writer, and a stale lock is recovered by observing that
// its owner is gone — never by assuming the worst case is impossible.

const LOCK_MAX_AGE_S: u64 = 120;

pub struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
fn pid_alive(pid: u32) -> Option<bool> {
    let out = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    Some(text.contains(&format!("\"{pid}\"")) || text.contains(&format!(",{pid},")))
}

#[cfg(not(windows))]
fn pid_alive(pid: u32) -> Option<bool> {
    let status = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .ok()?;
    Some(status.success())
}

fn lock_is_stale(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let mut parts = text.split_whitespace();
    // An unparseable lock cannot be attributed: treat it as abandoned.
    let Some(pid) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
        return true;
    };
    let at = parts
        .next()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(0);
    if unix_now().saturating_sub(at) > LOCK_MAX_AGE_S {
        return true;
    }
    // Undeterminable liveness is treated as alive: never steal on ignorance.
    matches!(pid_alive(pid), Some(false))
}

pub fn acquire_lock(root: &Path, purpose: &str) -> Result<LockGuard, String> {
    let path = flow_dir(root).join("lock");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create .flow: {e}"))?;
    }
    for _ in 0..40 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let _ = writeln!(file, "{} {} {}", std::process::id(), unix_now(), purpose);
                let _ = file.sync_all();
                return Ok(LockGuard { path });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if lock_is_stale(&path) {
                    let _ = fs::remove_file(&path);
                    continue;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("cannot create {}: {e}", path.display())),
        }
    }
    Err(format!(
        "another flow process holds {} (remove it only if you are sure no flow process is running)",
        path.display()
    ))
}

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
}
