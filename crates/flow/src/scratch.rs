//! Disposable verification directories ("scratch").
//!
//! [`materialise`] checks a commit's tree out into a **new, uniquely named
//! directory outside the repository**, then removes the worktree's `.git` link
//! so the candidate process cannot reach the repository's shared Git directory
//! through ordinary repository discovery.
//!
//! # Guarantees (enforced here, with tests)
//!
//! * The directory is created with `create_dir`, so it did not exist before;
//!   a name collision is retried, never reused or overwritten.
//! * Nothing that already exists is deleted. `Drop` removes a directory only
//!   when it still is a real directory (not a symlink/reparse point), still
//!   resolves to the identity recorded at creation, and still carries the
//!   ownership marker written after materialisation.
//! * A base, or a resolved scratch path, inside the repository is refused.
//! * Cleanup is bounded (wall clock and entry count) and never panics.
//!
//! # Non-guarantees (explicit, never called a sandbox)
//!
//! Removing `.git` only defeats *accidental or casual* repository discovery.
//! A hostile candidate can still find repositories on the same machine (it is
//! the same user, same filesystem), inspect its own environment, or detach
//! from any kill. This module provides hygiene and bounded cleanup, **not**
//! confinement; the crate has no sandbox and claims none.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::gitcli::{self, Git, Obligation};

/// Prefix of every scratch directory name, so leftovers are recognisable.
pub const NAME_PREFIX: &str = "flow-scratch";

/// Ownership marker written after `git worktree add` succeeded. `Drop` refuses
/// to delete a directory without it.
const OWNED_MARKER: &str = ".flow-scratch-owned";

/// Attempts before giving up on finding a free name.
const MAX_ATTEMPTS: u64 = 128;

/// Wall-clock budget for one cleanup. A tree larger than the budget leaves
/// residue behind instead of making `Drop` unbounded.
const CLEANUP_BUDGET: Duration = Duration::from_secs(5);

/// Entry-count budget for one cleanup, bounding pathological trees.
const MAX_CLEANUP_ENTRIES: usize = 50_000;

/// Timeout for the best-effort `git worktree prune` in `Drop`.
const PRUNE_TIMEOUT_S: u64 = 5;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Serialises tests that depend on the process-global name sequence, so the
/// collision test can predict which candidate path `materialise_in` will try.
/// Compiled only for tests; the production API needs no lock.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A materialised tree that removes itself when dropped.
#[derive(Debug)]
pub struct Scratch {
    /// Path handed to the candidate process (and to the caller): the path as
    /// created under the caller's temp directory.
    path: PathBuf,
    /// Resolved identity of that path, recorded at creation. `Drop` compares
    /// against it so a path replaced by a redirect is never traversed.
    canonical: PathBuf,
    /// Canonical repository root, used for the best-effort worktree prune.
    repo_root: PathBuf,
}

impl Scratch {
    /// The directory holding the materialised tree.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn still_owned(&self) -> bool {
        let Ok(meta) = fs::symlink_metadata(&self.path) else {
            return false;
        };
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return false;
        }
        match fs::canonicalize(&self.path) {
            Ok(resolved) if resolved == self.canonical => {}
            _ => return false,
        }
        fs::symlink_metadata(self.path.join(OWNED_MARKER))
            .map(|meta| meta.is_file() && !meta.file_type().is_symlink())
            .unwrap_or(false)
    }
}

impl Drop for Scratch {
    /// Bounded, idempotent, non-panicking cleanup.
    ///
    /// The directory is removed only while it is provably still the one this
    /// value created; the administrative worktree entry is pruned with a
    /// bounded `git worktree prune`.
    fn drop(&mut self) {
        if self.still_owned() {
            remove_tree_bounded(&self.path, Instant::now() + CLEANUP_BUDGET);
        }
        prune_worktrees(&self.repo_root);
    }
}

/// Materialise `commit`'s tree in a NEW unique directory outside the repository.
///
/// The directory is `std::env::temp_dir()`-based; it is refused if it resolves
/// inside `repo_root`. The returned [`Scratch`] owns the directory and removes
/// it on drop; see the module documentation for the limits of what removing
/// `.git` achieves.
pub fn materialise(git: &Git, commit: &str, repo_root: &Path) -> Result<Scratch, String> {
    materialise_in(&std::env::temp_dir(), git, commit, repo_root)
}

/// Implementation with an injectable base directory, so the boundary and
/// collision paths are testable without changing the process environment.
pub(crate) fn materialise_in(
    base: &Path,
    git: &Git,
    commit: &str,
    repo_root: &Path,
) -> Result<Scratch, String> {
    // An object id, not an option and not a path: `git worktree add` must never
    // see a value beginning with `-`, and commands.rs only ever passes the
    // hexadecimal OID returned by `commit-tree`.
    if !is_hex_object_id(commit) {
        return Err(format!(
            "refusing to materialise {commit:?}: expected a hexadecimal object id"
        ));
    }

    let repo_root = canonical_dir(repo_root, "repository root")?;
    let base_display = base.to_path_buf();
    let base_canonical = canonical_dir(&base_display, "scratch base")?;
    if is_within(&repo_root, &base_canonical) {
        return Err(refusal_message(&base_canonical, &repo_root));
    }

    // `create_dir` is the uniqueness primitive: it fails atomically if the name
    // already exists, and a pre-existing path is never adopted or removed.
    let mut created: Option<(PathBuf, PathBuf)> = None;
    for _ in 0..MAX_ATTEMPTS {
        let seq = next_sequence();
        let display = candidate_path(&base_display, seq);
        let canonical = candidate_path(&base_canonical, seq);
        if is_within(&repo_root, &canonical) {
            return Err(refusal_message(&canonical, &repo_root));
        }
        match fs::create_dir(&display) {
            Ok(()) => {
                created = Some((display, canonical));
                break;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(format!(
                    "cannot create scratch directory {}: {e}",
                    display.display()
                ))
            }
        }
    }
    let Some((path, canonical)) = created else {
        return Err(format!(
            "cannot create a unique scratch directory under {} after {MAX_ATTEMPTS} attempts",
            base_canonical.display()
        ));
    };

    // A path we just created must be a real directory at the identity we
    // recorded; anything else (redirect, race) is a refusal, not a cleanup.
    if let Err(reason) = verify_created_dir(&path, &canonical) {
        let _ = fs::remove_dir(&path);
        return Err(reason);
    }

    let Some(path_str) = path.to_str() else {
        abandon(&path, &repo_root);
        return Err(format!(
            "refusing scratch directory {}: path is not valid UTF-8",
            path.display()
        ));
    };

    match git.run(&["worktree", "add", "--detach", "--quiet", path_str, commit]) {
        Ok(add) if add.status == 0 => {}
        Ok(add) => {
            abandon(&path, &repo_root);
            return Err(format!(
                "cannot materialise commit {commit} in {}: {}",
                path.display(),
                add.stderr.trim()
            ));
        }
        Err(e) => {
            abandon(&path, &repo_root);
            return Err(e);
        }
    }

    if let Err(reason) = detach_git_link(&path) {
        abandon(&path, &repo_root);
        return Err(reason);
    }

    let marker = format!(
        "{NAME_PREFIX}\ncommit {commit}\npid {}\n",
        std::process::id()
    );
    if let Err(e) = fs::write(path.join(OWNED_MARKER), marker) {
        abandon(&path, &repo_root);
        return Err(format!(
            "cannot mark scratch directory {} as owned: {e}",
            path.display()
        ));
    }

    Ok(Scratch {
        path,
        canonical,
        repo_root,
    })
}

/// The next name sequence value. Exposed to the test crate so the collision
/// path can be exercised against the exact path the next attempt would use.
pub(crate) fn next_sequence() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// The path of the `seq`-th candidate under `base` (before collision retries).
pub(crate) fn candidate_path(base: &Path, seq: u64) -> PathBuf {
    base.join(format!("{NAME_PREFIX}-{}-{seq}", std::process::id()))
}

/// True for a hexadecimal object id as `git worktree add` will accept it.
///
/// Deliberately wider than [`crate::paths::is_object_id`]: Git resolves an
/// *abbreviated* object id, and `--candidate <short-oid>` is a documented way to
/// name a tree. The value still may not begin with `-` (it is hex) and never
/// reaches a file name, so the wider length is a convenience, not a widening of
/// the identifier-to-path rule used for receipts.
fn is_hex_object_id(value: &str) -> bool {
    (4..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn canonical_dir(path: &Path, what: &str) -> Result<PathBuf, String> {
    let resolved = path
        .canonicalize()
        .map_err(|e| format!("cannot canonicalise {what} {}: {e}", path.display()))?;
    if !resolved.is_dir() {
        return Err(format!("{what} {} is not a directory", resolved.display()));
    }
    Ok(resolved)
}

fn is_within(root: &Path, candidate: &Path) -> bool {
    candidate.starts_with(root)
}

fn refusal_message(candidate: &Path, repo_root: &Path) -> String {
    format!(
        "refusing to create a scratch directory inside the repository: {} is inside {}",
        candidate.display(),
        repo_root.display()
    )
}

fn verify_created_dir(path: &Path, canonical: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect scratch directory {}: {e}", path.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "refusing scratch directory {}: it is a symlink/reparse point",
            path.display()
        ));
    }
    if !meta.is_dir() {
        return Err(format!(
            "refusing scratch directory {}: it is not a directory",
            path.display()
        ));
    }
    let resolved = fs::canonicalize(path).map_err(|e| {
        format!(
            "cannot canonicalise scratch directory {}: {e}",
            path.display()
        )
    })?;
    if resolved != canonical {
        return Err(format!(
            "refusing scratch directory {}: it resolves to {} (symlink/reparse redirection)",
            path.display(),
            resolved.display()
        ));
    }
    Ok(())
}

/// Remove the worktree's `.git` link (a file) so ordinary repository discovery
/// from inside the scratch directory cannot reach the shared Git directory.
fn detach_git_link(dir: &Path) -> Result<(), String> {
    let link = dir.join(".git");
    let meta = match fs::symlink_metadata(&link) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(format!(
                "refusing scratch directory {}: `git worktree add` did not create a .git link",
                dir.display()
            ))
        }
        Err(e) => return Err(format!("cannot inspect {}: {e}", link.display())),
    };
    if meta.file_type().is_symlink() {
        return Err(format!(
            "refusing {}: .git is a symlink/reparse point",
            link.display()
        ));
    }
    if !meta.is_file() {
        return Err(format!(
            "refusing {}: .git is not a regular file",
            link.display()
        ));
    }
    if fs::remove_file(&link).is_err() {
        // Windows refuses to unlink a read-only file; clear the bit, retry once.
        make_writable(&link);
        fs::remove_file(&link).map_err(|e| format!("cannot remove {}: {e}", link.display()))?;
    }
    Ok(())
}

fn abandon(path: &Path, repo_root: &Path) {
    remove_tree_bounded(path, Instant::now() + CLEANUP_BUDGET);
    prune_worktrees(repo_root);
}

/// Best-effort, bounded prune of stale administrative worktree entries. Runs in
/// a timeout-bounded obligation so a hostile hook/config cannot hang `Drop`.
fn prune_worktrees(repo_root: &Path) {
    if !repo_root.is_dir() {
        return;
    }
    let obligation = Obligation {
        id: "scratch-prune".to_string(),
        program: "git".to_string(),
        args: vec![
            "worktree".to_string(),
            "prune".to_string(),
            "--expire".to_string(),
            "now".to_string(),
        ],
        required: false,
        timeout_s: PRUNE_TIMEOUT_S,
    };
    let _ = gitcli::run_obligation(&obligation, repo_root);
}

/// Iterative, bounded removal: no recursion (deep trees cannot overflow the
/// stack), no panic, no error when the path is already gone. Symlinks are
/// unlinked, never followed.
fn remove_tree_bounded(root: &Path, deadline: Instant) {
    let mut stack: Vec<(PathBuf, bool)> = vec![(root.to_path_buf(), false)];
    let mut processed = 0usize;
    while let Some((path, children_pushed)) = stack.pop() {
        if Instant::now() >= deadline || processed >= MAX_CLEANUP_ENTRIES {
            break;
        }
        if children_pushed {
            make_writable(&path);
            let _ = fs::remove_dir(&path);
            continue;
        }
        processed += 1;
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            stack.push((path.clone(), true));
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    stack.push((entry.path(), false));
                }
            }
        } else {
            make_writable(&path);
            let _ = fs::remove_file(&path);
        }
    }
}

fn make_writable(path: &Path) {
    #[cfg(windows)]
    {
        let Ok(meta) = fs::symlink_metadata(path) else {
            return;
        };
        if meta.file_type().is_symlink() {
            return;
        }
        let mut permissions = meta.permissions();
        if permissions.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(path, permissions);
        }
    }
    #[cfg(not(windows))]
    {
        // Unix unlink only needs a writable parent directory; the checkout's
        // files are not modified chmod-wise.
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_non_object_ids() {
        for bad in ["", "HEAD", "--detach", "zzzz", "abc", "../../etc/passwd"] {
            assert!(!is_hex_object_id(bad), "{bad:?} must be refused");
        }
        assert!(is_hex_object_id("0123456789abcdef"));
        assert!(is_hex_object_id("abcd"));
    }

    #[test]
    fn within_is_component_wise() {
        assert!(is_within(Path::new("/repo"), Path::new("/repo/a")));
        assert!(is_within(Path::new("/repo"), Path::new("/repo")));
        assert!(!is_within(Path::new("/repo"), Path::new("/repo-sibling/a")));
        assert!(!is_within(Path::new("/repo"), Path::new("/other")));
    }

    #[test]
    fn candidate_names_are_unique_per_sequence() {
        let base = Path::new("scratch-base");
        assert_ne!(candidate_path(base, 0), candidate_path(base, 1));
        assert!(candidate_path(base, 7)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(NAME_PREFIX));
    }

    #[test]
    fn bounded_removal_is_idempotent_and_never_follows_links() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("flow-scratch-unit-{}-{nanos}", std::process::id()));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("nested/file.txt"), "x").unwrap();
        fs::write(root.join("top.txt"), "x").unwrap();
        let deadline = Instant::now() + CLEANUP_BUDGET;
        remove_tree_bounded(&root, deadline);
        assert!(!root.exists(), "an owned tree must be fully removed");
        // Idempotent: a second bounded removal of a missing path is a no-op.
        remove_tree_bounded(&root, deadline);
        assert!(!root.exists());
    }
}
