//! Proposals and exact-tree candidates.
//!
//! The contract, in the order the code enforces it:
//!
//! 1. A **proposal** is an immutable, attributed change set: `agent_id`,
//!    `task_id`, `idempotency_key`, a pinned `base_commit`, and a Git patch whose
//!    bytes are hashed into `content_digest`. Re-submitting the same key with the
//!    same bytes returns the same proposal; the same key with different bytes is
//!    refused (`IDEMPOTENCY_CONFLICT`) — a correction is a new proposal id.
//! 2. The patch is applied **only** to a temporary index (`GIT_INDEX_FILE`
//!    pointing outside the repository). The user's worktree and index are never
//!    read for the result and never written.
//! 3. The resulting Git tree is the proposal's `tree_oid`. It is validated:
//!    no absolute/traversal path, no write into `.git/` or `.flow/`, no gitlink,
//!    no escaping symlink, no case-colliding pair, nothing the host cannot
//!    represent faithfully.
//! 4. A **candidate** is the immutable result of applying several proposals, in
//!    the explicit order given, to their common base. The candidate id binds the
//!    tree, the base and the ordered proposal list. Nothing touches a ref here.
//! 5. Assembly refuses ambiguity instead of guessing: different bases, the same
//!    path with incompatible operations (add/delete/rename/mode), or a hunk that
//!    does not apply, produce `BASE_MISMATCH` / `PATH_OPERATION_CONFLICT` /
//!    `CONFLICT` with the failing proposal named.

use std::fs;
use std::path::{Path, PathBuf};

use crate::gitcli::Git;
use crate::json::{self, Json};
use crate::paths;
// One validator for every "is this a Git object id" question in the crate: a
// second copy is a rule that can drift away from the trust boundary.
use crate::paths::is_object_id;
use crate::sha256::sha256_hex;
use crate::state::{self, SCHEMA_CANDIDATE, SCHEMA_PROPOSAL};

/// A patch larger than this is not a proposal, it is a repository dump.
pub const MAX_PATCH_BYTES: usize = 32 * 1024 * 1024;
/// Blobs are read only to validate a symlink target; bound that read too.
const MAX_SYMLINK_TARGET_BYTES: usize = 4096;

pub fn refusal(code: &str, detail: impl Into<String>) -> Json {
    Json::obj(vec![
        ("status", Json::str("REFUSED")),
        ("code", Json::str(code)),
        ("reason", Json::str(detail.into())),
    ])
}

// ------------------------------------------------------------------ records

/// One path touched by a proposal or a candidate, with its Git modes.
#[derive(Debug, Clone)]
pub struct Change {
    pub status: String,
    pub path: String,
    pub old_mode: String,
    pub new_mode: String,
    pub new_oid: String,
}

impl Change {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("path", Json::str(self.path.clone())),
            ("status", Json::str(self.status.clone())),
            ("old_mode", Json::str(self.old_mode.clone())),
            ("new_mode", Json::str(self.new_mode.clone())),
        ])
    }
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub proposal_id: String,
    pub agent_id: String,
    pub task_id: String,
    pub base_commit: String,
    pub base_tree: String,
    pub tree_oid: String,
    pub content_digest: String,
    pub changes: Vec<Change>,
    pub json: Json,
}

impl Proposal {
    fn metadata_path(root: &Path, id: &str) -> PathBuf {
        state::proposals_dir(root).join(format!("{id}.json"))
    }

    pub fn patch_path(root: &Path, id: &str) -> PathBuf {
        state::proposals_dir(root).join(format!("{id}.patch"))
    }

    pub fn load(root: &Path, id: &str) -> Result<Option<Proposal>, String> {
        if !paths::is_record_id(id) {
            return Err(format!(
                "INVALID_RECORD_ID: {id:?} is not a 32-character hex proposal id"
            ));
        }
        let path = Self::metadata_path(root, id);
        if !path.exists() {
            return Ok(None);
        }
        let value = json::parse(
            &fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?,
        )?;
        // A record written by a different schema is never reinterpreted: the
        // fields could mean something else entirely.
        let schema = value.get("schema").and_then(Json::as_str).unwrap_or("");
        if schema != SCHEMA_PROPOSAL {
            return Err(format!(
                "proposal {id} carries schema {schema:?}, not {SCHEMA_PROPOSAL}: refusing to reinterpret it"
            ));
        }
        Ok(Some(Self::from_json(value)))
    }

    pub fn list(root: &Path) -> Result<Vec<Proposal>, String> {
        let dir = state::proposals_dir(root);
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(&dir) else {
            return Ok(out);
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(id) = name.strip_suffix(".json") else {
                continue;
            };
            match Self::load(root, id) {
                Ok(Some(p)) => out.push(p),
                Ok(None) => {}
                Err(e) => return Err(format!("cannot load proposal {id}: {e}")),
            }
        }
        out.sort_by(|a, b| a.proposal_id.cmp(&b.proposal_id));
        Ok(out)
    }

    pub fn from_json(value: Json) -> Proposal {
        let get = |key: &str| {
            value
                .get(key)
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string()
        };
        let changes = value
            .get("changes")
            .and_then(Json::as_arr)
            .map(|items| {
                items
                    .iter()
                    .map(|c| Change {
                        status: c
                            .get("status")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        path: c
                            .get("path")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        old_mode: c
                            .get("old_mode")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        new_mode: c
                            .get("new_mode")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        new_oid: c
                            .get("new_oid")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Proposal {
            proposal_id: get("proposal_id"),
            agent_id: get("agent_id"),
            task_id: get("task_id"),
            base_commit: get("base_commit"),
            base_tree: get("base_tree"),
            tree_oid: get("tree_oid"),
            content_digest: get("content_digest"),
            changes,
            json: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub candidate_id: String,
    pub base_commit: String,
    pub tree_oid: String,
    pub proposals: Vec<String>,
    pub changes: Vec<Change>,
    pub json: Json,
}

impl Candidate {
    pub fn path(root: &Path, id: &str) -> PathBuf {
        state::candidates_dir(root).join(format!("{id}.json"))
    }

    pub fn load(root: &Path, id: &str) -> Result<Option<Candidate>, String> {
        if !paths::is_record_id(id) {
            return Err(format!(
                "INVALID_RECORD_ID: {id:?} is not a 32-character hex candidate id"
            ));
        }
        let path = Self::path(root, id);
        if !path.exists() {
            return Ok(None);
        }
        let value = json::parse(
            &fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?,
        )?;
        let schema = value.get("schema").and_then(Json::as_str).unwrap_or("");
        if schema != SCHEMA_CANDIDATE {
            return Err(format!(
                "candidate {id} carries schema {schema:?}, not {SCHEMA_CANDIDATE}: refusing to reinterpret it"
            ));
        }
        let get = |key: &str| {
            value
                .get(key)
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string()
        };
        let proposals = value
            .get("proposals")
            .and_then(Json::as_arr)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|p| p.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let changes = value
            .get("changes")
            .and_then(Json::as_arr)
            .map(|items| {
                items
                    .iter()
                    .map(|c| Change {
                        status: c
                            .get("status")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        path: c
                            .get("path")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        old_mode: c
                            .get("old_mode")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        new_mode: c
                            .get("new_mode")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        new_oid: c
                            .get("new_oid")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Some(Candidate {
            candidate_id: get("candidate_id"),
            base_commit: get("base_commit"),
            tree_oid: get("tree_oid"),
            proposals,
            changes,
            json: value,
        }))
    }
}

// ------------------------------------------------------------- temp index

/// A throw-away index file outside the repository. Its removal is best effort:
/// a leaked temp index is inert, so a failed removal is not an error.
struct TempIndex {
    path: PathBuf,
}

impl TempIndex {
    fn new() -> TempIndex {
        TempIndex {
            path: std::env::temp_dir().join(format!(
                "flow-index-{}-{}",
                std::process::id(),
                state::unique_suffix()
            )),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempIndex {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// ------------------------------------------------------------- validation

fn ident(value: &str, what: &str) -> Result<String, Json> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err(refusal(
            "INVALID_IDENTITY",
            format!("{what} must be 1..=128 characters, got {value:?}"),
        ));
    }
    if trimmed.starts_with('.')
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(refusal(
            "INVALID_IDENTITY",
            format!("{what} {value:?} must contain only ASCII letters, digits, '-', '_' or '.' and must not start with '.'"),
        ));
    }
    Ok(trimmed.to_string())
}

/// Parse `git diff-tree -r -z --raw --no-renames` output.
fn parse_raw_diff(stdout: &str) -> Result<Vec<Change>, String> {
    let mut changes = Vec::new();
    let mut parts = stdout.split('\0');
    while let Some(meta) = parts.next() {
        if meta.trim().is_empty() {
            continue;
        }
        let path = parts
            .next()
            .ok_or_else(|| "truncated diff-tree output: meta without a path".to_string())?;
        if path.is_empty() {
            continue;
        }
        let mut fields = meta.trim_start_matches(':').split(' ');
        let old_mode = fields.next().unwrap_or("").to_string();
        let new_mode = fields.next().unwrap_or("").to_string();
        let _old_oid = fields.next().unwrap_or("").to_string();
        let new_oid = fields.next().unwrap_or("").to_string();
        let status = fields.next().unwrap_or("").to_string();
        if status.is_empty() {
            return Err(format!("unparsable diff-tree entry {meta:?}"));
        }
        changes.push(Change {
            status,
            path: path.replace('\\', "/"),
            old_mode,
            new_mode,
            new_oid,
        });
    }
    Ok(changes)
}

fn diff_changes(git: &Git, from_tree: &str, to_tree: &str) -> Result<Vec<Change>, String> {
    let out = git.run_bounded(
        &[
            "diff-tree",
            "-r",
            "-z",
            "--raw",
            "--no-renames",
            "--no-commit-id",
            from_tree,
            to_tree,
        ],
        None,
        None,
    )?;
    if out.status != 0 {
        return Err(format!(
            "cannot diff {from_tree}..{to_tree}: {}",
            out.stderr.trim()
        ));
    }
    parse_raw_diff(&out.stdout)
}

/// Paths Flow must never accept from a proposal: they either escape the tree,
/// write Flow's own state, or cannot be represented faithfully on a supported
/// host. Refusing here keeps the failure at admission time, where the agent can
/// still correct its proposal.
fn validate_path(git: &Git, change: &Change) -> Result<(), Json> {
    let path = &change.path;
    if let Err(reason) = paths::validate_relative(path) {
        return Err(refusal(
            "FORBIDDEN_PATH",
            format!("change {path:?} is not a safe repository-relative path: {reason}"),
        ));
    }
    let first = path.split('/').next().unwrap_or("");
    if first == ".git" || path == ".git" {
        return Err(refusal(
            "FORBIDDEN_PATH",
            format!("change {path:?} writes into .git/: Flow never mutates Git metadata through a proposal"),
        ));
    }
    if first == ".flow" || path == ".flow" {
        return Err(refusal(
            "FORBIDDEN_PATH",
            format!(
                "change {path:?} writes into .flow/: Flow's own state is not a proposal target"
            ),
        ));
    }
    if change.new_mode == "160000" || change.old_mode == "160000" {
        return Err(refusal(
            "GITLINK_UNSUPPORTED",
            format!("change {path:?} is a submodule (gitlink): submodule content is not materialised, so it cannot be verified"),
        ));
    }
    if change.new_mode == "120000" && change.status != "D" && !change.new_oid.is_empty() {
        // A symlink whose target escapes the tree makes the checkout write
        // through it, so the materialised tree is no longer the verified tree.
        // Refuse at admission, where the agent can still correct its proposal.
        let out = git
            .run_bounded(&["cat-file", "blob", &change.new_oid], None, None)
            .map_err(|e| refusal("SYMLINK_UNREADABLE", e))?;
        if out.status != 0 {
            return Err(refusal(
                "SYMLINK_UNREADABLE",
                format!(
                    "cannot read the symlink target of {path:?}: {}",
                    out.stderr.trim()
                ),
            ));
        }
        if out.stdout.len() > MAX_SYMLINK_TARGET_BYTES {
            return Err(refusal(
                "SYMLINK_UNREADABLE",
                format!("the symlink target of {path:?} is above {MAX_SYMLINK_TARGET_BYTES} bytes"),
            ));
        }
        let target = out.stdout.trim_end_matches(['\n', '\r']);
        if symlink_target_escapes(target) {
            return Err(refusal(
                "SYMLINK_ESCAPE",
                format!(
                    "change {path:?} is a symlink to {target:?}: an absolute or parent-escaping target is refused"
                ),
            ));
        }
    }
    Ok(())
}

/// True when a proposed symlink's target would leave the tree or resolve
/// absolutely. The rules are deliberately syntactic: Flow cannot resolve a host
/// path at admission time, and a target that *could* escape later must be refused
/// now, where the agent can still correct it.
fn symlink_target_escapes(target: &str) -> bool {
    let absolute = target.starts_with('/')
        || target.starts_with('\\')
        || (target.len() >= 2 && target.as_bytes()[1] == b':');
    let escapes = target.split(['/', '\\']).any(|part| part == "..");
    target.is_empty() || absolute || escapes
}

/// Refuse a tree the host cannot represent or Flow cannot verify, whatever route
/// produced it.
fn validate_tree(git: &Git, tree: &str) -> Result<(), Json> {
    let listing = git
        .run_bounded(&["ls-tree", "-r", "-z", tree], None, None)
        .map_err(|e| refusal("TREE_UNREADABLE", format!("cannot list tree {tree}: {e}")))?;
    if listing.status != 0 {
        return Err(refusal(
            "TREE_UNREADABLE",
            format!("cannot list tree {tree}: {}", listing.stderr.trim()),
        ));
    }
    let entries = parse_tree_listing(tree, &listing.stdout)?;
    let mut folded: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for path in entries {
        *folded.entry(path.to_lowercase()).or_insert(0) += 1;
    }
    if let Some((name, count)) = folded.iter().find(|(_, c)| **c > 1) {
        return Err(refusal(
            "CASE_COLLISION",
            format!(
                "tree {tree} holds {count} paths that differ only by case (for example {name:?}): a case-insensitive host cannot materialise them, so the candidate could not be verified faithfully"
            ),
        ));
    }
    Ok(())
}

/// Parse one `ls-tree -r -z` answer into the paths of a candidate tree.
///
/// This is the parsing half of [`validate_tree`], separated so a hostile or
/// corrupted object store can be simulated byte for byte: an entry without a
/// tab, a gitlink, or a path Flow cannot represent must be a named refusal.
fn parse_tree_listing(tree: &str, listing: &str) -> Result<Vec<String>, Json> {
    let mut paths = Vec::new();
    for entry in listing.split('\0') {
        if entry.trim().is_empty() {
            continue;
        }
        let Some((meta, path)) = entry.split_once('\t') else {
            return Err(refusal(
                "TREE_UNREADABLE",
                format!("unparsable ls-tree entry {entry:?}"),
            ));
        };
        let mode = meta.split_whitespace().next().unwrap_or("");
        if mode == "160000" {
            return Err(refusal(
                "GITLINK_UNSUPPORTED",
                format!("tree {tree} contains a gitlink at {path:?}: submodule content is not materialised"),
            ));
        }
        if let Err(reason) = paths::validate_relative(path) {
            return Err(refusal(
                "FORBIDDEN_PATH",
                format!("tree {tree} contains {path:?}, which is not a safe path: {reason}"),
            ));
        }
        paths.push(path.to_string());
    }
    Ok(paths)
}

// -------------------------------------------------------------- admission

/// Read the patch bytes, bounded.
fn read_patch(path: &Path) -> Result<Vec<u8>, Json> {
    let meta = fs::metadata(path).map_err(|e| {
        refusal(
            "PATCH_UNREADABLE",
            format!("cannot stat patch {}: {e}", path.display()),
        )
    })?;
    if meta.len() as usize > MAX_PATCH_BYTES {
        return Err(refusal(
            "PATCH_TOO_LARGE",
            format!(
                "patch {} is {} bytes, above the {MAX_PATCH_BYTES}-byte admission bound",
                path.display(),
                meta.len()
            ),
        ));
    }
    fs::read(path).map_err(|e| {
        refusal(
            "PATCH_UNREADABLE",
            format!("cannot read patch {}: {e}", path.display()),
        )
    })
}

pub fn admit(
    git: &Git,
    root: &Path,
    agent: &str,
    task: &str,
    base: &str,
    patch_file: &Path,
    key: Option<&str>,
) -> Result<Json, String> {
    let agent = match ident(agent, "agent id") {
        Ok(v) => v,
        Err(refusal) => return Ok(refusal),
    };
    let task = match ident(task, "task id") {
        Ok(v) => v,
        Err(refusal) => return Ok(refusal),
    };
    if !is_object_id(base.trim()) {
        return Ok(refusal(
            "BASE_INVALID",
            format!("base {base:?} is not a 40- or 64-character hex commit id"),
        ));
    }
    let base = base.trim().to_string();
    // The base must be a commit that this repository actually holds: a tree OID
    // or an unknown object would make `read-tree` ambiguous or fail late.
    let base_tree_out = git.run_bounded(&["rev-parse", &format!("{base}^{{tree}}")], None, None)?;
    if base_tree_out.status != 0 {
        return Ok(refusal(
            "BASE_INVALID",
            format!(
                "base {base} is not a commit reachable in this object store: {}",
                base_tree_out.stderr.trim()
            ),
        ));
    }
    let base_tree = base_tree_out.stdout.trim().to_string();

    let patch = match read_patch(patch_file) {
        Ok(bytes) => bytes,
        Err(refusal) => return Ok(refusal),
    };
    let content_digest = sha256_hex(&patch);
    let key = match key {
        // Without an explicit key, the content is the key: re-submitting the same
        // bytes is idempotent, and any edit necessarily produces a new proposal.
        None => content_digest.clone(),
        Some(k) => match ident(k, "idempotency key") {
            Ok(v) => v,
            Err(refusal) => return Ok(refusal),
        },
    };
    let proposal_id = {
        let material =
            format!("{SCHEMA_PROPOSAL}\nagent:{agent}\ntask:{task}\nbase:{base}\nkey:{key}");
        sha256_hex(material.as_bytes())[..32].to_string()
    };

    if let Some(existing) = Proposal::load(root, &proposal_id)? {
        if existing.content_digest != content_digest {
            return Ok(refusal(
                "IDEMPOTENCY_CONFLICT",
                format!(
                    "proposal {proposal_id} already exists with content {} but this patch is {}: a correction is a new proposal, use a new --key",
                    &existing.content_digest[..12],
                    &content_digest[..12]
                ),
            ));
        }
        let mut out = existing.json.clone();
        if let Json::Obj(pairs) = &mut out {
            pairs.push(("idempotent".into(), Json::Bool(true)));
        }
        return Ok(out);
    }

    // Apply to a temporary index only. `read-tree` seeds it from the base tree;
    // `apply --cached` never reads or writes the worktree.
    let index = TempIndex::new();
    let seeded = git.run_bounded(&["read-tree", &base_tree], None, Some(index.path()))?;
    if seeded.status != 0 {
        return Err(format!(
            "cannot seed a temporary index from {base_tree}: {}",
            seeded.stderr.trim()
        ));
    }
    let applied = git.run_bounded(
        &["apply", "--cached", "--binary", "--whitespace=nowarn", "-"],
        Some(&patch),
        Some(index.path()),
    )?;
    if applied.status != 0 {
        return Ok(refusal(
            "PATCH_REFUSED",
            format!(
                "the patch does not apply to base {base} in a temporary index: {}",
                applied.stderr.trim()
            ),
        ));
    }
    let written = git.run_bounded(&["write-tree"], None, Some(index.path()))?;
    if written.status != 0 {
        return Err(format!(
            "cannot write the proposal tree: {}",
            written.stderr.trim()
        ));
    }
    let tree_oid = written.stdout.trim().to_string();
    drop(index);

    let changes = diff_changes(git, &base_tree, &tree_oid)?;
    if changes.is_empty() {
        return Ok(refusal(
            "EMPTY_PROPOSAL",
            "the patch changes nothing against its base: there is no proposal to record",
        ));
    }
    for change in &changes {
        if let Err(refusal) = validate_path(git, change) {
            return Ok(refusal);
        }
    }
    if let Err(refusal) = validate_tree(git, &tree_oid) {
        return Ok(refusal);
    }

    let record = Json::obj(vec![
        ("schema", Json::str(SCHEMA_PROPOSAL)),
        ("status", Json::str("ADMITTED")),
        ("proposal_id", Json::str(proposal_id.clone())),
        ("agent_id", Json::str(agent.clone())),
        ("task_id", Json::str(task.clone())),
        ("idempotency_key", Json::str(key.clone())),
        ("base_commit", Json::str(base.clone())),
        ("base_tree", Json::str(base_tree.clone())),
        ("tree_oid", Json::str(tree_oid.clone())),
        ("content_digest", Json::str(content_digest.clone())),
        ("patch_bytes", Json::Num(patch.len() as f64)),
        ("created_at", Json::str(state::iso8601_now())),
        (
            "changes",
            Json::Arr(changes.iter().map(Change::to_json).collect()),
        ),
        (
            "patch_path",
            Json::str(Proposal::patch_path(root, &proposal_id).to_string_lossy().to_string()),
        ),
        (
            "note",
            Json::str("immutable: re-submitting this key with different content is refused; the proposal tree was built in a temporary index and never touched the worktree"),
        ),
    ]);
    fs::create_dir_all(state::proposals_dir(root))
        .map_err(|e| format!("cannot create proposals dir: {e}"))?;
    state::write_atomic_bytes(&Proposal::patch_path(root, &proposal_id), &patch)?;
    state::write_atomic(
        &Proposal::metadata_path(root, &proposal_id),
        &(record.to_string_pretty() + "\n"),
    )?;
    Ok(record)
}

// --------------------------------------------------------------- assembly

/// Apply one stored patch to the temporary index.
fn apply_patch(git: &Git, index: &Path, patch: &[u8]) -> Result<crate::gitcli::Output, String> {
    git.run_bounded(
        &["apply", "--cached", "--binary", "--whitespace=nowarn", "-"],
        Some(patch),
        Some(index),
    )
}

fn classify_apply_failure(stderr: &str) -> &'static str {
    let text = stderr.to_lowercase();
    if text.contains("already exists") || text.contains("does not exist") {
        "PATH_CONFLICT"
    } else if text.contains("does not match index") || text.contains("does not apply") {
        "HUNK_CONFLICT"
    } else {
        "APPLY_REFUSED"
    }
}

/// Diagnose incompatible operations on one path across the proposals of one
/// candidate, before any patch is applied.
///
/// Rename/delete/mode/symlink collisions are diagnosed, never merged silently:
/// applying them in some order would produce a tree nobody agreed on, and a
/// symlink or a chmod changes what an obligation executes without changing the
/// bytes the patches mention. Split from `assemble` so every branch can be
/// pinned on constructed proposals instead of a repository dance.
fn path_conflicts(proposals: &[Proposal]) -> Option<Json> {
    let mut per_path: std::collections::BTreeMap<&str, Vec<&Proposal>> =
        std::collections::BTreeMap::new();
    for proposal in proposals {
        for change in &proposal.changes {
            per_path
                .entry(change.path.as_str())
                .or_default()
                .push(proposal);
        }
    }
    for (path, owners) in &per_path {
        if owners.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = owners.iter().map(|p| p.proposal_id.as_str()).collect();
        let mut modes: std::collections::BTreeSet<(String, String)> =
            std::collections::BTreeSet::new();
        for proposal in owners {
            for change in proposal.changes.iter().filter(|c| &c.path == path) {
                if change.status != "M"
                    || change.new_mode == "120000"
                    || change.old_mode == "120000"
                {
                    return Some(refusal(
                        "PATH_OPERATION_CONFLICT",
                        format!(
                            "proposal {} performs {:?} on {path:?} while it is also changed by {}: rename/delete/mode/symlink collisions are diagnosed, never merged silently",
                            proposal.proposal_id,
                            change.status,
                            ids.join(", ")
                        ),
                    ));
                }
                modes.insert((change.old_mode.clone(), change.new_mode.clone()));
            }
        }
        if modes.len() > 1 {
            return Some(refusal(
                "PATH_OPERATION_CONFLICT",
                format!(
                    "proposals {} change the mode of {path:?} differently: {:?}",
                    ids.join(", "),
                    modes.into_iter().collect::<Vec<_>>()
                ),
            ));
        }
    }
    None
}

pub fn assemble(root: &Path, proposal_ids: &[String]) -> Result<Json, String> {
    let git = Git::new(root);
    if proposal_ids.is_empty() {
        return Ok(refusal(
            "NO_PROPOSAL",
            "assemble needs at least one --proposal <id>",
        ));
    }
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for id in proposal_ids {
        if !seen.insert(id.as_str()) {
            return Ok(refusal(
                "DUPLICATE_PROPOSAL",
                format!("proposal {id:?} is listed more than once: a candidate applies each proposal exactly once"),
            ));
        }
    }
    let mut proposals: Vec<Proposal> = Vec::new();
    for id in proposal_ids {
        match Proposal::load(root, id)? {
            Some(p) => proposals.push(p),
            None => {
                return Ok(refusal(
                    "UNKNOWN_PROPOSAL",
                    format!("no admitted proposal {id:?} in .flow/proposals/"),
                ))
            }
        }
    }
    let base = proposals[0].base_commit.clone();
    let base_tree = proposals[0].base_tree.clone();
    let mismatched: Vec<String> = proposals
        .iter()
        .filter(|p| p.base_commit != base)
        .map(|p| format!("{}@{}", p.proposal_id, p.base_commit))
        .collect();
    if !mismatched.is_empty() {
        return Ok(refusal(
            "BASE_MISMATCH",
            format!(
                "every proposal in one candidate must share base {base}; these do not: {}",
                mismatched.join(", ")
            ),
        ));
    }

    // Incompatible operations on one path are refused *before* any patch is
    // applied, so the diagnostic does not depend on application order.
    if let Some(refusal) = path_conflicts(&proposals) {
        return Ok(refusal);
    }

    let index = TempIndex::new();
    let seeded = git.run_bounded(&["read-tree", &base_tree], None, Some(index.path()))?;
    if seeded.status != 0 {
        return Err(format!(
            "cannot seed a temporary index from {base_tree}: {}",
            seeded.stderr.trim()
        ));
    }
    for proposal in &proposals {
        let patch = match fs::read(Proposal::patch_path(root, &proposal.proposal_id)) {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(format!(
                    "proposal {} is recorded but its patch is unreadable: {e}",
                    proposal.proposal_id
                ))
            }
        };
        let applied = apply_patch(&git, index.path(), &patch)?;
        if applied.status != 0 {
            let kind = classify_apply_failure(&applied.stderr);
            return Ok(refusal(
                "CONFLICT",
                format!(
                    "{kind}: proposal {} ({} / {}) does not apply on top of the earlier proposals: {}",
                    proposal.proposal_id,
                    proposal.agent_id,
                    proposal.task_id,
                    applied.stderr.trim()
                ),
            ));
        }
    }
    let written = git.run_bounded(&["write-tree"], None, Some(index.path()))?;
    if written.status != 0 {
        return Err(format!(
            "cannot write the candidate tree: {}",
            written.stderr.trim()
        ));
    }
    let tree_oid = written.stdout.trim().to_string();
    drop(index);

    if let Err(refusal) = validate_tree(&git, &tree_oid) {
        return Ok(refusal);
    }
    let changes = diff_changes(&git, &base_tree, &tree_oid)?;
    let candidate_id = {
        let material = format!(
            "{SCHEMA_CANDIDATE}\nbase:{base}\ntree:{tree_oid}\nproposals:{}",
            proposal_ids.join(",")
        );
        sha256_hex(material.as_bytes())[..32].to_string()
    };
    let record = Json::obj(vec![
        ("schema", Json::str(SCHEMA_CANDIDATE)),
        ("status", Json::str("ASSEMBLED")),
        ("candidate_id", Json::str(candidate_id.clone())),
        ("base_commit", Json::str(base.clone())),
        ("base_tree", Json::str(base_tree.clone())),
        ("tree_oid", Json::str(tree_oid.clone())),
        (
            "proposals",
            Json::Arr(proposal_ids.iter().map(|p| Json::str(p.clone())).collect()),
        ),
        ("created_at", Json::str(state::iso8601_now())),
        (
            "attribution",
            Json::Arr(
                proposals
                    .iter()
                    .map(|p| {
                        Json::obj(vec![
                            ("proposal_id", Json::str(p.proposal_id.clone())),
                            ("agent_id", Json::str(p.agent_id.clone())),
                            ("task_id", Json::str(p.task_id.clone())),
                            ("base_commit", Json::str(p.base_commit.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "changes",
            Json::Arr(changes.iter().map(Change::to_json).collect()),
        ),
        (
            "note",
            Json::str("immutable candidate: verify it with `flow run --candidate <id>`, publish it with `flow publish --candidate <id> --expected-old <base>`; no ref, index or worktree was touched"),
        ),
    ]);
    fs::create_dir_all(state::candidates_dir(root))
        .map_err(|e| format!("cannot create candidates dir: {e}"))?;
    let path = Candidate::path(root, &candidate_id);
    if path.exists() {
        // Same inputs, same id. Only an identical record may reuse the slot.
        let existing = Candidate::load(root, &candidate_id)?;
        match existing {
            Some(c) if c.tree_oid == tree_oid && c.proposals == proposal_ids => {}
            _ => {
                return Ok(refusal(
                    "CANDIDATE_IMMUTABLE",
                    format!("candidate {candidate_id} exists with different content: refusing to overwrite it"),
                ))
            }
        }
    } else {
        state::write_atomic(&path, &(record.to_string_pretty() + "\n"))?;
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    // These parsers read answers from a separate Git process. A corrupted object
    // store or a Git that answers nonsense must produce a named refusal, never a
    // panic or a silently folded path.

    #[test]
    fn raw_diffs_are_parsed_and_truncated_answers_are_refused() {
        let changes = parse_raw_diff(
            ":100644 100644 aaaa bbbb M\0a.txt\0:000000 100644 0000 cccc A\0b/c.txt\0",
        )
        .expect("a well-formed diff parses");
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].status, "M");
        assert_eq!(changes[0].path, "a.txt");
        assert_eq!(changes[0].old_mode, "100644");
        assert_eq!(changes[0].new_mode, "100644");
        assert_eq!(changes[0].new_oid, "bbbb");
        assert_eq!(changes[1].status, "A");
        assert_eq!(changes[1].path, "b/c.txt");

        let reason = parse_raw_diff(":100644 100644 a b M").expect_err("meta without a path");
        assert!(reason.contains("truncated diff-tree output"), "{reason}");

        let reason = parse_raw_diff(":100644 100644 a b\0p\0").expect_err("missing status");
        assert!(reason.contains("unparsable diff-tree entry"), "{reason}");

        // An empty path is skipped, not recorded as a change of `""`.
        assert!(parse_raw_diff(":100644 100644 a b M\0\0")
            .unwrap()
            .is_empty());
        assert!(parse_raw_diff("").unwrap().is_empty());
    }

    #[test]
    fn apply_failures_are_classified_by_their_cause() {
        assert_eq!(
            classify_apply_failure("error: a.txt: already exists in working directory"),
            "PATH_CONFLICT"
        );
        assert_eq!(
            classify_apply_failure("error: b.txt: does not exist in index"),
            "PATH_CONFLICT"
        );
        assert_eq!(
            classify_apply_failure("error: patch does not match index"),
            "HUNK_CONFLICT"
        );
        assert_eq!(
            classify_apply_failure("error: patch does not apply"),
            "HUNK_CONFLICT"
        );
        assert_eq!(
            classify_apply_failure("error: unknown failure"),
            "APPLY_REFUSED"
        );
        // Git's wording is matched case-insensitively.
        assert_eq!(
            classify_apply_failure("ERROR: FILE ALREADY EXISTS"),
            "PATH_CONFLICT"
        );
    }

    #[test]
    fn symlink_targets_that_leave_the_tree_are_refused() {
        for escaping in [
            "",
            "/etc/passwd",
            "\\server\\share",
            "C:/Windows",
            "c:x",
            "../a",
            "a/../../b",
            "..\\x",
            // A `..` component is refused even when it would normalise back
            // inside: admission cannot prove where the tree is materialised.
            "a/b/../c",
        ] {
            assert!(symlink_target_escapes(escaping), "{escaping:?} must escape");
        }
        for safe in ["a", "a/b", "./a", "..hidden", "a..b"] {
            assert!(!symlink_target_escapes(safe), "{safe:?} stays inside");
        }
    }

    #[test]
    fn tree_listings_are_parsed_and_every_unsafe_entry_is_refused() {
        let listing =
            "100644 blob aaaa\tb.txt\x00040000 tree bbbb\tsrc\x00120000 blob cccc\tlink\0";
        let paths = parse_tree_listing("t", listing).expect("a well-formed listing parses");
        assert_eq!(
            paths,
            vec!["b.txt".to_string(), "src".to_string(), "link".to_string()]
        );
        assert!(parse_tree_listing("t", "\0\0").unwrap().is_empty());

        let reason = parse_tree_listing("t", "100644 blob aaaa").expect_err("no tab");
        assert_eq!(
            reason.get("code").and_then(Json::as_str),
            Some("TREE_UNREADABLE")
        );

        let reason = parse_tree_listing("t", "160000 commit aaaa\tsub\0").expect_err("gitlink");
        assert_eq!(
            reason.get("code").and_then(Json::as_str),
            Some("GITLINK_UNSUPPORTED")
        );

        for unsafe_path in ["../x", "a/../../b", "/abs", ""] {
            let entry = format!("100644 blob aaaa\t{unsafe_path}\0");
            let reason = parse_tree_listing("t", &entry).expect_err("unsafe tree path");
            assert_eq!(
                reason.get("code").and_then(Json::as_str),
                Some("FORBIDDEN_PATH"),
                "{entry:?}"
            );
        }
    }

    // --------------------------------------------------- admission refusals

    fn change(status: &str, path: &str, old_mode: &str, new_mode: &str, new_oid: &str) -> Change {
        Change {
            status: status.to_string(),
            path: path.to_string(),
            old_mode: old_mode.to_string(),
            new_mode: new_mode.to_string(),
            new_oid: new_oid.to_string(),
        }
    }

    /// A `Git` whose command fails: every path that runs before Git is reached
    /// can be tested without building a repository, and the paths that do reach
    /// Git see a non-zero answer (the "cannot be read" refusal).
    fn failing_git() -> Git {
        Git::new(std::env::temp_dir())
    }

    fn code_of(refusal: &Json) -> String {
        refusal
            .get("code")
            .and_then(Json::as_str)
            .unwrap_or("<none>")
            .to_string()
    }

    #[test]
    fn validate_path_refuses_metadata_state_gitlinks_and_escaping_symlinks() {
        let git = failing_git();
        for (entry, expected) in [
            (
                change("M", "../x", "100644", "100644", "aa"),
                "FORBIDDEN_PATH",
            ),
            (
                change("M", ".git/config", "100644", "100644", "aa"),
                "FORBIDDEN_PATH",
            ),
            (
                change("M", ".git", "100644", "100644", "aa"),
                "FORBIDDEN_PATH",
            ),
            (
                change("M", ".flow/state", "100644", "100644", "aa"),
                "FORBIDDEN_PATH",
            ),
            (
                change("M", "sub/module", "160000", "160000", "aa"),
                "GITLINK_UNSUPPORTED",
            ),
            (
                change("D", "sub/module", "160000", "000000", "aa"),
                "GITLINK_UNSUPPORTED",
            ),
            // A symlink whose blob this repository does not hold cannot be
            // inspected, so admission refuses rather than assuming a safe target.
            (
                change("M", "link", "100644", "120000", "abcd"),
                "SYMLINK_UNREADABLE",
            ),
        ] {
            let refusal = validate_path(&git, &entry).expect_err("the change must be refused");
            assert_eq!(code_of(&refusal), expected, "{entry:?}: {refusal}");
        }
        // A deletion of a symlink has no target to inspect, and an ordinary file
        // change touches no symlink: both are admitted here.
        validate_path(&git, &change("D", "link", "120000", "000000", "aa")).expect("deletion");
        validate_path(&git, &change("M", "link", "120000", "120000", "")).expect("empty target");
        validate_path(&git, &change("M", "a/b.txt", "100644", "100644", "aa")).expect("plain file");
    }

    #[test]
    fn an_unreadable_diff_or_tree_listing_is_a_named_refusal() {
        let git = failing_git();
        let reason = diff_changes(&git, "aaaa", "bbbb").expect_err("git cannot diff here");
        assert!(reason.contains("cannot diff"), "{reason}");
        let refusal = validate_tree(&git, "0000").expect_err("git cannot list this tree");
        assert_eq!(code_of(&refusal), "TREE_UNREADABLE");
    }

    // ------------------------------------------ cross-proposal path conflicts

    fn proposal(id: &str, changes: Vec<Change>) -> Proposal {
        Proposal {
            proposal_id: id.to_string(),
            agent_id: "agent".to_string(),
            task_id: "task".to_string(),
            base_commit: "base".to_string(),
            base_tree: "base-tree".to_string(),
            tree_oid: "tree".to_string(),
            content_digest: "digest".to_string(),
            changes,
            json: Json::Null,
        }
    }

    #[test]
    fn proposals_on_different_paths_never_conflict() {
        let one = proposal("one", vec![change("M", "a.txt", "100644", "100644", "aa")]);
        let two = proposal("two", vec![change("M", "b.txt", "100644", "100644", "bb")]);
        assert!(path_conflicts(&[one, two]).is_none());
    }

    #[test]
    fn identities_are_bounded_at_128_characters() {
        // The bound is inclusive: an agent id of exactly 128 characters is valid,
        // 129 is not. A `>=` there would silently shrink the documented contract.
        assert!(
            ident(&"a".repeat(128), "agent id").is_ok(),
            "128 characters must be accepted"
        );
        let refusal = ident(&"a".repeat(129), "agent id").expect_err("129 is above the bound");
        assert_eq!(code_of(&refusal), "INVALID_IDENTITY");
        assert_eq!(
            ident("a", "agent id").unwrap(),
            "a",
            "a one-character id is the lower bound"
        );
        assert_eq!(
            ident("", "agent id")
                .unwrap_err()
                .get("code")
                .and_then(Json::as_str),
            Some("INVALID_IDENTITY")
        );
    }

    #[test]
    fn a_temporary_index_is_removed_when_dropped() {
        // The index is a real file in the temp directory; `Drop` must not leak it.
        // It is inert when leaked, which is exactly why nothing else would notice.
        let index = TempIndex::new();
        let path = index.path().to_path_buf();
        fs::write(&path, "").expect("the temp index is writable");
        assert!(path.exists());
        drop(index);
        assert!(!path.exists(), "a dropped temp index must be removed");
    }

    #[test]
    fn two_edits_on_one_path_are_merged_only_when_they_are_plain_content_edits() {
        let one = proposal("one", vec![change("M", "a.txt", "100644", "100644", "aa")]);
        let two = proposal("two", vec![change("M", "a.txt", "100644", "100644", "bb")]);
        assert!(
            path_conflicts(&[one.clone(), two]).is_none(),
            "two plain content edits on one path are compatible"
        );

        // A delete (or any non-`M` status) on a path another proposal edits is
        // diagnosed instead of being applied in some order.
        let delete = proposal("three", vec![change("D", "a.txt", "100644", "000000", "")]);
        let refusal = path_conflicts(&[one.clone(), delete]).expect("a delete collides");
        assert_eq!(code_of(&refusal), "PATH_OPERATION_CONFLICT");

        // A symlink conversion is a collision too: the bytes do not say that the
        // path stopped being a regular file.
        let to_symlink = proposal("four", vec![change("M", "a.txt", "100644", "120000", "cc")]);
        assert_eq!(
            code_of(&path_conflicts(&[one.clone(), to_symlink]).expect("type change collides")),
            "PATH_OPERATION_CONFLICT"
        );
        let from_symlink = proposal("five", vec![change("M", "a.txt", "120000", "100644", "dd")]);
        assert_eq!(
            code_of(&path_conflicts(&[one.clone(), from_symlink]).expect("type change collides")),
            "PATH_OPERATION_CONFLICT"
        );

        // Two plain edits that disagree on the resulting mode are refused: one of
        // them would silently win.
        let changed_mode = proposal("six", vec![change("M", "a.txt", "100644", "100755", "ee")]);
        assert_eq!(
            code_of(&path_conflicts(&[one, changed_mode]).expect("mode disagreement collides")),
            "PATH_OPERATION_CONFLICT"
        );

        // Two deletes of the same path are a collision even though both report
        // the same (old, new) modes: applying them in order would make the second
        // patch fail with a generic conflict instead of naming the operation
        // collision up front.
        let delete_one = proposal("seven", vec![change("D", "a.txt", "100644", "000000", "")]);
        let delete_two = proposal("eight", vec![change("D", "a.txt", "100644", "000000", "")]);
        assert_eq!(
            code_of(&path_conflicts(&[delete_one, delete_two]).expect("two deletes collide")),
            "PATH_OPERATION_CONFLICT"
        );

        // A file that two proposals turn into a symlink: the mode pair is the
        // same for both, so only the symlink branch of the guard catches it.
        let symlink_one = proposal("nine", vec![change("M", "a.txt", "100644", "120000", "ff")]);
        let symlink_two = proposal("ten", vec![change("M", "a.txt", "100644", "120000", "gg")]);
        assert_eq!(
            code_of(
                &path_conflicts(&[symlink_one, symlink_two]).expect("two symlink changes collide")
            ),
            "PATH_OPERATION_CONFLICT"
        );
    }

    // A deterministic, dependency-free generator (the same note as in
    // `commands.rs`): a randomised sweep on every build, not coverage-guided
    // fuzzing. No dependency is added for it because the crate is deliberately
    // zero-dependency and CI runs `--locked --offline`.
    struct Rand(u64);

    impl Rand {
        fn below(&mut self, bound: usize) -> usize {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            (x.wrapping_mul(0x2545_F491_4F6C_DD1D) % bound as u64) as usize
        }

        fn bytes(&mut self, max: usize) -> Vec<u8> {
            const ALPHABET: &[u8] = b"\0\t\n /\\:.-_a0\xff";
            let len = self.below(max + 1);
            let mut out = Vec::with_capacity(len);
            for _ in 0..len {
                if self.below(4) == 0 {
                    out.push(self.below(256) as u8);
                } else {
                    out.push(ALPHABET[self.below(ALPHABET.len())]);
                }
            }
            out
        }
    }

    fn random_target(random: &mut Rand) -> String {
        const PIECES: &[&str] = &["a", "b", "/", "\\", ".", "-", "_", " ", ":", ".."];
        let len = random.below(10);
        let mut out = String::new();
        for _ in 0..len {
            out.push_str(PIECES[random.below(PIECES.len())]);
        }
        out
    }

    #[test]
    fn arbitrary_tree_listings_and_raw_diffs_are_parsed_or_refused_and_never_panic() {
        let mut random = Rand(0x5EED_2026_1357_9BDF);
        let mut parsed = 0usize;
        let mut refused = 0usize;
        for _ in 0..2_000 {
            let listing = if random.below(2) == 0 {
                let mut out = String::new();
                for _ in 0..(1 + random.below(3)) {
                    out.push_str(if random.below(8) == 0 {
                        "160000 commit aaaa\t"
                    } else {
                        "100644 blob aaaa\t"
                    });
                    out.push_str(&random_target(&mut random));
                    out.push('\0');
                }
                out
            } else {
                String::from_utf8_lossy(&random.bytes(96)).into_owned()
            };
            match parse_tree_listing("t", &listing) {
                Ok(paths) => {
                    parsed += 1;
                    for path in paths {
                        assert!(
                            crate::paths::validate_relative(&path).is_ok(),
                            "{path:?} was accepted but is not a safe path"
                        );
                    }
                }
                Err(reason) => {
                    refused += 1;
                    let code = reason.get("code").and_then(Json::as_str);
                    assert!(
                        matches!(
                            code,
                            Some("TREE_UNREADABLE" | "GITLINK_UNSUPPORTED" | "FORBIDDEN_PATH")
                        ),
                        "{reason:?}"
                    );
                }
            }
            match parse_raw_diff(&String::from_utf8_lossy(&random.bytes(96))) {
                Ok(changes) => {
                    parsed += 1;
                    for change in changes {
                        assert!(!change.status.is_empty(), "a change without a status");
                        assert!(!change.path.contains('\\'), "separators are normalised");
                    }
                }
                Err(reason) => {
                    refused += 1;
                    assert!(!reason.is_empty());
                }
            }
        }
        assert!(
            parsed > 100 && refused > 100,
            "both answers must be exercised, got {parsed} parsed and {refused} refused"
        );
    }

    #[test]
    fn identities_follow_the_bound_and_the_charset_on_random_input() {
        const CHARSET: &[char] = &['a', 'Z', '0', '9', '.', '_', '-'];
        const NOISE: &[char] = &[' ', ':', '/', 'é', '\t'];
        let mut random = Rand(0x5EED_2026_2468_ACE0);
        let mut accepted = 0usize;
        let mut refused = 0usize;
        for _ in 0..4_000 {
            let len = random.below(141);
            let mut value: String = (0..len)
                .map(|_| CHARSET[random.below(CHARSET.len())])
                .collect();
            if random.below(10) < 3 {
                let at = random.below(len + 1);
                value.insert(at, NOISE[random.below(NOISE.len())]);
            }
            let trimmed = value.trim();
            let allowed = !trimmed.is_empty()
                && trimmed.len() <= 128
                && !trimmed.starts_with('.')
                && trimmed
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
            match ident(&value, "proposal") {
                Ok(got) => {
                    accepted += 1;
                    assert!(allowed, "{value:?} was accepted but breaks the stated rule");
                    assert_eq!(got, trimmed, "the identity is the trimmed value");
                }
                Err(reason) => {
                    refused += 1;
                    assert!(
                        !allowed,
                        "{value:?} was refused but satisfies the stated rule"
                    );
                    assert_eq!(
                        reason.get("code").and_then(Json::as_str),
                        Some("INVALID_IDENTITY")
                    );
                }
            }
        }
        assert!(
            accepted > 100 && refused > 100,
            "both answers must be exercised, got {accepted} accepted and {refused} refused"
        );
    }

    #[test]
    fn symlink_targets_are_separator_symmetric_and_a_safe_target_stays_safe() {
        let mut random = Rand(0x5EED_2026_0F0F_5EED);
        for _ in 0..4_000 {
            let target = random_target(&mut random);
            let escapes = symlink_target_escapes(&target);
            // The rule cannot depend on which separator a host's Git happens to
            // print, so both spellings must answer the same.
            assert_eq!(
                escapes,
                symlink_target_escapes(&target.replace('\\', "/")),
                "{target:?}"
            );
            assert_eq!(
                escapes,
                symlink_target_escapes(&target.replace('/', "\\")),
                "{target:?}"
            );
            let absolute = target.starts_with('/')
                || target.starts_with('\\')
                || (target.len() >= 2 && target.as_bytes()[1] == b':');
            let traverses = target.split(['/', '\\']).any(|part| part == "..");
            assert_eq!(
                escapes,
                target.is_empty() || absolute || traverses,
                "{target:?} does not follow the documented rule"
            );
            if !escapes {
                assert!(
                    !symlink_target_escapes(&format!("{target}/a")),
                    "{target:?} plus a plain component must stay safe"
                );
                assert!(
                    !symlink_target_escapes(&format!("d/{target}")),
                    "{target:?} below a plain directory must stay safe"
                );
            }
        }
    }
}
