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

    // The tests below pin the ownership guard: `Drop` deletes a directory only
    // when it is still the real directory this value created, still resolves to
    // the identity recorded at creation, and still carries the marker written
    // after materialisation. Everything else is left untouched, because the
    // alternative is deleting a user's directory after a redirect.

    fn unit_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("flow-scratch-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn scratch_for(path: &Path, repo_root: &Path) -> Scratch {
        Scratch {
            path: path.to_path_buf(),
            canonical: fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
            repo_root: repo_root.to_path_buf(),
        }
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
    fn cleanup_leaves_a_directory_it_did_not_mark_as_owned() {
        let root = unit_root("unowned");
        let tree = root.join("tree");
        fs::create_dir(&tree).unwrap();
        fs::write(tree.join("pre-existing.txt"), "keep").unwrap();
        drop(scratch_for(&tree, &root.join("gone-repo")));
        assert!(
            tree.join("pre-existing.txt").is_file(),
            "a directory without the ownership marker is never removed"
        );
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn cleanup_leaves_a_path_that_is_no_longer_the_scratch_directory() {
        let root = unit_root("not-owned");
        // A file where the scratch directory was: never removed.
        let file = root.join("tree");
        fs::write(&file, "x").unwrap();
        drop(scratch_for(&file, &root));
        assert!(file.is_file(), "a file is not a scratch directory");

        // A marker that is itself a directory does not prove ownership.
        let marked = root.join("marked");
        fs::create_dir_all(marked.join(OWNED_MARKER)).unwrap();
        fs::write(marked.join("keep.txt"), "keep").unwrap();
        drop(scratch_for(&marked, &root));
        assert!(
            marked.join("keep.txt").is_file(),
            "an unowned tree survives"
        );
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn cleanup_removes_a_directory_it_created_and_marked() {
        let root = unit_root("owned");
        let tree = root.join("tree");
        fs::create_dir(&tree).unwrap();
        fs::write(tree.join(OWNED_MARKER), "owned\n").unwrap();
        fs::write(tree.join("file.txt"), "x").unwrap();
        // A repository root that no longer exists: the prune is skipped, the
        // removal still happens.
        drop(scratch_for(&tree, &root.join("gone-repo")));
        assert!(!tree.exists(), "an owned, marked tree is removed");
        assert!(root.exists(), "only the scratch tree is removed");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn cleanup_never_traverses_a_redirect_that_replaced_the_scratch() {
        let root = unit_root("redirect");
        let outside = unit_root("redirect-out");
        fs::write(outside.join("keep.txt"), "keep").unwrap();
        let link = root.join("tree");
        make_dir_link(&outside, &link).expect(
            "this gate proves that a redirect is never traversed, so the host must allow one",
        );
        drop(scratch_for(&link, &root));
        assert!(
            outside.join("keep.txt").is_file(),
            "content behind a redirect must survive cleanup"
        );
        let _ = fs::remove_dir(&link);
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
        remove_tree_bounded(&outside, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn a_redirect_that_carries_the_marker_is_never_traversed() {
        // The marker alone does not prove ownership: a redirect to a directory
        // that happens to carry one must still be refused, or cleanup would
        // delete another tree through the link.
        let root = unit_root("redirect-marker");
        let outside = unit_root("redirect-marker-out");
        fs::write(outside.join(OWNED_MARKER), "owned\n").unwrap();
        fs::write(outside.join("keep.txt"), "keep").unwrap();
        let link = root.join("tree");
        if let Err(e) = make_dir_link(&outside, &link) {
            eprintln!("SKIP a redirect that carries the marker: cannot create a link ({e})");
            remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
            remove_tree_bounded(&outside, Instant::now() + CLEANUP_BUDGET);
            return;
        }
        let scratch = Scratch {
            path: link.clone(),
            canonical: fs::canonicalize(&outside).unwrap(),
            repo_root: root.clone(),
        };
        assert!(
            !scratch.still_owned(),
            "a redirect is not the directory that was created, even with a marker"
        );
        drop(scratch);
        assert!(
            outside.join("keep.txt").is_file(),
            "content behind a redirect must survive cleanup"
        );
        let _ = fs::remove_dir(&link);
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
        remove_tree_bounded(&outside, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn verify_created_dir_refuses_a_missing_path_and_a_file() {
        let root = unit_root("verify");
        let missing = root.join("missing");
        let reason = verify_created_dir(&missing, &missing)
            .expect_err("a vanished scratch directory is a refusal");
        assert!(reason.contains("cannot inspect"), "{reason}");

        let file = root.join("file");
        fs::write(&file, "x").unwrap();
        let reason =
            verify_created_dir(&file, &file).expect_err("a file is not a scratch directory");
        assert!(reason.contains("not a directory"), "{reason}");

        let dir = root.join("dir");
        fs::create_dir(&dir).unwrap();
        verify_created_dir(&dir, &fs::canonicalize(&dir).unwrap())
            .expect("a real directory at the recorded identity is accepted");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn detach_git_link_refuses_anything_but_the_worktree_link() {
        let root = unit_root("detach");
        let missing = root.join("no-git");
        fs::create_dir(&missing).unwrap();
        let reason =
            detach_git_link(&missing).expect_err("a checkout without a .git link is refused");
        assert!(reason.contains("did not create a .git link"), "{reason}");

        let dir_git = root.join("dir-git");
        fs::create_dir_all(dir_git.join(".git")).unwrap();
        let reason =
            detach_git_link(&dir_git).expect_err("a .git directory is not the worktree link");
        assert!(reason.contains("not a regular file"), "{reason}");

        let ok = root.join("ok");
        fs::create_dir(&ok).unwrap();
        fs::write(ok.join(".git"), "gitdir: ../x\n").unwrap();
        #[cfg(windows)]
        {
            // Windows refuses to unlink a read-only file; the retry must clear
            // the bit rather than fail the run.
            let mut permissions = fs::metadata(ok.join(".git")).unwrap().permissions();
            permissions.set_readonly(true);
            fs::set_permissions(ok.join(".git"), permissions).unwrap();
        }
        detach_git_link(&ok).expect("a regular .git link is removed");
        assert!(!ok.join(".git").exists());
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn bounded_removal_stops_at_its_deadline_and_tolerates_a_missing_path() {
        let root = unit_root("deadline");
        let tree = root.join("tree");
        fs::create_dir_all(tree.join("nested")).unwrap();
        fs::write(tree.join("nested/file.txt"), "x").unwrap();
        // An expired budget leaves the tree alone instead of running long.
        remove_tree_bounded(&tree, Instant::now() - Duration::from_secs(1));
        assert!(
            tree.join("nested/file.txt").is_file(),
            "an expired budget removes nothing"
        );
        // A path that is already gone is a no-op, never an error.
        make_writable(&root.join("absent"));
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
        assert!(!root.exists());
    }

    // The tests below reach the creation-side refusals: `materialise_in` is the
    // only place a scratch directory is created, and every branch there decides
    // whether a *new* directory is created or an existing one is left alone.

    #[test]
    fn a_vanished_scratch_directory_is_never_claimed() {
        let root = unit_root("vanished");
        let gone = root.join("gone");
        let scratch = scratch_for(&gone, &root);
        assert!(
            !scratch.still_owned(),
            "a path that no longer exists is not owned"
        );
        drop(scratch);
        assert!(!gone.exists(), "nothing is created by dropping");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn materialise_refuses_a_non_object_id_before_touching_the_filesystem() {
        let root = unit_root("materialise-id");
        let git = Git::new(&root);
        for bad in ["HEAD", "--detach", "abc", ""] {
            let err = materialise_in(&root, &git, bad, &root)
                .expect_err("a non-object-id commit must be refused");
            assert!(
                err.contains("expected a hexadecimal object id"),
                "{bad:?}: {err}"
            );
        }
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn materialise_refuses_a_base_or_a_candidate_inside_the_repository() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = unit_root("materialise-inside");
        let repo = root.join("repo");
        fs::create_dir(&repo).unwrap();
        let git = Git::new(&repo);

        // The base *is* the repository: refused before any name is tried.
        let err = materialise_in(&repo, &git, "abcd", &repo)
            .expect_err("a scratch base inside the repository is refused");
        assert!(err.contains("inside the repository"), "{err}");

        let base = root.join("base");
        fs::create_dir(&base).unwrap();
        let base = fs::canonicalize(&base).unwrap();

        // The base is outside, but the candidate name the next attempt will use
        // resolves exactly onto the repository root, so creating it would write
        // inside the repository. The name sequence is process-global and the suite
        // runs tests in parallel: another test can consume the value between the
        // read here and the read inside `materialise_in`, which turns the collision
        // into an ordinary missing object. Retry with a fresh value a bounded
        // number of times rather than depend on the scheduler; the guard under test
        // must still refuse, and a missing guard fails every attempt.
        let mut refused = None;
        for _ in 0..8 {
            let seq = next_sequence();
            let candidate = candidate_path(&base, seq + 1);
            if fs::create_dir(&candidate).is_err() {
                continue;
            }
            match materialise_in(&base, &git, "abcd", &candidate) {
                Err(reason) if reason.contains("inside the repository") => {
                    assert!(candidate.is_dir(), "the refused candidate is left alone");
                    refused = Some(reason);
                    break;
                }
                Err(reason) if reason.contains("cannot materialise") => continue,
                other => panic!("expected an inside-the-repository refusal, got {other:?}"),
            }
        }
        let reason = refused.expect("the candidate-inside guard must refuse within 8 attempts");
        assert!(reason.contains("inside the repository"), "{reason}");

        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn canonical_dir_refuses_a_path_that_is_not_a_directory() {
        let root = unit_root("canonical-file");
        let file = root.join("file");
        fs::write(&file, "x").unwrap();
        let err = canonical_dir(&file, "scratch base").expect_err("a file is not a base");
        assert!(err.contains("is not a directory"), "{err}");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn verify_created_dir_refuses_a_redirect_and_a_different_identity() {
        let root = unit_root("verify-redirect");
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let link = root.join("link");
        if let Err(e) = make_dir_link(&target, &link) {
            eprintln!("SKIP verify_created_dir refuses a redirect: cannot create a link ({e})");
            remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
            return;
        }
        let canonical_target = fs::canonicalize(&target).unwrap();
        let recorded_elsewhere = root.join("elsewhere");
        fs::create_dir(&recorded_elsewhere).unwrap();
        let reason = verify_created_dir(&link, &fs::canonicalize(&recorded_elsewhere).unwrap())
            .expect_err("a redirect to another identity is refused");
        assert!(
            reason.contains("symlink/reparse point") || reason.contains("resolves to"),
            "{reason}"
        );
        // Even when the recorded identity is the link's own target, a link is not
        // a directory Flow created, so the answer must never be a silent `Ok`
        // with the link traversed. On Windows a junction may be reported as a
        // plain directory by `is_symlink()`; the recorded-identity check then
        // accepts it, which is why `materialise` records the *created* path and
        // refuses before the link exists.
        let _ = verify_created_dir(&link, &canonical_target);
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn detach_git_link_refuses_a_link_and_clears_a_read_only_file() {
        let root = unit_root("detach-link");
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let linked = root.join("linked-git");
        fs::create_dir(&linked).unwrap();
        if let Err(e) = make_dir_link(&target, &linked.join(".git")) {
            eprintln!("SKIP detach_git_link refuses a link: cannot create a link ({e})");
            remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
            return;
        }
        let reason = detach_git_link(&linked).expect_err("a .git link is not a worktree link");
        assert!(
            reason.contains("symlink/reparse point") || reason.contains("not a regular file"),
            "{reason}"
        );

        #[cfg(windows)]
        {
            let read_only = root.join("read-only-git");
            fs::create_dir(&read_only).unwrap();
            let link = read_only.join(".git");
            fs::write(&link, "gitdir: ../x\n").unwrap();
            let mut permissions = fs::metadata(&link).unwrap().permissions();
            permissions.set_readonly(true);
            fs::set_permissions(&link, permissions).unwrap();
            // The fallback branch: the first unlink fails on Windows, the bit is
            // cleared, and the second attempt succeeds.
            detach_git_link(&read_only).expect("a read-only .git link is removed");
            assert!(!link.exists());
        }
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn an_owned_directory_that_resolves_elsewhere_is_not_claimed() {
        // The recorded identity is what proves ownership: a real directory whose
        // canonical path no longer matches it is left alone.
        let root = unit_root("moved");
        let tree = root.join("tree");
        fs::create_dir(&tree).unwrap();
        fs::write(tree.join(OWNED_MARKER), "owned\n").unwrap();
        fs::write(tree.join("keep.txt"), "keep").unwrap();
        let elsewhere = root.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        let scratch = Scratch {
            path: tree.clone(),
            canonical: fs::canonicalize(&elsewhere).unwrap(),
            repo_root: root.clone(),
        };
        assert!(!scratch.still_owned(), "a different identity is not owned");
        drop(scratch);
        assert!(
            tree.join("keep.txt").is_file(),
            "cleanup must not remove a directory owned by someone else"
        );
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn materialise_gives_up_after_the_bounded_number_of_collisions() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = unit_root("collisions");
        let base = root.join("base");
        fs::create_dir(&base).unwrap();
        let base = fs::canonicalize(&base).unwrap();
        let repo = root.join("repo");
        fs::create_dir(&repo).unwrap();
        let git = Git::new(&repo);
        // Same shared-sequence caveat as
        // `materialise_refuses_a_base_or_a_candidate_inside_the_repository`: occupy
        // the 128 names the *next* attempt will try, but re-read the sequence and
        // retry when a parallel test consumed a value in between (the attempt then
        // lands on a free name and fails for another reason).
        let mut refused = None;
        for _ in 0..8 {
            let seq = next_sequence();
            // Occupy exactly the 128 names the next `materialise_in` will try: the
            // bound must fail closed instead of looping forever.
            for offset in 1..=MAX_ATTEMPTS {
                if let Err(e) = fs::create_dir(candidate_path(&base, seq + offset)) {
                    assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{e}");
                }
            }
            match materialise_in(&base, &git, "abcd", &repo) {
                Err(reason) if reason.contains("after 128 attempts") => {
                    refused = Some(reason);
                    break;
                }
                Err(reason) if reason.contains("cannot materialise") => continue,
                other => panic!("expected the retry budget to be exhausted, got {other:?}"),
            }
        }
        let reason = refused.expect("the retry budget must be exhausted within 8 attempts");
        assert!(reason.contains("after 128 attempts"), "{reason}");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn verify_created_dir_refuses_a_recorded_identity_that_differs() {
        let root = unit_root("verify-identity");
        let dir = root.join("dir");
        fs::create_dir(&dir).unwrap();
        let other = root.join("other");
        fs::create_dir(&other).unwrap();
        let reason = verify_created_dir(&dir, &fs::canonicalize(&other).unwrap())
            .expect_err("a directory that resolves elsewhere is not the created one");
        assert!(reason.contains("resolves to"), "{reason}");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn cleanup_clears_a_read_only_file_before_removing_it() {
        let root = unit_root("readonly-cleanup");
        let tree = root.join("tree");
        fs::create_dir(&tree).unwrap();
        let file = tree.join("locked.txt");
        fs::write(&file, "x").unwrap();
        let mut permissions = fs::metadata(&file).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&file, permissions).unwrap();
        remove_tree_bounded(&tree, Instant::now() + CLEANUP_BUDGET);
        assert!(
            !tree.exists(),
            "a tree with a read-only file is still removed"
        );
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn a_scratch_base_that_is_a_file_is_refused_before_anything_is_created() {
        // The base is canonicalised before the creation loop, so a file can never
        // become a scratch parent: the refusal names the path, and nothing is
        // created, adopted or removed. This also pins that the guard runs before
        // the uniqueness loop rather than after a failed `create_dir`.
        let repo = unit_root("base-file-repo");
        let elsewhere = unit_root("base-file-out");
        let base = elsewhere.join("base-is-a-file");
        fs::write(&base, "not a directory").unwrap();
        let git = Git::new(&repo);
        let err = materialise_in(&base, &git, "abcd", &repo)
            .expect_err("a file is not a usable scratch base");
        assert!(err.contains("not a directory"), "{err}");
        assert_eq!(
            fs::read_dir(&elsewhere).unwrap().count(),
            1,
            "a refused base must not gain a scratch directory"
        );
        remove_tree_bounded(&repo, Instant::now() + CLEANUP_BUDGET);
        remove_tree_bounded(&elsewhere, Instant::now() + CLEANUP_BUDGET);
    }

    #[test]
    fn detach_git_link_refuses_a_tree_without_a_git_link() {
        // `git worktree add` always creates the link; a tree without it means the
        // command silently did something else, so the materialisation must fail
        // closed rather than hand back a directory that still reaches the shared
        // Git directory.
        let root = unit_root("detach-missing");
        let dir = root.join("tree");
        fs::create_dir(&dir).unwrap();
        let err = detach_git_link(&dir).expect_err("a tree without a .git link is refused");
        assert!(err.contains("did not create a .git link"), "{err}");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }

    #[cfg(windows)]
    #[test]
    fn detach_git_link_clears_a_read_only_git_file() {
        // Windows refuses to unlink a read-only file. The `.git` link is written
        // by another process, so its attributes are not ours to assume; the
        // removal has to clear the bit and retry once instead of failing and
        // leaving the scratch directory attached.
        let root = unit_root("detach-readonly");
        let dir = root.join("tree");
        fs::create_dir(&dir).unwrap();
        let link = dir.join(".git");
        fs::write(&link, "gitdir: ../.git/worktrees/scratch\n").unwrap();
        let mut permissions = fs::metadata(&link).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&link, permissions).unwrap();
        detach_git_link(&dir).expect("a read-only .git file must be cleared and removed");
        assert!(!link.exists(), "the link must be gone");
        remove_tree_bounded(&root, Instant::now() + CLEANUP_BUDGET);
    }
}
