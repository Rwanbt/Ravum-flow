//! End-to-end CLI tests on disposable repositories.
//!
//! These tests exercise the shipped binary exactly as a user would. They are the
//! evidence for the "flow run/publish" claims; a skipped test is never reported
//! as a pass (see the printed SKIP lines and the report).

#[path = "../src/json.rs"]
#[allow(dead_code)]
mod json;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use json::Json;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Repo {
    path: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = remove_tree(&self.path);
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
    // Intentional: a test fixture directory must be deletable, so read-only bits
    // set by Git on its object files are cleared before removing the tree.
    #[allow(clippy::permissions_set_readonly_false)]
    fn clear(p: &Path) {
        if let Ok(meta) = fs::symlink_metadata(p) {
            let perms = meta.permissions();
            if perms.readonly() {
                let mut writable = perms.clone();
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

fn new_repo() -> Repo {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("flow-it-{}-{}", std::process::id(), n));
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Test"]);
    git(&path, &["config", "user.email", "flow@example.invalid"]);
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

fn flow(repo: &Path, args: &[&str]) -> (i32, Json) {
    flow_with_env(repo, args, &[])
}

fn flow_with_env(repo: &Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, Json) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flow"));
    cmd.current_dir(repo).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("flow runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    // A simulated crash exits before printing: that is expected, not a parse failure.
    let parsed = if text.trim().is_empty() {
        Json::Null
    } else {
        json::parse(&text).unwrap_or_else(|e| {
            panic!(
                "flow {args:?} printed non-JSON ({e}):\n{text}\n{}",
                String::from_utf8_lossy(&out.stderr)
            )
        })
    };
    (out.status.code().unwrap_or(-1), parsed)
}

fn status_of(value: &Json) -> String {
    value
        .get("status")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

fn write_config(repo: &Path, obligations: &str) {
    let dir = repo.join(".flow");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.json"),
        format!(
            r#"{{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{obligations}],"disabled":false}}"#
        ),
    )
    .unwrap();
}

fn sha256_of(path: &Path) -> String {
    #[path = "../src/sha256.rs"]
    mod sha256;
    sha256::sha256_hex(&fs::read(path).unwrap())
}

// ------------------------------------------------------------------- tests
#[test]
fn init_creates_state_and_refuses_second_call() {
    let repo = new_repo();
    let (code, out) = flow(&repo.path, &["init"]);
    assert_eq!(code, 0);
    assert_eq!(status_of(&out), "INITIALIZED");
    assert!(repo.path.join(".flow").join("config.json").is_file());

    let (code2, out2) = flow(&repo.path, &["init"]);
    assert_eq!(code2, 1, "a second init without --force must be refused");
    assert_eq!(status_of(&out2), "REFUSED");
}

#[test]
fn doctor_is_read_only_and_reports_guarantees() {
    let repo = new_repo();
    let before = git(&repo.path, &["status", "--porcelain=v1"]);
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 0);
    assert_eq!(status_of(&out), "REPORT");
    assert!(out
        .get("checks")
        .and_then(Json::as_arr)
        .map(|a| !a.is_empty())
        .unwrap_or(false));
    assert!(out.get("non_guarantees").and_then(Json::as_arr).is_some());
    let after = git(&repo.path, &["status", "--porcelain=v1"]);
    assert_eq!(before, after, "doctor must not modify the repository");
}

#[test]
fn run_without_obligations_is_unknown_never_passed() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1);
    assert_eq!(status_of(&out), "UNKNOWN");
    assert!(out.get("reason").and_then(Json::as_str).is_some());
}

#[test]
fn run_passing_obligation_binds_the_index_tree() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("b.txt"), "beta\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    let index_tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "expected PASSED, got {out}");
    assert_eq!(status_of(&out), "PASSED");
    assert_eq!(
        out.get("tree_oid").and_then(Json::as_str),
        Some(index_tree.as_str())
    );
    assert!(repo
        .path
        .join(".flow")
        .join("receipts")
        .join(format!("{index_tree}.json"))
        .is_file());
    assert!(out
        .get("context_id")
        .and_then(Json::as_str)
        .map(|s| s.len() == 64)
        .unwrap_or(false));
}

#[test]
fn run_failing_obligation_is_failed_and_publish_refuses() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"needs-git","program":"git","args":["rev-parse","HEAD"],"required":true,"timeout_s":30}"#,
    );
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1);
    assert_eq!(status_of(&out), "FAILED");

    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(code2, 1);
    assert_eq!(status_of(&out2), "REFUSED");
}

#[test]
fn publish_without_receipt_is_refused() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 1);
    assert_eq!(status_of(&out), "REFUSED");
    assert!(out
        .get("reason")
        .and_then(Json::as_str)
        .unwrap_or("")
        .contains("no receipt"));
}

#[test]
fn publish_moves_the_ref_with_cas_and_records_the_journal() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("c.txt"), "gamma\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    let tree = git(&repo.path, &["write-tree"]);
    let old = git(&repo.path, &["rev-parse", "HEAD"]);
    flow(&repo.path, &["run"]);

    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 0, "expected PUBLISHED, got {out}");
    assert_eq!(status_of(&out), "PUBLISHED");
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    assert_ne!(head, old);
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD^{tree}"]), tree);
    assert_eq!(out.get("tree").and_then(Json::as_str), Some(tree.as_str()));

    let journal = fs::read_to_string(repo.path.join(".flow").join("journal.jsonl")).unwrap();
    assert!(journal.contains("intent"));
    assert!(journal.contains("PUBLISHED"));
}

#[test]
fn publish_with_stale_expected_old_is_refused() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("d.txt"), "delta\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    flow(&repo.path, &["run"]);
    let head_before = git(&repo.path, &["rev-parse", "HEAD"]);

    let (code, out) = flow(&repo.path, &["publish", "--expected-old", &"0".repeat(40)]);
    assert_eq!(code, 1);
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "nothing may move"
    );
}

#[test]
fn crash_after_intent_is_not_published_and_reconciles() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("e.txt"), "epsilon\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    flow(&repo.path, &["run"]);
    let head_before = git(&repo.path, &["rev-parse", "HEAD"]);

    let (code, _) = flow_with_env(
        &repo.path,
        &["publish"],
        &[("FLOW_TEST_CRASH", "after-intent")],
    );
    assert_eq!(code, 89, "the simulated crash must not look like success");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);

    let (code2, out2) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code2, 0);
    let resolved = out2
        .get("resolved")
        .and_then(Json::as_arr)
        .expect("resolved list");
    assert!(!resolved.is_empty(), "pending intent must be reported");
    assert_eq!(
        resolved[0].get("resolution").and_then(Json::as_str),
        Some("NOT_PUBLISHED")
    );
}

#[test]
fn crash_after_ref_is_observed_as_published() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("f.txt"), "zeta\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    flow(&repo.path, &["run"]);
    let head_before = git(&repo.path, &["rev-parse", "HEAD"]);

    let (code, _) = flow_with_env(
        &repo.path,
        &["publish"],
        &[("FLOW_TEST_CRASH", "after-ref")],
    );
    assert_eq!(code, 89);
    assert_ne!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "the ref was published before the simulated crash"
    );

    let (_, out) = flow(&repo.path, &["reconcile"]);
    let resolved = out
        .get("resolved")
        .and_then(Json::as_arr)
        .expect("resolved list");
    assert_eq!(
        resolved[0].get("resolution").and_then(Json::as_str),
        Some("PUBLISHED"),
        "reconcile must observe the published ref instead of retrying blindly"
    );
}

#[test]
fn dirty_and_untracked_files_are_preserved_byte_for_byte() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("a.txt"), "dirty human edit\n").unwrap();
    fs::write(repo.path.join("untracked.txt"), "human untracked\n").unwrap();
    let dirty_hash = sha256_of(&repo.path.join("a.txt"));
    let untracked_hash = sha256_of(&repo.path.join("untracked.txt"));

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);
    assert_eq!(sha256_of(&repo.path.join("a.txt")), dirty_hash);
    assert_eq!(sha256_of(&repo.path.join("untracked.txt")), untracked_hash);
    let diff = out
        .get("worktree_diff")
        .and_then(Json::as_arr)
        .expect("worktree_diff");
    assert!(
        diff.iter().any(|v| v.as_str() == Some("a.txt")),
        "the receipt must expose that the worktree diverges from the verified tree"
    );
}

#[test]
fn disable_blocks_run_and_publish() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    let (code, out) = flow(&repo.path, &["disable"]);
    assert_eq!(code, 0);
    assert_eq!(status_of(&out), "DISABLED");
    let (code2, out2) = flow(&repo.path, &["run"]);
    assert_eq!(code2, 1);
    assert_eq!(status_of(&out2), "REFUSED");
    let (code3, _) = flow(&repo.path, &["enable"]);
    assert_eq!(code3, 0);
}

#[test]
fn unsafe_obligation_id_is_refused() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"../../escape","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1);
    assert_eq!(status_of(&out), "REFUSED");
}

#[test]
fn obligation_timeout_is_timeout_not_pass() {
    if !program_exists("python") {
        eprintln!("SKIP obligation_timeout_is_timeout_not_pass: python not available");
        return;
    }
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"slow","program":"python","args":["-c","import time; time.sleep(5)"],"required":true,"timeout_s":1}"#,
    );
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1);
    assert_eq!(status_of(&out), "TIMEOUT");
}

fn program_exists(name: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .any(|dir| dir.join(name).is_file() || dir.join(format!("{name}.exe")).is_file())
}
