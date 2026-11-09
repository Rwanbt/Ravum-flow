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

fn is_object_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.chars().all(|c| c.is_ascii_hexdigit())
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
        let absolute = target.starts_with('/')
            || target.starts_with('\\')
            || (target.len() >= 2 && target.as_bytes()[1] == b':');
        let escapes = target.split(['/', '\\']).any(|part| part == "..");
        if target.is_empty() || absolute || escapes {
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
    let mut folded: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for entry in listing.stdout.split('\0') {
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
    let mut per_path: std::collections::BTreeMap<&str, Vec<&Proposal>> =
        std::collections::BTreeMap::new();
    for proposal in &proposals {
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
                    return Ok(refusal(
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
            return Ok(refusal(
                "PATH_OPERATION_CONFLICT",
                format!(
                    "proposals {} change the mode of {path:?} differently: {:?}",
                    ids.join(", "),
                    modes.into_iter().collect::<Vec<_>>()
                ),
            ));
        }
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
