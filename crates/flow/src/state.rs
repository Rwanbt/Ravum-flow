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
pub const CONTEXT_CODEC: &str = "flow-context/3";

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
    /// Declared toolchain probes: `[["rustc","-Vv"],["cargo","-V"]]`. Their
    /// output is hashed into the build context, so a toolchain switch between
    /// `run` and `publish` invalidates the receipt.
    pub context_probes: Vec<Vec<String>>,
    pub disabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            policy_version: POLICY_VERSION.to_string(),
            execution_boundary: "local-trusted".to_string(),
            obligations: Vec::new(),
            context_probes: Vec::new(),
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
            (
                "context_probes",
                Json::Arr(
                    self.context_probes
                        .iter()
                        .map(|probe| {
                            Json::Arr(probe.iter().map(|a| Json::str(a.clone())).collect())
                        })
                        .collect(),
                ),
            ),
            ("disabled", Json::Bool(self.disabled)),
        ])
    }

    /// Decode a policy strictly.
    ///
    /// A policy is what a `PASSED` *means*: an unknown key, a mistyped value, a
    /// timeout outside `[1, 86400]` or a non-string argument is refused instead
    /// of being silently dropped or clamped into a different command. Every
    /// refusal names the field, so an operator can fix the file.
    pub fn from_json(value: &Json) -> Result<Config, String> {
        let Json::Obj(pairs) = value else {
            return Err("INVALID_POLICY: the policy must be a JSON object".into());
        };
        const POLICY_KEYS: [&str; 6] = [
            "schema",
            "policy_version",
            "execution_boundary",
            "obligations",
            "context_probes",
            "disabled",
        ];
        for (key, _) in pairs {
            if !POLICY_KEYS.contains(&key.as_str()) {
                return Err(format!(
                    "INVALID_POLICY: unknown policy key {key:?}: refusing to guess what it means"
                ));
            }
        }
        let schema = value.get("schema").and_then(Json::as_str).unwrap_or("");
        if schema != SCHEMA_CONFIG {
            return Err(format!("unsupported config schema {schema:?}"));
        }
        let policy_version = match value.get("policy_version") {
            None => POLICY_VERSION.to_string(),
            Some(v) => v
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "INVALID_POLICY: policy_version must be a string".to_string())?,
        };
        if policy_version.trim().is_empty() {
            return Err("INVALID_POLICY: policy_version must not be empty".into());
        }
        let execution_boundary = match value.get("execution_boundary") {
            None => "local-trusted".to_string(),
            Some(v) => {
                let boundary = v.as_str().ok_or_else(|| {
                    "INVALID_POLICY: execution_boundary must be a string".to_string()
                })?;
                if boundary != "local-trusted" {
                    return Err(format!(
                        "INVALID_POLICY: execution_boundary {boundary:?} is not implemented; only \"local-trusted\" is supported"
                    ));
                }
                boundary.to_string()
            }
        };
        let mut obligations = Vec::new();
        match value.get("obligations") {
            None => {}
            Some(Json::Arr(items)) => {
                const OBLIGATION_KEYS: [&str; 5] =
                    ["id", "program", "args", "required", "timeout_s"];
                for item in items {
                    let Json::Obj(fields) = item else {
                        return Err("INVALID_POLICY: every obligation must be an object".into());
                    };
                    for (key, _) in fields {
                        if !OBLIGATION_KEYS.contains(&key.as_str()) {
                            return Err(format!("INVALID_POLICY: unknown obligation key {key:?}"));
                        }
                    }
                    let id = item
                        .get("id")
                        .and_then(Json::as_str)
                        .ok_or_else(|| {
                            "INVALID_POLICY: obligation id must be a string".to_string()
                        })?
                        .to_string();
                    let program = item
                        .get("program")
                        .and_then(Json::as_str)
                        .ok_or_else(|| {
                            "INVALID_POLICY: obligation program must be a string".to_string()
                        })?
                        .to_string();
                    if id.is_empty() || program.is_empty() {
                        return Err("obligation needs a non-empty id and program".into());
                    }
                    let args = match item.get("args") {
                        None => Vec::new(),
                        Some(Json::Arr(values)) => values
                            .iter()
                            .map(|v| {
                                v.as_str().map(str::to_string).ok_or_else(|| {
                                    "INVALID_POLICY: every obligation argument must be a string"
                                        .to_string()
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        Some(_) => {
                            return Err(
                                "INVALID_POLICY: obligation args must be an array of strings"
                                    .into(),
                            )
                        }
                    };
                    let required = match item.get("required") {
                        None => true,
                        Some(v) => v.as_bool().ok_or_else(|| {
                            "INVALID_POLICY: obligation required must be a boolean".to_string()
                        })?,
                    };
                    let timeout_s = match item.get("timeout_s") {
                        None => 300u64,
                        Some(v) => {
                            let number = v.as_num().ok_or_else(|| {
                                "INVALID_POLICY: obligation timeout_s must be a number".to_string()
                            })?;
                            if !number.is_finite()
                                || number.fract() != 0.0
                                || !(1.0..=86_400.0).contains(&number)
                            {
                                return Err(format!(
                                    "INVALID_POLICY: obligation timeout_s {number} is outside the supported integer range [1, 86400]"
                                ));
                            }
                            number as u64
                        }
                    };
                    obligations.push(Obligation {
                        id,
                        program,
                        args,
                        required,
                        timeout_s,
                    });
                }
            }
            Some(_) => return Err("INVALID_POLICY: obligations must be an array".into()),
        }
        let context_probes = match value.get("context_probes") {
            None => Vec::new(),
            Some(Json::Arr(probes)) => {
                let mut out = Vec::new();
                for probe in probes {
                    let Json::Arr(args) = probe else {
                        return Err("INVALID_POLICY: every context probe must be an array".into());
                    };
                    if args.is_empty() {
                        return Err("INVALID_POLICY: a context probe must name a program".into());
                    }
                    out.push(
                        args.iter()
                            .map(|v| {
                                v.as_str().map(str::to_string).ok_or_else(|| {
                                    "INVALID_POLICY: every context probe argument must be a string"
                                        .to_string()
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    );
                }
                out
            }
            Some(_) => {
                return Err("INVALID_POLICY: context_probes must be an array of arrays".into())
            }
        };
        let disabled = match value.get("disabled") {
            None => false,
            Some(v) => v
                .as_bool()
                .ok_or_else(|| "INVALID_POLICY: disabled must be a boolean".to_string())?,
        };
        Ok(Config {
            policy_version,
            execution_boundary,
            obligations,
            context_probes,
            disabled,
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

/// Maximum size of the **active** journal segment before it is archived.
///
/// Every append rewrites the active file (rename-based atomicity, no `O_APPEND`
/// and no writable hard link). Without a bound, the cost of one publication grew
/// with the whole history (FBL-012). 256 KiB keeps an append in the measured
/// single-digit-millisecond range while making it independent of the journal's
/// age: the active segment is rotated to `journal.<20 digits>.jsonl` and a fresh
/// one starts. Archived segments are evidence and are never deleted.
pub const JOURNAL_SEGMENT_BYTES: u64 = 256 * 1024;

/// Archived journal segments, oldest first.
///
/// The active file is `journal.jsonl`; an archived one is
/// `journal.<20-digit sequence>.jsonl`. Anything else in `.flow/` is ignored, so
/// a foreign file can never be read as history.
fn journal_segments(root: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = flow_dir(root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        // No `.flow/` yet is an empty history, not an error.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("cannot list {}: {e}", dir.display())),
    };
    let mut segments: Vec<(u64, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(rest) = name.strip_prefix("journal.") else {
            continue;
        };
        let Some(sequence) = rest.strip_suffix(".jsonl") else {
            continue;
        };
        if sequence.len() != 20 || !sequence.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(number) = sequence.parse::<u64>() {
            segments.push((number, entry.path()));
        }
    }
    segments.sort_by_key(|(number, _)| *number);
    Ok(segments.into_iter().map(|(_, path)| path).collect())
}

/// Next free sequence number for an archived segment.
fn next_journal_sequence(root: &Path) -> u64 {
    let Ok(segments) = journal_segments(root) else {
        return 1;
    };
    let last = segments.last().and_then(|path| {
        let name = path.file_name()?.to_string_lossy().to_string();
        let digits = name.strip_prefix("journal.")?.strip_suffix(".jsonl")?;
        digits.parse::<u64>().ok()
    });
    last.map(|number| number.saturating_add(1)).unwrap_or(1)
}

pub fn receipts_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("receipts")
}

pub fn logs_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("logs")
}

/// One marker per in-flight verification, used by `gc` to refuse while a run is
/// writing evidence. A marker is removed by the run's guard, including on error.
pub fn runs_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("runs")
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
    if let Some(parent) = parent_to_flush(path) {
        let dir = File::open(parent)
            .map_err(|e| format!("cannot open {} for fsync: {e}", parent.display()))?;
        dir.sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", parent.display()))?;
    }
    Ok(())
}

// ------------------------------------------------------- content-addressed store

/// Directory of content-addressed blobs below `.flow/`.
pub fn blobs_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("blobs")
}

/// A content digest is exactly 64 lowercase hexadecimal characters.
pub fn is_content_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The on-disk path of a content-addressed blob, or `None` for a non-digest.
pub fn blob_path(root: &Path, digest: &str) -> Option<PathBuf> {
    if !is_content_digest(digest) {
        return None;
    }
    Some(blobs_dir(root).join(&digest[..2]).join(digest))
}

/// The root-relative path recorded in a record: never an absolute host path.
pub fn blob_relative_path(digest: &str) -> String {
    format!(".flow/blobs/{}/{}", &digest[..2], digest)
}

/// Publish bytes at `path` only if the name does not exist yet.
///
/// The content is written to a uniquely named staging file in the same
/// directory, fsynced, then hard-linked onto the final name. The hard link is
/// the atomic create-or-fail primitive: two writers racing on one name produce
/// exactly one winner and one `AlreadyExists`, never two visible contents.
/// Returns `Ok(true)` when this call created the name, `Ok(false)` when it
/// already existed.
pub fn create_once(path: &Path, data: &[u8]) -> Result<bool, String> {
    // An existing name is never rewritten, so it is answered *before* the
    // hard-link guard. That guard refuses a target with `nlink > 1`, and for a few
    // microseconds a concurrent creator's own staging link makes the final name
    // legitimately have two links (`name` + `name.tmp-flow-*`). Treating that as
    // an error failed a valid concurrent `propose` on macOS (CI run 43711951264):
    // "it has 2 hard links". The name exists, this call did not create it, and the
    // caller re-reads and validates the winner's bytes.
    if fs::symlink_metadata(path).is_ok() {
        return Ok(false);
    }
    if let Err(reason) = guard_below_flow(path) {
        // Same window, but the guard ran first: if the name exists now, another
        // writer published it (a symlink included) and nothing is written here.
        if fs::symlink_metadata(path).is_ok() {
            return Ok(false);
        }
        return Err(reason);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-flow-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    {
        let mut file =
            File::create(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
        file.write_all(data)
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        file.sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", tmp.display()))?;
    }
    let linked = fs::hard_link(&tmp, path);
    let _ = fs::remove_file(&tmp);
    match linked {
        Ok(()) => {
            #[cfg(not(windows))]
            if let Some(parent) = parent_to_flush(path) {
                if let Ok(dir) = File::open(parent) {
                    let _ = dir.sync_all();
                }
            }
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(format!("cannot publish {}: {e}", path.display())),
    }
}

/// Store bytes under their own digest and return that digest.
///
/// The name is the digest, so two writers of the same content converge on one
/// file and two writers of different content never share a name. An existing
/// file is re-read and compared: a digest collision is refused, never assumed.
pub fn put_blob(root: &Path, bytes: &[u8]) -> Result<String, String> {
    let digest = crate::sha256::sha256_hex(bytes);
    let path = blob_path(root, &digest)
        .ok_or_else(|| "internal: put_blob produced an invalid digest".to_string())?;
    if path.exists() {
        let existing =
            fs::read(&path).map_err(|e| format!("cannot re-read blob {}: {e}", path.display()))?;
        if existing != bytes {
            return Err(format!(
                "BLOB_COLLISION: blob {digest} already exists with different bytes: refusing to alias two contents"
            ));
        }
        return Ok(digest);
    }
    match create_once(&path, bytes)? {
        true => Ok(digest),
        false => {
            let existing = fs::read(&path)
                .map_err(|e| format!("cannot re-read blob {}: {e}", path.display()))?;
            if existing != bytes {
                Err(format!(
                    "BLOB_COLLISION: blob {digest} appeared concurrently with different bytes"
                ))
            } else {
                Ok(digest)
            }
        }
    }
}

/// Read a blob and prove it still hashes to the name it was stored under.
pub fn verified_blob(root: &Path, digest: &str) -> Result<Vec<u8>, String> {
    let path = blob_path(root, digest)
        .ok_or_else(|| format!("PROPOSAL_INTEGRITY: {digest:?} is not a sha256 content digest"))?;
    let bytes = fs::read(&path).map_err(|e| {
        format!(
            "PROPOSAL_INTEGRITY: blob {digest} is unreadable at {}: {e}",
            path.display()
        )
    })?;
    let actual = crate::sha256::sha256_hex(&bytes);
    if actual != digest {
        return Err(format!(
            "PROPOSAL_INTEGRITY: blob {digest} hashes to {actual}: the bytes do not match the recorded content"
        ));
    }
    Ok(bytes)
}

/// The directory entry a rename must flush, or `None` when there is none to flush.
///
/// `Path::parent` returns `Some("")` for a bare relative name (`"journal.jsonl"`):
/// that is the process working directory, which cannot be opened as `""`. Every
/// target the product writes is built from the workspace root and carries a real
/// parent, so a bare name is skipped instead of turning the write into a failed
/// `File::open("")`. Pinning the rule in one named place keeps the skip a decision
/// rather than an accident of string emptiness.
#[cfg(not(windows))]
fn parent_to_flush(path: &Path) -> Option<&Path> {
    path.parent().filter(|p| !p.as_os_str().is_empty())
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
    // `tagged`, not `record`: the schema stamp is what lets a future reader
    // refuse a record it does not understand.
    let mut line = tagged.to_string();
    line.push('\n');
    // The active segment is bounded: past `JOURNAL_SEGMENT_BYTES` it is archived
    // and a fresh one starts, so an append never rewrites the whole history
    // (FBL-012). The rotation is a rename, which is atomic and keeps the order:
    // archived segments are read first, oldest sequence first.
    let current_len = match fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => return Err(format!("cannot inspect journal {}: {e}", path.display())),
    };
    if current_len > 0 && current_len.saturating_add(line.len() as u64) > JOURNAL_SEGMENT_BYTES {
        rotate_journal(root, &path)?;
    }
    // The journal is rewritten and renamed rather than opened with `O_APPEND`.
    // A rename replaces the directory entry, so a *hard link* to an outside file
    // can no longer be written through — the case the `nlink > 1` check cannot see
    // on Windows. It also makes every append atomic: a torn line is impossible.
    // The cost is O(active segment) per append, now bounded by the rotation above.
    let mut content = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("cannot read journal {}: {e}", path.display())),
    };
    content.extend_from_slice(line.as_bytes());
    write_atomic_bytes(&path, &content).map_err(|e| format!("cannot append journal: {e}"))
}

/// Archive the active segment under the next sequence number.
///
/// A failure here is reported, never swallowed: the caller is about to append to
/// a file that would otherwise grow without bound.
fn rotate_journal(root: &Path, path: &Path) -> Result<(), String> {
    let sequence = next_journal_sequence(root);
    let archived = flow_dir(root).join(format!("journal.{sequence:020}.jsonl"));
    fs::rename(path, &archived)
        .map_err(|e| format!("cannot rotate journal into {}: {e}", archived.display()))?;
    // Make the new directory entry durable before the next append creates the
    // active file again; Windows has no directory fsync and says so in the README.
    #[cfg(not(windows))]
    if let Some(parent) = parent_to_flush(&archived) {
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
    let mut paths = journal_segments(root).map_err(JournalError::Io)?;
    // The active segment is read last: the history is archived segments in
    // sequence order, then whatever has been appended since the last rotation. A
    // crash between the rotation's rename and the creation of the new active file
    // leaves no `journal.jsonl`; that is an intact history, not an empty one.
    paths.push(journal_path(root));
    let mut records = Vec::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        let file = File::open(&path).map_err(|e| {
            JournalError::Io(format!("cannot open journal {}: {e}", path.display()))
        })?;
        for (lineno, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|e| {
                JournalError::Io(format!(
                    "cannot read line {} of {}: {e}",
                    lineno + 1,
                    path.display()
                ))
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let record = json::parse(&line).map_err(|e| {
                JournalError::Corrupt(format!(
                    "torn or invalid line {} of {}: {e}",
                    lineno + 1,
                    path.display()
                ))
            })?;
            records.push(record);
        }
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
    /// Identity of the execution that wrote this receipt, when the writer
    /// recorded one. Older receipts have no run id and are still readable.
    pub run_id: String,
    pub obligations: Vec<Json>,
    pub logs: Vec<String>,
    /// `[{id, stdout_sha256, stderr_sha256}]`: the digest of every log the
    /// receipt names, so a later reader can prove the evidence was not swapped.
    pub log_digests: Vec<Json>,
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
            run_id: get("run_id"),
            obligations: value
                .get("obligations")
                .and_then(Json::as_arr)
                .map(<[Json]>::to_vec)
                .unwrap_or_default(),
            logs: string_list(&value, "logs"),
            log_digests: value
                .get("log_digests")
                .and_then(Json::as_arr)
                .map(<[Json]>::to_vec)
                .unwrap_or_default(),
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
    seal_record(value)
}

/// Bind any persisted record to its own content.
pub fn seal_record(value: &Json) -> Json {
    let body = without_field(value, "integrity");
    let digest = crate::sha256::sha256_hex(body.to_string().as_bytes());
    let mut sealed = body;
    if let Json::Obj(pairs) = &mut sealed {
        pairs.push(("integrity".to_string(), Json::str(digest)));
    }
    sealed
}

pub fn verify_receipt_integrity(value: &Json) -> Result<(), String> {
    verify_record_integrity(value).map_err(|e| {
        if e.contains("record") {
            e.replacen("record", "receipt", 2)
        } else {
            e
        }
    })
}

/// Verify a record sealed by [`seal_record`]. Legacy records without a seal are
/// not rejected here: their own content-derived ids and digests are checked by
/// the module that owns them, and rewriting history is not a migration.
pub fn verify_record_integrity(value: &Json) -> Result<(), String> {
    let recorded = value
        .get("integrity")
        .and_then(Json::as_str)
        .ok_or("record carries no integrity digest")?;
    let body = without_field(value, "integrity");
    let computed = crate::sha256::sha256_hex(body.to_string().as_bytes());
    if computed != recorded {
        return Err(format!(
            "record content does not match its integrity digest (recorded {recorded}, computed {computed})"
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

    #[cfg(not(windows))]
    #[test]
    fn only_a_real_parent_directory_is_flushed_after_the_rename() {
        // Every target the product writes is rooted in the workspace, so the
        // parent is a real directory and its entry is the one the rename must
        // make durable.
        assert_eq!(
            parent_to_flush(Path::new(".flow/journal.jsonl")),
            Some(Path::new(".flow"))
        );
        assert_eq!(
            parent_to_flush(Path::new("/srv/flow/.flow/journal.jsonl")),
            Some(Path::new("/srv/flow/.flow"))
        );
        // A bare relative name has an empty parent: it denotes the process
        // working directory, which `File::open("")` cannot reach. The flush is
        // skipped there, and the write must not fail because of the empty string.
        assert_eq!(parent_to_flush(Path::new("journal.jsonl")), None);
        assert_eq!(parent_to_flush(Path::new("/")), None);
    }

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
    fn an_unreadable_journal_is_an_error_not_an_empty_journal() {
        // A journal path that exists but cannot be read (here: a directory left
        // by a hostile clone or a crashed tool) is not an empty journal. Treating
        // it as empty would append a record and silently drop the history.
        let root = temp_root("journal-unreadable");
        fs::create_dir_all(root.join(".flow")).unwrap();
        fs::create_dir(root.join(".flow").join("journal.jsonl")).unwrap();
        let record = json::parse(r#"{"schema":"flow-journal/1","event":"record"}"#).unwrap();
        let reason = append_journal(&root, &record)
            .expect_err("a directory where the journal should be cannot be read");
        assert!(reason.contains("cannot read journal"), "{reason}");
        assert!(
            !reason.contains("cannot rename"),
            "the refusal must name the read, not the later write: {reason}"
        );
        let _ = fs::remove_dir_all(&root);
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
    fn a_config_whose_obligations_are_not_a_list_is_refused() {
        // A shape Flow does not understand is not an obligation set: guessing
        // "no obligations" would silently accept a policy that runs nothing.
        let value = json::parse(
            r#"{"schema":"flow-config/1","obligations":"none","policy_version":"FLOW-POLICY-1"}"#,
        )
        .unwrap();
        let reason = Config::from_json(&value)
            .err()
            .expect("a non-array obligations field must be refused");
        assert!(reason.contains("obligations must be an array"), "{reason}");
    }

    #[test]
    fn strict_policy_decoder_refuses_every_silent_coercion() {
        let cases: &[(&str, &str)] = &[
            (
                "unknown top-level key",
                r#"{"schema":"flow-config/1","requierd":false,"obligations":[]}"#,
            ),
            (
                "unknown obligation key",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","timeout":1}]}"#,
            ),
            (
                "mixed args",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","args":["test",5]}]}"#,
            ),
            (
                "negative timeout",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","timeout_s":-1}]}"#,
            ),
            (
                "fractional timeout",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","timeout_s":0.4}]}"#,
            ),
            (
                "huge timeout",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","timeout_s":86401}]}"#,
            ),
            (
                "string timeout",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","timeout_s":"60"}]}"#,
            ),
            (
                "non-boolean required",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","required":"yes"}]}"#,
            ),
            (
                "unsupported boundary",
                r#"{"schema":"flow-config/1","execution_boundary":"untrusted","obligations":[]}"#,
            ),
            (
                "non-boolean disabled",
                r#"{"schema":"flow-config/1","disabled":"no","obligations":[]}"#,
            ),
            (
                "bad probes",
                r#"{"schema":"flow-config/1","context_probes":"rustc -Vv","obligations":[]}"#,
            ),
        ];
        for (label, text) in cases {
            let value = json::parse(text).unwrap();
            let reason = Config::from_json(&value)
                .err()
                .unwrap_or_else(|| panic!("{label} must be refused"));
            assert!(
                reason.contains("INVALID_POLICY") || reason.contains("obligation needs"),
                "{label}: {reason}"
            );
        }
    }

    #[test]
    fn every_optional_policy_field_has_an_explicit_default() {
        // A policy that names only its schema is still a policy: every omitted
        // field resolves to the documented default rather than to whatever the
        // decoder happened to leave in an uninitialised variable.
        let minimal = json::parse(r#"{"schema":"flow-config/1"}"#).unwrap();
        let config = Config::from_json(&minimal).expect("a schema-only policy is complete");
        assert_eq!(config.policy_version, POLICY_VERSION);
        assert_eq!(config.execution_boundary, "local-trusted");
        assert!(config.obligations.is_empty());
        assert!(config.context_probes.is_empty());
        assert!(!config.disabled);

        let obligation =
            json::parse(r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git"}]}"#)
                .unwrap();
        let config =
            Config::from_json(&obligation).expect("args, required and timeout are optional");
        assert!(config.obligations[0].args.is_empty());
        assert!(config.obligations[0].required);
        assert_eq!(config.obligations[0].timeout_s, 300);
    }

    #[test]
    fn context_probes_round_trip_and_are_validated_as_lists_of_strings() {
        let full = json::parse(
            r#"{"schema":"flow-config/1","context_probes":[["rustc","-Vv"],["cargo","-V"]]}"#,
        )
        .unwrap();
        let config = Config::from_json(&full).expect("a probe list of string lists is valid");
        assert_eq!(config.context_probes.len(), 2);
        // The writer must reproduce the probes it read: a policy that drops its
        // probes on save would silently change the context identity.
        let written = config.to_json();
        assert_eq!(
            Config::from_json(&written)
                .expect("round trip")
                .context_probes,
            config.context_probes
        );

        for (label, text) in [
            (
                "a probe that is not a list",
                r#"{"schema":"flow-config/1","context_probes":["rustc","-Vv"]}"#,
            ),
            (
                "an empty probe",
                r#"{"schema":"flow-config/1","context_probes":[[]]}"#,
            ),
            (
                "a non-string probe argument",
                r#"{"schema":"flow-config/1","context_probes":[["rustc",7]]}"#,
            ),
        ] {
            let value = json::parse(text).unwrap();
            let reason = Config::from_json(&value)
                .err()
                .unwrap_or_else(|| panic!("{label} must be refused"));
            assert!(reason.contains("INVALID_POLICY"), "{label}: {reason}");
        }
    }

    #[test]
    fn the_decoder_refuses_every_malformed_policy_shape_by_name() {
        let cases: &[(&str, &str, &str)] = &[
            ("not an object", r#"[]"#, "the policy must be a JSON object"),
            (
                "empty policy version",
                r#"{"schema":"flow-config/1","policy_version":""}"#,
                "policy_version must not be empty",
            ),
            (
                "non-string policy version",
                r#"{"schema":"flow-config/1","policy_version":1}"#,
                "policy_version must be a string",
            ),
            (
                "non-string boundary",
                r#"{"schema":"flow-config/1","execution_boundary":7}"#,
                "execution_boundary must be a string",
            ),
            (
                "obligation not an object",
                r#"{"schema":"flow-config/1","obligations":[[]]}"#,
                "every obligation must be an object",
            ),
            (
                "program not a string",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":5}]}"#,
                "obligation program must be a string",
            ),
            (
                "id not a string",
                r#"{"schema":"flow-config/1","obligations":[{"id":5,"program":"git"}]}"#,
                "obligation id must be a string",
            ),
            (
                "args not an array",
                r#"{"schema":"flow-config/1","obligations":[{"id":"x","program":"git","args":"test"}]}"#,
                "obligation args must be an array",
            ),
        ];
        for (label, text, expected) in cases {
            let value = json::parse(text).unwrap();
            let reason = Config::from_json(&value)
                .err()
                .unwrap_or_else(|| panic!("{label} must be refused"));
            assert!(reason.contains(expected), "{label}: {reason}");
        }
    }

    #[test]
    fn a_state_directory_that_is_a_plain_file_is_named_by_every_writer() {
        let root = temp_root("state-as-file");
        fs::write(root.join(".flow"), "not a directory").unwrap();
        let config = Config::default();
        let reason = save_config(&root, &config).expect_err("a file cannot hold the policy");
        assert!(reason.contains("cannot create .flow"), "{reason}");
        let record = json::parse(r#"{"schema":"flow-journal/1","event":"record"}"#).unwrap();
        let reason = append_journal(&root, &record).expect_err("a file cannot hold the journal");
        assert!(reason.contains("cannot create .flow"), "{reason}");
        let receipt = json::parse(
            r#"{"tree_oid":"0000000000000000000000000000000000000000","verification_scope":{"mode":"complete"}}"#,
        )
        .unwrap();
        let reason = save_receipt(&root, &receipt).expect_err("a file cannot hold a receipt");
        assert!(reason.contains("cannot create receipts dir"), "{reason}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_blob_store_refuses_a_collision_and_an_invalid_digest() {
        let root = temp_root("blob-collision");
        let bytes = b"first contents";
        let digest = put_blob(&root, bytes).expect("the store accepts bytes");
        assert_eq!(verified_blob(&root, &digest).unwrap(), bytes.to_vec());
        // Someone replaced the object under its own name: that is a collision,
        // never an alias for two contents.
        let path = blob_path(&root, &digest).expect("a valid digest has a path");
        fs::write(&path, b"other contents").unwrap();
        let reason = put_blob(&root, bytes).expect_err("a replaced blob is a collision");
        assert!(reason.contains("BLOB_COLLISION"), "{reason}");
        let reason = verified_blob(&root, "not-a-digest").expect_err("a non-digest names nothing");
        assert!(reason.contains("PROPOSAL_INTEGRITY"), "{reason}");
        // A parent that is a plain file cannot hold a store entry.
        let file = root.join("plain-file");
        fs::write(&file, b"x").unwrap();
        let reason =
            create_once(&file.join("child"), b"data").expect_err("a file cannot hold an entry");
        assert!(reason.contains("cannot create"), "{reason}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_journal_rotates_and_still_reads_every_record_in_order() {
        let root = temp_root("journal-rotate");
        let payload = "x".repeat(60_000);
        let mut expected = Vec::new();
        for index in 0..12 {
            let record = json::parse(&format!(
                r#"{{"schema":"flow-journal/1","event":"record","id":"id-{index}","payload":"{payload}"}}"#
            ))
            .unwrap();
            append_journal(&root, &record).unwrap();
            expected.push(format!("id-{index}"));
        }
        let segments = journal_segments(&root).unwrap();
        assert!(
            segments.len() >= 2,
            "the history must have been archived, not kept in one growing file: {segments:?}"
        );
        assert!(
            fs::metadata(journal_path(&root)).unwrap().len() <= JOURNAL_SEGMENT_BYTES,
            "the active segment is bounded"
        );
        let records = read_journal(&root).unwrap();
        let ids: Vec<String> = records
            .iter()
            .filter_map(|record| record.get("id").and_then(Json::as_str).map(str::to_string))
            .collect();
        assert_eq!(
            ids, expected,
            "archived segments are read oldest first, then the active one"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_journal_without_an_active_segment_keeps_its_archived_history() {
        let root = temp_root("journal-no-active");
        for index in 0..2 {
            append_journal(
                &root,
                &json::parse(&format!(
                    r#"{{"schema":"flow-journal/1","event":"record","id":"id-{index}"}}"#
                ))
                .unwrap(),
            )
            .unwrap();
        }
        // The crash window between the rotation rename and the new active file.
        let archived = flow_dir(&root).join(format!("journal.{:020}.jsonl", 1));
        fs::rename(journal_path(&root), &archived).unwrap();
        assert_eq!(
            read_journal(&root).unwrap().len(),
            2,
            "a missing active segment is an intact history, not an empty one"
        );
        append_journal(
            &root,
            &json::parse(r#"{"schema":"flow-journal/1","event":"record","id":"new"}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(read_journal(&root).unwrap().len(), 3);
        assert!(
            archived.exists(),
            "the archive is never deleted by an append"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_corrupt_archived_segment_is_refused_and_named() {
        let root = temp_root("journal-corrupt-archive");
        fs::create_dir_all(flow_dir(&root)).unwrap();
        let archived = flow_dir(&root).join(format!("journal.{:020}.jsonl", 1));
        fs::write(&archived, "{not a record\n").unwrap();
        match read_journal(&root) {
            Err(JournalError::Corrupt(reason)) => {
                assert!(
                    reason.contains("journal.00000000000000000001.jsonl"),
                    "{reason}"
                );
                assert!(reason.contains("line 1"), "{reason}");
            }
            other => panic!("a corrupt archive must be a named refusal: {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn journal_segments_only_accept_the_archived_name_shape() {
        let root = temp_root("journal-names");
        fs::create_dir_all(flow_dir(&root)).unwrap();
        fs::write(journal_path(&root), "").unwrap();
        let accepted = flow_dir(&root).join(format!("journal.{:020}.jsonl", 7));
        fs::write(&accepted, "").unwrap();
        for foreign in [
            "journal.1.jsonl",
            "journal.abcdefghijklmnopqrst.jsonl",
            "other.jsonl",
        ] {
            fs::write(flow_dir(&root).join(foreign), "").unwrap();
        }
        let segments = journal_segments(&root).unwrap();
        assert_eq!(segments, vec![accepted]);
        assert_eq!(next_journal_sequence(&root), 8);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn measure_journal_append_cost() {
        // Measurement only: the CI gate forbids `ignored` tests, so this runs
        // when explicitly requested and is a no-op (a pass) otherwise.
        if std::env::var("FLOW_JOURNAL_BENCH").as_deref() != Ok("1") {
            return;
        }
        let record = json::parse(
            r#"{"schema":"flow-journal/1","event":"intent","id":"x","ref":"refs/heads/main","old":"a","new":"b","tree":"t","at":"2026-11-15T00:00:00Z"}"#,
        )
        .unwrap();
        for n in [500usize, 1_000, 2_000] {
            let root = temp_root(&format!("journal-bench-{n}"));
            let started = std::time::Instant::now();
            for _ in 0..n {
                append_journal(&root, &record).unwrap();
            }
            let elapsed = started.elapsed();
            let bytes = fs::metadata(journal_path(&root))
                .map(|m| m.len())
                .unwrap_or(0);
            eprintln!(
                "MEASURED journal_append n={n} bytes={bytes} total_ms={} per_append_us={:.1}",
                elapsed.as_millis(),
                elapsed.as_micros() as f64 / n as f64
            );
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn content_addressed_store_creates_once_and_verifies_bytes() {
        let root = temp_root("blob-store");
        let digest = put_blob(&root, b"alpha\n").unwrap();
        assert!(is_content_digest(&digest));
        assert_eq!(put_blob(&root, b"alpha\n").unwrap(), digest);
        assert_eq!(verified_blob(&root, &digest).unwrap(), b"alpha\n");
        assert_eq!(
            blob_relative_path(&digest),
            format!(".flow/blobs/{}/{}", &digest[..2], digest)
        );
        assert!(blob_path(&root, "not-a-digest").is_none());
        assert!(!is_content_digest(&digest.to_uppercase()));
        let path = blob_path(&root, &digest).unwrap();
        fs::write(&path, b"tampered\n").unwrap();
        let err = verified_blob(&root, &digest).expect_err("tampered bytes must be refused");
        assert!(err.contains("PROPOSAL_INTEGRITY"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_once_never_overwrites_an_existing_name() {
        let root = temp_root("create-once");
        let path = root.join(".flow").join("records").join("one.json");
        assert!(create_once(&path, b"first").unwrap());
        assert!(!create_once(&path, b"second").unwrap());
        assert_eq!(fs::read(&path).unwrap(), b"first");
        assert_eq!(
            fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1,
            "no staging file may survive"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn create_once_treats_a_staging_link_as_an_existing_name_not_an_error() {
        use std::os::unix::fs::MetadataExt;

        // A concurrent creator's staging link leaves the final name with two
        // links for a moment. The guard refuses `nlink > 1`; `create_once` must
        // answer "not created" for an existing name instead of failing the whole
        // command (reproduced on macOS by `g1`).
        let root = temp_root("create-once-linked");
        let path = root.join(".flow").join("records").join("two.json");
        assert!(create_once(&path, b"first").unwrap());
        let staging = root.join(".flow").join("records").join("staging");
        fs::hard_link(&path, &staging).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().nlink(),
            2,
            "the regression needs two links on the final name"
        );
        assert!(
            !create_once(&path, b"second").unwrap(),
            "an existing (even multi-linked) name is simply not created"
        );
        assert_eq!(fs::read(&path).unwrap(), b"first");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn receipt_load_reads_run_id_and_log_digests() {
        let root = temp_root("receipt-fields");
        fs::create_dir_all(receipts_dir(&root)).unwrap();
        let tree = "a".repeat(40);
        let text = format!(
            r#"{{"schema":"flow-receipt/2","tree_oid":"{tree}","status":"PASSED","verification_scope":{{"mode":"complete"}},"run_id":"run-1","log_digests":[{{"id":"x","stdout_sha256":"d"}}],"logs":["l"]}}"#
        );
        fs::write(receipt_path(&root, &tree), text).unwrap();
        let receipt = Receipt::load(&root, &tree).unwrap().unwrap();
        assert_eq!(receipt.run_id, "run-1");
        assert_eq!(receipt.log_digests.len(), 1);
        assert_eq!(receipt.logs, vec!["l".to_string()]);
        let _ = fs::remove_dir_all(&root);
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
        // A record without a schema is stamped with this build's journal schema.
        append_journal(&root, &Json::obj(vec![("event", Json::str("untagged"))])).unwrap();
        // A record that is not an object cannot be tagged; it is still appended
        // as the caller wrote it rather than being silently rewritten.
        append_journal(&root, &Json::Num(1.0)).unwrap();
        let records = read_journal(&root).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(
            records[0].get("schema").and_then(Json::as_str),
            Some("flow-journal/0")
        );
        assert_eq!(
            records[1].get("schema").and_then(Json::as_str),
            Some(SCHEMA_JOURNAL)
        );
        assert_eq!(records[2], Json::Num(1.0));

        // A schema-less object is stamped exactly once (the assertion above pins the
        // value), and a record that already carries one is not stamped a second time:
        // two `"schema"` keys in one line are ambiguous, and the last one would
        // silently win in some readers.
        let mut text = fs::read_to_string(journal_path(&root)).unwrap();
        for line in text.lines().take(2) {
            assert_eq!(
                line.matches("\"schema\"").count(),
                1,
                "a record carries exactly one schema stamp: {line}"
            );
        }

        // Blank lines are layout, never records.
        text.push_str("\n \n");
        fs::write(journal_path(&root), &text).unwrap();
        assert_eq!(read_journal(&root).unwrap().len(), 3);
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
        // only moves the clock, while a wrong day count (`/` replaced by `%`) keeps
        // the clock and moves the date by centuries. Neither is visible in the
        // shape alone, so both are recomputed from the same `unix_now()`.
        let secs = unix_now();
        let got = u64::from(hour) * 3600 + u64::from(minute) * 60 + u64::from(second);
        let want = secs % 86_400;
        let delta = got.abs_diff(want);
        assert!(
            delta <= 2 || delta >= 86_398,
            "the clock time must match the epoch second ({got} vs {want}): {now}"
        );
        let (year, month, day) = civil_from_days((secs / 86_400) as i64);
        let want_date = format!("{year:04}-{month:02}-{day:02}");
        assert_eq!(
            &now[..10],
            want_date,
            "the date must be the UTC date of the same epoch second: {now}"
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
