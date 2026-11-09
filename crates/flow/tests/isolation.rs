//! Isolation tests: scratch directory safety, bounded obligation output, real
//! timeout containment, and Git environment hijack resistance.
//!
//! The crate is a binary without a library target, so the sibling modules are
//! imported with the same `#[path]` trick used by `tests/cli.rs`. Everything
//! here is deterministic on Windows without elevation; anything that cannot be
//! exercised prints an explicit `SKIP <name>: <reason>` line and never reports
//! a silent success.
//!
//! The "child probe" tests below are helpers: in a normal `cargo test` run they
//! return immediately; the parent test spawns the test binary itself with
//! `--exact <probe>` plus a marker argument, which the probe detects through
//! `std::env::args()` (an obligation's environment is scrubbed, so an
//! environment variable could not be used to signal it).

#[path = "../src/gitcli.rs"]
#[allow(dead_code)]
mod gitcli;
#[path = "../src/scratch.rs"]
#[allow(dead_code)]
mod scratch;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use gitcli::{Git, Obligation};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

// --------------------------------------------------------------- fixtures
struct Repo {
    path: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = remove_tree(&self.path);
    }
}

/// Remove a test fixture, clearing the read-only bits Git may set.
fn remove_tree(path: &Path) -> std::io::Result<()> {
    #[allow(clippy::permissions_set_readonly_false)]
    fn clear(p: &Path) {
        if let Ok(meta) = fs::symlink_metadata(p) {
            let permissions = meta.permissions();
            if permissions.readonly() {
                let mut writable = permissions.clone();
                writable.set_readonly(false);
                let _ = fs::set_permissions(p, writable);
            }
        }
        if p.is_dir() {
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    clear(&entry.path());
                }
            }
        }
    }
    clear(path);
    fs::remove_dir_all(path)
}

fn new_repo(tag: &str) -> Repo {
    let path = std::env::temp_dir().join(format!(
        "flow-iso-{tag}-{}-{}",
        std::process::id(),
        next_id()
    ));
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Isolation Test"]);
    git(
        &path,
        &["config", "user.email", "flow-isolation@example.invalid"],
    );
    git(&path, &["config", "commit.gpgsign", "false"]);
    fs::write(path.join("a.txt"), "alpha\n").unwrap();
    git(&path, &["add", "-A"]);
    git(&path, &["commit", "-q", "-m", "seed"]);
    Repo { path }
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ----------------------------------------------------------- child probes
const ENV_PROBE_ARG: &str = "FLOW_ISOLATION_ENV_PROBE_CHILD";
const FLOOD_ARG: &str = "FLOW_ISOLATION_FLOOD_CHILD";
const TIMEOUT_ARG: &str = "FLOW_ISOLATION_TIMEOUT_CHILD";
const GRANDCHILD_ARG: &str = "FLOW_ISOLATION_GRANDCHILD_CHILD";
const HOLD_ARG: &str = "FLOW_ISOLATION_HOLD_PIPE_CHILD";

fn arg_present(marker: &str) -> bool {
    std::env::args().any(|arg| arg == marker)
}

fn probe_command(test: &str, marker: &str) -> Command {
    let exe = std::env::current_exe().expect("test executable");
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", test, "--nocapture", marker]);
    cmd
}

fn probe_obligation(id: &str, test: &str, marker: &str, timeout_s: u64) -> Obligation {
    let exe = std::env::current_exe().expect("test executable");
    Obligation {
        id: id.to_string(),
        program: exe.to_string_lossy().into_owned(),
        args: vec![
            "--exact".to_string(),
            test.to_string(),
            "--nocapture".to_string(),
            marker.to_string(),
        ],
        required: true,
        timeout_s,
    }
}

/// Writes more than the per-stream cap. The obligation must drain (not block)
/// the child, retain a bounded prefix and flag truncation.
#[test]
fn flood_child_probe() {
    if !arg_present(FLOOD_ARG) {
        return;
    }
    const FLOOD_BYTES: usize = 3 * 1024 * 1024;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let chunk = [b'x'; 64 * 1024];
    let mut written = 0;
    while written < FLOOD_BYTES {
        out.write_all(&chunk).expect("flood write");
        written += chunk.len();
    }
    out.flush().expect("flood flush");
}

/// Sleeps far longer than the configured timeout; must be killed.
#[test]
fn timeout_child_probe() {
    if !arg_present(TIMEOUT_ARG) {
        return;
    }
    std::thread::sleep(Duration::from_secs(120));
}

/// Holds the inherited obligation pipes open after its parent exited. 15 s is
/// well beyond the runner's reader grace, so a runner that joined its reader
/// threads could not return within the bounded assertion below.
#[test]
fn hold_pipe_child_probe() {
    if !arg_present(HOLD_ARG) {
        return;
    }
    std::thread::sleep(Duration::from_secs(15));
}

/// Spawns `hold_pipe_child_probe`, which inherits this process's stdout/stderr
/// (the obligation pipes) and outlives it, then exits immediately.
#[test]
fn grandchild_child_probe() {
    if !arg_present(GRANDCHILD_ARG) {
        return;
    }
    let mut cmd = Command::new(std::env::current_exe().expect("test executable"));
    cmd.args(["--exact", "hold_pipe_child_probe", "--nocapture", HOLD_ARG]);
    // Intentionally not waited on: the grandchild must outlive this process and
    // keep the inherited obligation pipes open, which is exactly the case the
    // parent test exercises. It self-terminates after a few seconds.
    #[allow(clippy::zombie_processes)]
    let _grandchild = cmd.spawn().expect("grandchild spawns");
}

/// Runs in a child process whose environment carries hostile `GIT_*` values.
#[test]
fn git_env_probe_child() {
    if std::env::var_os("FLOW_ISOLATION_PROBE").is_none() {
        return;
    }
    let repo =
        PathBuf::from(std::env::var_os("FLOW_ISOLATION_PROBE_REPO").expect("probe repo env"));
    let expected_tree = std::env::var("FLOW_ISOLATION_PROBE_TREE").expect("probe tree env");
    let expected_head = std::env::var("FLOW_ISOLATION_PROBE_HEAD").expect("probe head env");
    let expected_root =
        PathBuf::from(std::env::var_os("FLOW_ISOLATION_PROBE_ROOT").expect("probe root env"));

    let git = Git::new(&repo);
    assert_eq!(
        git.write_tree().expect("write_tree must not be redirected"),
        expected_tree,
        "GIT_INDEX_FILE/GIT_DIR redirected write_tree"
    );
    assert_eq!(
        git.capture(&["rev-parse", "HEAD"])
            .expect("rev-parse HEAD must not be redirected"),
        expected_head,
        "GIT_DIR redirected rev-parse HEAD"
    );
    let root = gitcli::toplevel(&repo).expect("toplevel must not be redirected");
    assert_eq!(root, expected_root, "GIT_WORK_TREE redirected toplevel");
    let reported = git
        .capture(&["rev-parse", "--show-toplevel"])
        .expect("rev-parse --show-toplevel must not be redirected");
    assert_eq!(
        Path::new(&reported)
            .canonicalize()
            .expect("canonical toplevel"),
        expected_root,
        "GIT_DIR redirected rev-parse --show-toplevel"
    );
}

// ------------------------------------------------------- scratch directory
#[test]
fn scratch_materialises_head_outside_repo_and_cleans_up() {
    let _guard = scratch::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let repo = new_repo("scratch-basic");
    let git = Git::new(&repo.path);
    let head = git.head_oid().expect("head oid");

    let first = scratch::materialise(&git, &head, &repo.path).expect("materialise head");
    let second = scratch::materialise(&git, &head, &repo.path).expect("materialise head again");

    assert_ne!(
        first.path(),
        second.path(),
        "scratch directories must be unique"
    );
    assert!(first.path().is_dir() && second.path().is_dir());
    assert!(
        !first.path().starts_with(&repo.path),
        "scratch must live outside the repository: {}",
        first.path().display()
    );
    assert_eq!(
        fs::read_to_string(first.path().join("a.txt")).unwrap(),
        "alpha\n",
        "the commit's tree must be materialised"
    );
    assert!(
        !first.path().join(".git").exists(),
        "the .git link must be removed so the shared gitdir is unreachable"
    );

    // Ordinary repository discovery from the scratch directory must fail.
    let probe = Command::new("git")
        .arg("-C")
        .arg(first.path())
        .args(["rev-parse", "--show-toplevel"])
        .env("GIT_CEILING_DIRECTORIES", first.path())
        .output()
        .expect("git probe runs");
    assert!(
        !probe.status.success(),
        "a scratch directory must not be a discoverable work tree: {}",
        String::from_utf8_lossy(&probe.stdout)
    );

    let first_path = first.path().to_path_buf();
    let second_path = second.path().to_path_buf();
    drop(first);
    drop(second);
    assert!(
        !first_path.exists(),
        "Drop must remove the scratch directory"
    );
    assert!(
        !second_path.exists(),
        "Drop must remove the scratch directory"
    );
    assert_eq!(
        git.status_porcelain().unwrap(),
        Vec::<String>::new(),
        "materialising and dropping must leave the repository unchanged"
    );
}

#[test]
fn scratch_never_deletes_a_preexisting_path() {
    let _guard = scratch::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let repo = new_repo("scratch-preexisting");
    let git = Git::new(&repo.path);
    let head = git.head_oid().expect("head oid");
    let base = std::env::temp_dir().join(format!(
        "flow-scratch-base-{}-{}",
        std::process::id(),
        next_id()
    ));
    fs::create_dir_all(&base).expect("scratch base");

    // Pre-create the exact paths the next few attempts would prefer (another
    // test may consume sequence numbers concurrently, so cover a small range).
    let first = scratch::next_sequence();
    let mut taken = Vec::new();
    for offset in 1..=4 {
        let path = scratch::candidate_path(&base, first + offset);
        match fs::create_dir(&path) {
            Ok(()) => {
                fs::write(path.join("sentinel.txt"), "keep me").expect("sentinel");
                taken.push(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => panic!("cannot pre-create {}: {e}", path.display()),
        }
    }
    assert!(!taken.is_empty(), "no candidate path could be pre-created");

    let scratch = scratch::materialise_in(&base, &git, &head, &repo.path)
        .expect("materialise must retry around occupied names");
    let used = fs::canonicalize(scratch.path()).expect("scratch canonical");
    for path in &taken {
        assert_ne!(
            used,
            fs::canonicalize(path).expect("taken canonical"),
            "materialise must not adopt a pre-existing path"
        );
        assert!(
            path.join("sentinel.txt").is_file(),
            "a pre-existing path must not be deleted while looking for a free name"
        );
    }

    let used_display = scratch.path().to_path_buf();
    drop(scratch);
    assert!(
        !used_display.exists(),
        "the owned scratch directory is removed"
    );
    for path in &taken {
        assert!(
            path.join("sentinel.txt").is_file(),
            "Drop must never delete a pre-existing path"
        );
    }
    let _ = remove_tree(&base);
}

#[test]
fn scratch_refuses_paths_inside_the_repository() {
    let _guard = scratch::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let repo = new_repo("scratch-inside");
    let git = Git::new(&repo.path);
    let head = git.head_oid().expect("head oid");

    // (a) a base inside the repository is refused before anything is created.
    let inside_base = repo.path.join("scratch-base");
    fs::create_dir(&inside_base).expect("inside base");
    let err = scratch::materialise_in(&inside_base, &git, &head, &repo.path)
        .expect_err("a base inside the repository must be refused");
    assert!(err.contains("inside the repository"), "{err}");
    assert_eq!(
        fs::read_dir(&inside_base).unwrap().count(),
        0,
        "a refusal must not create or remove anything"
    );

    // (b) the public entry point refuses when the temp dir resolves inside the
    // supplied repository root (the same check, not a different code path).
    let err = scratch::materialise(&git, &head, &std::env::temp_dir())
        .expect_err("a scratch path under the repo root must be refused");
    assert!(err.contains("inside the repository"), "{err}");
}

#[test]
fn scratch_refuses_non_object_ids() {
    let _guard = scratch::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let repo = new_repo("scratch-oid");
    let git = Git::new(&repo.path);
    for bad in ["", "HEAD", "--detach", "../../etc/passwd"] {
        let err = scratch::materialise(&git, bad, &repo.path)
            .expect_err("a non-OID commit must be refused before Git runs");
        assert!(err.contains("object id"), "{bad:?}: {err}");
    }
}

#[test]
fn scratch_refuses_a_base_that_redirects_into_the_repository() {
    let _guard = scratch::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let repo = new_repo("scratch-link");
    let git = Git::new(&repo.path);
    let head = git.head_oid().expect("head oid");
    let target = repo.path.join("redirect-target");
    fs::create_dir(&target).expect("link target");
    let link = std::env::temp_dir().join(format!(
        "flow-scratch-link-{}-{}",
        std::process::id(),
        next_id()
    ));
    if let Err(e) = make_dir_link(&target, &link) {
        eprintln!("SKIP scratch_refuses_a_base_that_redirects_into_the_repository: cannot create a directory link ({e})");
        return;
    }
    let err = scratch::materialise_in(&link, &git, &head, &repo.path)
        .expect_err("a redirecting base must be refused");
    assert!(err.contains("inside the repository"), "{err}");
    assert!(link.exists(), "the refused link itself must not be deleted");
    let _ = fs::remove_dir(&link);
}

#[cfg(windows)]
fn make_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
    // A junction does not require elevation (a symlink usually does), which
    // keeps this test deterministic on Windows 11 without admin rights.
    let out = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "mklink /J failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )))
    }
}

#[cfg(unix)]
fn make_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

// ----------------------------------------------------- output and timeouts
#[test]
fn obligation_output_is_bounded_and_drained() {
    let obligation = probe_obligation("flood", "flood_child_probe", FLOOD_ARG, 20);
    let started = Instant::now();
    let outcome = gitcli::run_obligation(&obligation, Path::new("."));
    let elapsed = started.elapsed();

    assert_eq!(
        outcome.status, "PASSED",
        "a flood must be drained, not deadlock the child: {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "flood containment took {elapsed:?}"
    );
    assert!(
        outcome.stdout_truncated,
        "3 MiB on stdout must set the truncation flag"
    );
    assert!(
        outcome.stdout.len() <= gitcli::MAX_STREAM_BYTES + gitcli::TRUNCATION_MARKER.len(),
        "stdout must stay bounded, got {} bytes",
        outcome.stdout.len()
    );
    assert!(
        outcome.stdout.contains("output truncated"),
        "the bounded capture must carry the truncation marker"
    );
    eprintln!(
        "MEASURED flood: stdout_bytes={} stdout_truncated={} stderr_truncated={} elapsed_ms={}",
        outcome.stdout.len(),
        outcome.stdout_truncated,
        outcome.stderr_truncated,
        elapsed.as_millis()
    );
}

#[test]
fn timeout_kills_the_child_and_returns_in_bounded_time() {
    let obligation = probe_obligation("timeout", "timeout_child_probe", TIMEOUT_ARG, 1);
    let started = Instant::now();
    let outcome = gitcli::run_obligation(&obligation, Path::new("."));
    let elapsed = started.elapsed();

    assert!(outcome.timed_out, "a 1 s timeout must be reported");
    assert_eq!(outcome.status, "TIMEOUT");
    assert!(
        elapsed < Duration::from_secs(30),
        "timeout containment took {elapsed:?} for a 1 s timeout"
    );
    eprintln!(
        "MEASURED timeout: elapsed_ms={} status={} exit={:?}",
        elapsed.as_millis(),
        outcome.status,
        outcome.exit
    );
}

/// A child that exits immediately but leaves a grandchild holding the stdout
/// and stderr pipes open must not hang the runner: the reader threads are
/// abandoned after a bounded grace period instead of being joined.
#[test]
fn surviving_grandchild_does_not_hang_the_runner() {
    let obligation = probe_obligation("grandchild", "grandchild_child_probe", GRANDCHILD_ARG, 60);
    let started = Instant::now();
    let outcome = gitcli::run_obligation(&obligation, Path::new("."));
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(8),
        "the runner waited for the surviving grandchild instead of detaching its reader: {elapsed:?}"
    );
    assert_eq!(
        outcome.status, "PASSED",
        "the immediate child exited successfully: {outcome:?}"
    );
    assert!(!outcome.timed_out, "the immediate child did not time out");
    eprintln!(
        "MEASURED grandchild: elapsed_ms={} status={} stdout_bytes={}",
        elapsed.as_millis(),
        outcome.status,
        outcome.stdout.len()
    );
}

// ---------------------------------------------------- environment hijacking
#[test]
fn git_ignores_hijacked_repository_environment() {
    let victim = new_repo("hijack-victim");
    let decoy = new_repo("hijack-decoy");
    // A staged change makes the victim's index tree distinctive.
    fs::write(victim.path.join("a.txt"), "victim staged\n").unwrap();
    git(&victim.path, &["add", "-A"]);
    let expected_tree = git(&victim.path, &["write-tree"]);
    let expected_head = git(&victim.path, &["rev-parse", "HEAD"]);
    let expected_root = victim.path.canonicalize().expect("canonical victim");

    let decoy_git = decoy.path.join(".git");
    let decoy_index = decoy_git.join("index");

    // Control: prove the hijack variables are effective for an unscrubbed git.
    // Without this, a passing probe could be vacuous.
    let control = Command::new("git")
        .arg("-C")
        .arg(&victim.path)
        .arg("write-tree")
        .env("GIT_DIR", &decoy_git)
        .env("GIT_WORK_TREE", &decoy.path)
        .env("GIT_INDEX_FILE", &decoy_index)
        .output()
        .expect("control git runs");
    let control_tree = String::from_utf8_lossy(&control.stdout).trim().to_string();
    assert!(
        !control.status.success() || control_tree != expected_tree,
        "control failed: an unscrubbed git was not redirected (tree {control_tree:?})"
    );

    let mut cmd = probe_command("git_env_probe_child", ENV_PROBE_ARG);
    cmd.env("FLOW_ISOLATION_PROBE", "1")
        .env("FLOW_ISOLATION_PROBE_REPO", &victim.path)
        .env("FLOW_ISOLATION_PROBE_TREE", &expected_tree)
        .env("FLOW_ISOLATION_PROBE_HEAD", &expected_head)
        .env("FLOW_ISOLATION_PROBE_ROOT", &expected_root)
        .env("GIT_DIR", &decoy_git)
        .env("GIT_WORK_TREE", &decoy.path)
        .env("GIT_INDEX_FILE", &decoy_index)
        .env("GIT_OBJECT_DIRECTORY", decoy_git.join("objects"))
        .env(
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            decoy_git.join("objects"),
        )
        .env("GIT_COMMON_DIR", &decoy_git)
        .env("GIT_NAMESPACE", "hijack")
        .env("GIT_CONFIG", decoy.path.join("hostile-config"))
        .env(
            "GIT_CONFIG_GLOBAL",
            decoy.path.join("hostile-global.gitconfig"),
        )
        .env(
            "GIT_CONFIG_SYSTEM",
            decoy.path.join("hostile-system.gitconfig"),
        )
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", decoy.path.join("hooks"))
        .env("GIT_CEILING_DIRECTORIES", &victim.path)
        .env("GIT_DISCOVERY_ACROSS_FILESYSTEM", "1")
        .env("GIT_ASKPASS", decoy.path.join("askpass"))
        .env("SSH_ASKPASS", decoy.path.join("askpass"))
        .env("GIT_SSH", decoy.path.join("ssh"))
        .env("GIT_SSH_COMMAND", decoy.path.join("ssh"))
        .env("GIT_EXTERNAL_DIFF", decoy.path.join("diff"))
        .env("GIT_PAGER", "false")
        .env("GIT_EDITOR", "false")
        .env("GIT_SEQUENCE_EDITOR", "false")
        .env("GIT_PROTOCOL_FROM_USER", "0")
        .env("GIT_ALLOW_PROTOCOL", "file");

    let out = cmd.output().expect("environment probe child runs");
    assert!(
        out.status.success(),
        "Git plumbing was redirected by the host environment.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
