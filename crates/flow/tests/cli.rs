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

/// Write a policy **and adopt it**, exactly as an operator would: `.flow/config.json`
/// is a claim, and nothing in it runs until this working copy approved its digest
/// (`flow approve`). Tests that deliberately skip the approval call use
/// [`write_config_unapproved`] instead.
fn write_config(repo: &Path, obligations: &str) {
    write_config_unapproved(repo, obligations);
    let (code, out) = flow(repo, &["approve"]);
    assert_eq!(
        code, 0,
        "the policy must be approvable before it is used: {out}"
    );
    assert_eq!(status_of(&out), "APPROVED");
}

fn write_config_unapproved(repo: &Path, obligations: &str) {
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

/// Create a file symlink, returning the platform error unchanged.
fn symlink_file(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link)
    }
}

/// A gate must not turn "the host could not create the fixture" into a green
/// `... ok`. On Unix a symlink needs no privilege, so the failure is fatal; on
/// Windows it needs `SeCreateSymbolicLinkPrivilege`, so the test says so loudly
/// and the CI job does not claim that gate on Windows.
#[cfg(unix)]
fn link_unavailable(gate: &str, error: &std::io::Error) -> bool {
    panic!("{gate} is a required gate and must run: cannot create a symlink ({error})")
}

#[cfg(windows)]
fn link_unavailable(gate: &str, error: &std::io::Error) -> bool {
    println!(
        "SKIP {gate}: this host cannot create a file symlink without SeCreateSymbolicLinkPrivilege ({error})"
    );
    true
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
    // The receipt shows the obligation's own output: a preview that is empty or
    // that truncates without a mark would hide the evidence an operator needs.
    let obligations = out
        .get("obligations")
        .and_then(Json::as_arr)
        .expect("the receipt lists the executed obligations");
    assert_eq!(obligations.len(), 1, "{out}");
    let preview_text = obligations[0]
        .get("stdout_preview")
        .and_then(Json::as_str)
        .expect("the receipt carries the obligation's stdout preview");
    assert!(
        preview_text.starts_with("git version "),
        "the preview must show the obligation's own stdout: {preview_text:?}"
    );
    assert!(
        preview_text.chars().count() <= 401,
        "the preview keeps the documented 400 characters plus an ellipsis: {preview_text:?}"
    );
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

/// The crash-injection hook is compiled out of release builds by design; CI
/// proves the released binary ignores it. A release-mode run reports these
/// tests as skipped instead of failing on the missing hook.
fn skip_without_debug_hook(name: &str) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    println!(
        "SKIP {name}: FLOW_TEST_CRASH is inert in release builds (CI proves the released binary ignores it)"
    );
    true
}

#[test]
fn crash_after_intent_is_not_published_and_reconciles() {
    if skip_without_debug_hook("crash_after_intent_is_not_published_and_reconciles") {
        return;
    }
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
    if skip_without_debug_hook("crash_after_ref_is_observed_as_published") {
        return;
    }
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
    write_config_unapproved(
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
        // A skipped gate is not a pass: the CI job requires this test to run.
        panic!("this gate requires `python` on PATH; installing it is a prerequisite, not a skip");
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

// =================================================================== P0 gates
//
// The tests below reproduce the three blocking defects recorded in
// docs/reviews/FLOW-FOUNDATION-P0-HARDENING-REPORT.md. They are written against
// the observable CLI contract only (JSON fields + exit codes), so the *same*
// test body runs unchanged on the defective baseline and on the corrected
// build: baseline FAIL is the reproduction, fixed PASS is the proof.
//
//   P0-A  a partial execution was recorded as a complete PASSED proof
//   P0-B  a receipt stayed valid after the verification policy changed
//   P0-C  an unreadable journal was reported as a successful reconciliation
//
// A test here is a *gate*: if it fails, publication must not be considered safe.

const GREEN: &str =
    r#"{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}"#;
const RED: &str = r#"{"id":"red","program":"git","args":["rev-parse","--verify","refs/heads/definitely-absent"],"required":true,"timeout_s":30}"#;

/// Flow's own state (`config`, `journal`, `receipts`) must never enter the
/// verified tree: otherwise every policy change moves the tree OID and the
/// staleness scenarios cannot even be expressed. `flow init` grows the same
/// exclusion in `.flow/.gitignore`; the tests set it on the fixture directly so
/// that the baseline (which lacks that fix) is exercised identically.
fn exclude_flow(repo: &Path) {
    let info = repo.join(".git").join("info");
    fs::create_dir_all(&info).unwrap();
    let path = info.join("exclude");
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    if !text.contains(".flow/") {
        text.push_str(".flow/\n");
    }
    fs::write(path, text).unwrap();
}

fn receipt_file(repo: &Path, tree: &str) -> PathBuf {
    repo.join(".flow")
        .join("receipts")
        .join(format!("{tree}.json"))
}

fn read_json_file(path: &Path) -> Json {
    json::parse(&fs::read_to_string(path).unwrap()).unwrap()
}

fn write_json_file(path: &Path, value: &Json) {
    fs::write(path, value.to_string_pretty() + "\n").unwrap();
}

/// Replace the top-level `status` value, preserving every other field — the
/// cheapest realistic forgery of a receipt by someone with write access.
fn with_status(value: &Json, status: &str) -> Json {
    let mut out = value.clone();
    if let Json::Obj(pairs) = &mut out {
        if let Some(slot) = pairs.iter_mut().find(|(k, _)| k == "status") {
            slot.1 = Json::str(status);
        }
    }
    out
}

fn journal_file(repo: &Path) -> PathBuf {
    repo.join(".flow").join("journal.jsonl")
}

/// Create a repository whose index tree is already committed and stable, with
/// `.flow/` excluded, ready for a run/publish scenario.
///
/// A real content change is staged so the index tree differs from `HEAD`'s: a
/// verified tree that is already the ref's tree has nothing to publish, and
/// `publish` says so (`ALREADY_PUBLISHED`).
fn prepared_repo(obligations: &str) -> (Repo, String, String) {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, obligations);
    fs::write(
        repo.path.join("verified.txt"),
        "content under verification\n",
    )
    .unwrap();
    git(&repo.path, &["add", "-A"]);
    let tree = git(&repo.path, &["write-tree"]);
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    (repo, tree, head)
}

// ------------------------------------------------------------------ P0-A
#[test]
fn p0a_selective_run_never_yields_a_publishable_receipt() {
    let (repo, tree, head_before) = prepared_repo(&format!("{GREEN},{RED}"));

    let (code, out) = flow(&repo.path, &["run", "--obligation", "green"]);
    assert_eq!(
        code, 1,
        "a run that executed only part of the required policy must not exit 0: {out}"
    );
    assert_ne!(
        status_of(&out),
        "PASSED",
        "a selective run must never claim the whole policy passed: {out}"
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "a partial execution must not occupy the complete-receipt slot {tree}.json"
    );
    assert_ne!(
        out.get("code").and_then(Json::as_str),
        Some("DUPLICATE_OBLIGATION_ID"),
        "one id selected once is a valid selection, refused for a partial run: {out}"
    );

    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code2, 1,
        "publish must refuse after a selective run: {out2}"
    );
    assert_eq!(status_of(&out2), "REFUSED");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "nothing may be published when a required obligation was never executed"
    );
}

#[test]
fn p0a_duplicate_or_unknown_obligation_ids_are_refused() {
    let (repo, _, _) = prepared_repo(&format!("{GREEN},{RED}"));

    let (code, out) = flow(
        &repo.path,
        &["run", "--obligation", "green", "--obligation", "green"],
    );
    assert_eq!(
        code, 1,
        "a duplicated id is ambiguous and must be refused: {out}"
    );
    assert_eq!(status_of(&out), "REFUSED");

    let (code2, out2) = flow(&repo.path, &["run", "--obligation", "nope"]);
    assert_eq!(code2, 1);
    assert_eq!(status_of(&out2), "REFUSED");
}

#[test]
fn p0a_full_run_with_one_failing_required_obligation_is_never_publishable() {
    let (repo, _, head_before) = prepared_repo(&format!("{GREEN},{RED}"));
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1);
    assert_eq!(status_of(&out), "FAILED");
    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(code2, 1);
    assert_eq!(status_of(&out2), "REFUSED");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

#[test]
fn p0a_an_optional_failing_obligation_does_not_mask_a_green_required_set() {
    let optional_red = RED.replace("\"required\":true", "\"required\":false");
    let (repo, _, _) = prepared_repo(&format!("{GREEN},{optional_red}"));
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 0,
        "a failing optional obligation is traced but does not block: {out}"
    );
    assert_eq!(status_of(&out), "PASSED");
    let listed = out
        .get("obligations")
        .and_then(Json::as_arr)
        .expect("obligations");
    assert!(
        listed
            .iter()
            .any(|o| o.get("id").and_then(Json::as_str) == Some("red")
                && o.get("required").and_then(Json::as_bool) == Some(false)),
        "the optional obligation must be visible with required=false: {out}"
    );
}

// ------------------------------------------------------------------ P0-B
#[test]
fn p0b_adding_a_required_obligation_makes_the_receipt_stale() {
    let (repo, tree, head_before) = prepared_repo(GREEN);
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "the single green obligation must pass: {out}");
    assert!(receipt_file(&repo.path, &tree).exists());

    // The policy now demands one more required obligation for the SAME tree.
    write_config(&repo.path, &format!("{GREEN},{RED}"));

    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code2, 1,
        "a receipt produced under a weaker policy is not publishable: {out2}"
    );
    assert_eq!(status_of(&out2), "REFUSED");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "a stale receipt must never move the ref"
    );
}

#[test]
fn p0b_changing_a_timeout_makes_the_receipt_stale() {
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    // Same command, same tree, different deadline: the verification policy the
    // receipt proves is not the policy that now applies.
    write_config(
        &repo.path,
        r#"{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":1}"#,
    );

    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(code2, 1, "timeout is part of the policy identity: {out2}");
    assert_eq!(status_of(&out2), "REFUSED");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

#[test]
fn p0b_changing_the_policy_version_makes_the_receipt_stale() {
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    let dir = repo.path.join(".flow");
    let path = dir.join("config.json");
    let mut config = read_json_file(&path);
    if let Json::Obj(pairs) = &mut config {
        if let Some(slot) = pairs.iter_mut().find(|(k, _)| k == "policy_version") {
            slot.1 = Json::str("FLOW-POLICY-2");
        }
    }
    write_json_file(&path, &config);

    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(code2, 1, "policy_version is part of the identity: {out2}");
    assert_eq!(status_of(&out2), "REFUSED");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

#[test]
fn p0b_a_tampered_receipt_is_not_publishable() {
    let (repo, tree, head_before) = prepared_repo(RED);
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "the required obligation fails: {out}");
    assert_eq!(status_of(&out), "FAILED");

    let path = receipt_file(&repo.path, &tree);
    assert!(path.exists(), "a failed run still records its receipt");
    let receipt = read_json_file(&path);
    write_json_file(&path, &with_status(&receipt, "PASSED"));

    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code2, 1,
        "editing a receipt on disk must not turn FAILED into publishable: {out2}"
    );
    assert_eq!(status_of(&out2), "REFUSED");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

#[test]
fn p0b_explicit_ref_must_name_a_branch() {
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    let (code2, out2) = flow(&repo.path, &["publish", "--ref", "HEAD"]);
    assert_eq!(code2, 1, "an ambiguous ref must be refused: {out2}");
    assert_eq!(status_of(&out2), "REFUSED");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);

    let (code3, out3) = flow(&repo.path, &["publish", "--ref", "refs/tags/not-a-branch"]);
    assert_eq!(code3, 1);
    assert_eq!(status_of(&out3), "REFUSED");
}

/// A symbolic ref moves its target while the CLI would report the name the
/// operator passed. Publishing must refuse instead of naming the wrong branch.
#[test]
fn p1_a_symbolic_ref_is_never_a_publication_target() {
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");
    git(
        &repo.path,
        &["symbolic-ref", "refs/heads/alias", "refs/heads/main"],
    );
    let (code, out) = flow(
        &repo.path,
        &[
            "publish",
            "--ref",
            "refs/heads/alias",
            "--expected-old",
            &head_before,
        ],
    );
    assert_eq!(code, 1, "a symbolic ref must not be published: {out}");
    assert_eq!(
        out.get("code").and_then(Json::as_str),
        Some("REF_IS_SYMREF")
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "refs/heads/main"]),
        head_before,
        "the target branch must not move"
    );
}

// ------------------------------------------------------------------ P0-C
#[test]
fn p0c_corrupt_journal_is_not_reported_as_reconciled() {
    if skip_without_debug_hook("p0c_corrupt_journal_is_not_reported_as_reconciled") {
        return;
    }
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    let (crash, _) = flow_with_env(
        &repo.path,
        &["publish"],
        &[("FLOW_TEST_CRASH", "after-intent")],
    );
    assert_eq!(crash, 89, "the simulated crash must not look like success");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);

    let path = journal_file(&repo.path);
    let mut corrupted = fs::read(&path).unwrap();
    corrupted.extend_from_slice(b"{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"tr");
    fs::write(&path, &corrupted).unwrap();
    let on_disk = fs::read(&path).unwrap();

    let (code2, out2) = flow(&repo.path, &["reconcile"]);
    assert_ne!(
        code2, 0,
        "an unreadable journal cannot be reconciled successfully: {out2}"
    );
    let status = status_of(&out2);
    assert!(
        matches!(
            status.as_str(),
            "CORRUPT_JOURNAL" | "UNKNOWN_OUTCOME" | "MANUAL_REQUIRED"
        ),
        "expected a corruption state, got {status}: {out2}"
    );
    assert_eq!(
        out2.get("manual_required").and_then(Json::as_bool),
        Some(true),
        "the operator must be told a human decision is required: {out2}"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        on_disk,
        "the raw journal must be preserved for inspection"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "reconcile must never move a ref"
    );
}

#[test]
fn p0c_duplicate_intent_id_requires_a_human() {
    let (repo, tree, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);
    let other = git(
        &repo.path,
        &["commit-tree", &tree, "-p", &head_before, "-m", "x"],
    );
    let record = format!(
        "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"dup-1\",\"ref\":\"refs/heads/main\",\"old\":\"{head_before}\",\"new\":\"{other}\",\"tree\":\"{tree}\",\"at\":\"2026-11-09T00:00:00Z\"}}\n"
    );
    fs::write(journal_file(&repo.path), format!("{record}{record}")).unwrap();

    let (code2, out2) = flow(&repo.path, &["reconcile"]);
    assert_ne!(code2, 0, "two intents sharing one id are ambiguous: {out2}");
    assert_eq!(status_of(&out2), "NEEDS_RECONCILIATION");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

#[test]
fn p0c_ref_moved_elsewhere_needs_reconciliation() {
    if skip_without_debug_hook("p0c_ref_moved_elsewhere_needs_reconciliation") {
        return;
    }
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    let (crash, _) = flow_with_env(
        &repo.path,
        &["publish"],
        &[("FLOW_TEST_CRASH", "after-intent")],
    );
    assert_eq!(crash, 89);

    // Someone else moves the ref to a third commit: the intent is neither
    // observably applied nor observably absent.
    git(
        &repo.path,
        &["commit", "-q", "--allow-empty", "-m", "concurrent writer"],
    );
    let moved = git(&repo.path, &["rev-parse", "HEAD"]);
    assert_ne!(moved, head_before);

    let (code2, out2) = flow(&repo.path, &["reconcile"]);
    assert_ne!(code2, 0, "an ambiguous outcome is not a success: {out2}");
    assert_eq!(status_of(&out2), "NEEDS_RECONCILIATION");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        moved,
        "reconcile must not fight the concurrent writer"
    );
}

#[test]
fn p0c_reconcile_is_idempotent() {
    if skip_without_debug_hook("p0c_reconcile_is_idempotent") {
        return;
    }
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);
    let (crash, _) = flow_with_env(
        &repo.path,
        &["publish"],
        &[("FLOW_TEST_CRASH", "after-intent")],
    );
    assert_eq!(crash, 89);

    let (code1, out1) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code1, 0, "{out1}");
    let resolved = out1.get("resolved").and_then(Json::as_arr).unwrap_or(&[]);
    assert_eq!(resolved.len(), 1);

    let (code2, out2) = flow(&repo.path, &["reconcile"]);
    assert_eq!(
        code2, 0,
        "a second reconcile has nothing left to do: {out2}"
    );
    assert_eq!(
        out2.get("resolved")
            .and_then(Json::as_arr)
            .map(<[Json]>::len),
        Some(0)
    );
    // The journal must still be readable and contain exactly one outcome for
    // the intent: re-running must not append a second, contradictory outcome.
    let text = fs::read_to_string(journal_file(&repo.path)).unwrap();
    let outcomes = text
        .lines()
        .filter(|l| l.contains("\"event\":\"outcome\""))
        .count();
    assert_eq!(outcomes, 1, "idempotent reconcile, got:\n{text}");
}

#[test]
fn p0c_status_refuses_to_guess_on_a_corrupt_journal() {
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);
    fs::write(journal_file(&repo.path), "{not json at all\n").unwrap();

    let (code2, out2) = flow(&repo.path, &["status"]);
    assert_ne!(
        code2, 0,
        "a status report that silently ignores an unreadable journal is a lie: {out2}"
    );
}

// ------------------------------------------------- concurrent publication
#[test]
fn two_concurrent_publishes_publish_at_most_once() {
    let (repo, tree, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    let children: Vec<std::process::Child> = (0..2)
        .map(|_| {
            Command::new(env!("CARGO_BIN_EXE_flow"))
                .current_dir(&repo.path)
                .arg("publish")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("flow publish spawns")
        })
        .collect();
    let mut published = 0;
    for child in children {
        let out = child.wait_with_output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        if text.contains("\"PUBLISHED\"") {
            published += 1;
        }
    }
    assert_eq!(published, 1, "exactly one cooperative writer may win");

    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    assert_ne!(head, head_before);
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD^{tree}"]),
        tree,
        "the published commit must carry exactly the verified tree"
    );

    // The journal must still be parseable: interleaved appends are corruption.
    let (code3, out3) = flow(&repo.path, &["explain"]);
    assert_eq!(
        code3, 0,
        "journal must remain readable after a race: {out3}"
    );
    let (_, out4) = flow(&repo.path, &["reconcile"]);
    assert_eq!(
        out4.get("resolved")
            .and_then(Json::as_arr)
            .map(<[Json]>::len),
        Some(0),
        "no intent may be left pending after a successful publication"
    );
}

#[test]
fn p0b_a_timeout_override_is_never_publishable() {
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, out) = flow(&repo.path, &["run", "--timeout", "1"]);
    assert_eq!(
        code, 0,
        "the obligation still passes under a tighter deadline: {out}"
    );
    // The receipt proves the deadline that was applied (1 s), not the one the
    // configuration declares (30 s): publishing it would overstate the policy.
    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code2, 1,
        "an ad-hoc deadline does not prove the configured policy: {out2}"
    );
    assert_eq!(
        out2.get("code").and_then(Json::as_str),
        Some("STALE_POLICY")
    );
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

#[test]
fn p0b_an_ambiguous_config_is_refused_not_guessed() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    // `required` twice: two readers could disagree about the winning value, so
    // the policy is not a policy at all.
    fs::write(
        repo.path.join(".flow").join("config.json"),
        r#"{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{"id":"green","program":"git","args":["--version"],"required":true,"required":false,"timeout_s":30}],"disabled":false}"#,
    )
    .unwrap();
    let (code, out) = flow(&repo.path, &["run"]);
    assert_ne!(code, 0, "duplicate keys make the policy ambiguous: {out}");
    assert_eq!(status_of(&out), "ERROR");
}

// ------------------------------------------- materialisation fidelity (P0-1)
//
// `git worktree add` exits 0, with an empty stderr, on trees the host cannot
// reproduce. A required obligation then runs on content that is not the tree,
// while the receipt claims the tree was verified: a false PASSED with no
// tampering involved.

#[test]
fn p0a_case_colliding_tree_is_refused_not_certified() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    fs::write(repo.path.join("one.bin"), "ONE\n").unwrap();
    fs::write(repo.path.join("two.bin"), "TWO\n").unwrap();
    let one = git(&repo.path, &["hash-object", "-w", "one.bin"]);
    let two = git(&repo.path, &["hash-object", "-w", "two.bin"]);
    for (blob, path) in [(&one, "src/Case.txt"), (&two, "src/case.txt")] {
        git(
            &repo.path,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{blob},{path}"),
            ],
        );
    }
    let tree = git(&repo.path, &["write-tree"]);
    let names = git(&repo.path, &["ls-tree", "-r", "--name-only", &tree]);
    assert_eq!(
        names.lines().filter(|l| l.starts_with("src/")).count(),
        2,
        "the fixture must really carry a case collision: {names}"
    );

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 1,
        "a tree this host cannot reproduce must never be certified: {out}"
    );
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(
        out.get("code").and_then(Json::as_str),
        Some("UNSUPPORTED_CHECKOUT")
    );
    assert!(
        out.to_string().contains("differ only by case"),
        "the refusal must name the case collision, not a generic stat failure: {out}"
    );
    let (code2, out2) = flow(&repo.path, &["publish"]);
    assert_eq!(code2, 1, "and nothing may be published: {out2}");
}

#[test]
fn status_counts_tracked_changes_and_untracked_files_separately() {
    // The two counts answer different questions ("what changed under version
    // control" vs "what is not versioned at all"); folding them together makes a
    // receipt's worktree statement ambiguous.
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    fs::write(repo.path.join("tracked.txt"), "one\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    git(&repo.path, &["commit", "-q", "-m", "seed"]);

    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out.get("dirty_count").and_then(Json::as_num),
        Some(0.0),
        "{out}"
    );
    assert_eq!(
        out.get("untracked_count").and_then(Json::as_num),
        Some(0.0),
        "{out}"
    );

    fs::write(repo.path.join("untracked.txt"), "new\n").unwrap();
    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out.get("dirty_count").and_then(Json::as_num),
        Some(0.0),
        "an untracked file is not a tracked change: {out}"
    );
    assert_eq!(
        out.get("untracked_count").and_then(Json::as_num),
        Some(1.0),
        "{out}"
    );

    fs::write(repo.path.join("tracked.txt"), "two\n").unwrap();
    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out.get("dirty_count").and_then(Json::as_num),
        Some(1.0),
        "{out}"
    );
    assert_eq!(
        out.get("untracked_count").and_then(Json::as_num),
        Some(1.0),
        "{out}"
    );
}

#[test]
fn p0a_symlink_rewritten_as_a_file_is_refused() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    // With symlinks disabled, Git writes the link target as file content: a
    // transient, silent rewrite of the tree the obligations would inspect.
    git(&repo.path, &["config", "core.symlinks", "false"]);
    fs::write(repo.path.join("target.txt"), "SECRET\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    let blob = git(&repo.path, &["hash-object", "-w", "target.txt"]);
    git(
        &repo.path,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("120000,{blob},link"),
        ],
    );
    let tree = git(&repo.path, &["write-tree"]);
    let listing = git(&repo.path, &["ls-tree", "-r", &tree]);
    assert!(
        listing.contains("120000"),
        "the fixture must hold a symlink entry: {listing}"
    );

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 1,
        "a symlink checked out as a file is not the tree: {out}"
    );
    assert_eq!(
        out.get("code").and_then(Json::as_str),
        Some("UNSUPPORTED_CHECKOUT")
    );
}

// ------------------------------------------------ duplicate ids (P0-2)

#[test]
fn p0c_one_outcome_cannot_close_two_intents() {
    let (repo, tree, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    let other = git(
        &repo.path,
        &["commit-tree", &tree, "-p", &head_before, "-m", "x"],
    );
    let intent = format!(
        "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"dup-1\",\"ref\":\"refs/heads/main\",\"old\":\"{head_before}\",\"new\":\"{other}\",\"tree\":\"{tree}\",\"at\":\"2026-11-09T00:00:00Z\"}}\n"
    );
    let outcome = "{\"schema\":\"flow-journal/1\",\"event\":\"outcome\",\"id\":\"dup-1\",\"status\":\"PUBLISHED\",\"at\":\"2026-11-09T00:00:01Z\"}\n";
    fs::write(
        journal_file(&repo.path),
        format!("{intent}{intent}{outcome}"),
    )
    .unwrap();

    let (code2, out2) = flow(&repo.path, &["reconcile"]);
    assert_ne!(
        code2, 0,
        "one outcome must not be allowed to close two intents: {out2}"
    );
    assert_eq!(status_of(&out2), "NEEDS_RECONCILIATION");

    let (_, out3) = flow(&repo.path, &["status"]);
    assert_ne!(
        out3.get("pending_intents").and_then(Json::as_num),
        Some(0.0),
        "status must not report zero pending intents over an unresolved twin: {out3}"
    );
}

// ------------------------------------------------- adversarial regressions
/// A cloned repository can ship `.flow/journal.jsonl` as a symlink. Appending
/// through it would write outside the workspace: the journal write must refuse.
#[test]
fn a_symlinked_journal_is_never_written_through() {
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");

    let outside = std::env::temp_dir().join(format!(
        "flow-outside-journal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::write(&outside, "outside content\n").unwrap();
    let journal = repo.path.join(".flow").join("journal.jsonl");
    let _ = fs::remove_file(&journal);
    if let Err(e) = crate::symlink_file(&outside, &journal) {
        if link_unavailable("a_symlinked_journal_is_never_written_through", &e) {
            let _ = fs::remove_file(&outside);
            return;
        }
    }

    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code, 1,
        "a journal symlink must not be written through: {out}"
    );
    assert_eq!(
        fs::read_to_string(&outside).unwrap(),
        "outside content\n",
        "the outside file must be byte-identical"
    );
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
    let _ = fs::remove_file(&outside);
}

/// `core.autocrlf` / `core.eol` / `.gitattributes` make the checkout rewrite
/// bytes. An obligation asserting on those bytes would see content the tree does
/// not hold, so the checkout is refused instead of certified.
#[test]
fn a_converted_checkout_is_refused_not_certified() {
    let (repo, _, _) = prepared_repo(GREEN);
    fs::write(repo.path.join(".gitattributes"), "*.txt text eol=crlf\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 1,
        "a checkout that may convert content must be refused: {out}"
    );
    assert_eq!(
        out.get("code").and_then(Json::as_str),
        Some("UNSUPPORTED_CHECKOUT"),
        "{out}"
    );
}

/// Unknown options and value flags without a value are usage errors: silently
/// ignoring `--dry-run` would publish when the operator asked for a plan.
#[test]
fn unknown_and_valueless_options_are_usage_errors() {
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);

    let (code, out) = flow(&repo.path, &["publish", "--nonsense"]);
    assert_eq!(code, 2, "an unknown option must not be ignored: {out}");
    assert_eq!(status_of(&out), "ERROR");

    let (code, out) = flow(&repo.path, &["publish", "--ref"]);
    assert_eq!(
        code, 2,
        "a value flag without a value must be refused: {out}"
    );

    let (code, out) = flow(&repo.path, &["run", "--obligation"]);
    assert_eq!(
        code, 2,
        "a value flag without a value must be refused: {out}"
    );

    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "no usage error may move a ref"
    );
}

/// A boolean flag a command does not implement is a usage error, never a silently
/// ignored no-op: `flow run --dry-run` used to execute the declared obligations and
/// write a receipt for an operator who asked for a plan — the same defect class as
/// `publish --dry-run=true`, closed in the second independent review.
#[test]
fn a_boolean_flag_a_command_does_not_implement_is_a_usage_error() {
    let (repo, tree, head_before) = prepared_repo(GREEN);

    let (code, out) = flow(&repo.path, &["run", "--dry-run"]);
    assert_eq!(
        code, 2,
        "`run --dry-run` must not execute obligations and certify: {out}"
    );
    assert_eq!(
        status_of(&out),
        "ERROR",
        "an unimplemented option is a usage error, not a receipt: {out}"
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "a usage error must not leave the complete receipt slot {tree}.json"
    );

    let (code, out) = flow(&repo.path, &["status", "--force"]);
    assert_eq!(
        code, 2,
        "`status --force` must be refused, not ignored: {out}"
    );
    assert_eq!(status_of(&out), "ERROR", "{out}");

    let (code, out) = flow(&repo.path, &["doctor", "--adopt-policy"]);
    assert_eq!(
        code, 2,
        "`doctor --adopt-policy` must be refused, not ignored: {out}"
    );

    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "an unimplemented option may not move a ref"
    );
}

// ------------------------------------------------- production-candidate gates
/// `explain --tree` takes an object id, not a path: a traversal attempt must be
/// refused before any file is read.
#[test]
fn explain_refuses_a_path_instead_of_a_tree_id() {
    let (repo, _, _) = prepared_repo(GREEN);
    for bad in [
        "../../../etc/passwd",
        "/etc/passwd",
        "deadbeef",
        "..",
        "receipts/x",
    ] {
        let (code, out) = flow(&repo.path, &["explain", "--tree", bad]);
        assert_eq!(code, 1, "explain --tree {bad:?} must be refused: {out}");
        let text = out.to_string();
        assert!(
            text.contains("INVALID_TREE_ID") || text.contains("not a 40- or 64-character hex"),
            "the refusal must name the reason: {out}"
        );
    }
    let (code, out) = flow(&repo.path, &["proposals", "show", "../../../etc/passwd"]);
    assert_eq!(code, 1, "proposals show with a path must be refused: {out}");
    let (code, out) = flow(&repo.path, &["run", "--candidate", "../../x"]);
    assert_eq!(
        code, 1,
        "run --candidate with a path must be refused: {out}"
    );
}

/// A journal that cannot even be read is an explicit unknown outcome, never an
/// empty history that reconciliation could "close".
#[test]
fn reconcile_reports_an_unreadable_journal_instead_of_guessing() {
    let (repo, _, _) = prepared_repo(GREEN);
    let journal = journal_file(&repo.path);
    fs::remove_file(&journal).unwrap();
    fs::create_dir_all(&journal).unwrap();
    let (code, out) = flow(&repo.path, &["reconcile"]);
    assert_eq!(
        code, 1,
        "a refusal that needs a human exits non-zero: {out}"
    );
    assert_eq!(
        out.get("code").and_then(Json::as_str),
        Some("JOURNAL_UNREADABLE"),
        "{out}"
    );
    assert_eq!(
        out.get("manual_required").and_then(Json::as_bool),
        Some(true)
    );
    assert!(journal.is_dir(), "recovery must not rewrite the journal");
}

/// A Git call that answers with more than Flow's bounded stream can retain is a
/// failed observation, not a ref that does not exist: recovery must ask a human
/// instead of closing the intent as "nothing was published".
#[cfg(unix)]
#[test]
fn reconcile_reports_an_unreadable_ref_instead_of_calling_it_absent() {
    use std::os::unix::fs::PermissionsExt;
    let (repo, _, _) = prepared_repo(GREEN);
    let intent = r#"{"schema":"flow-journal/1","event":"intent","id":"intent-unreadable-ref","ref":"refs/heads/main","old":"0000000000000000000000000000000000000000","new":"1111111111111111111111111111111111111111"}"#;
    fs::write(journal_file(&repo.path), format!("{intent}\n")).unwrap();
    let git_binary = std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join("git"))
                .find(|candidate| candidate.is_file())
        })
        .expect("the test host has a git binary on PATH");
    let shim = repo.path.join("shim-bin");
    fs::create_dir_all(&shim).unwrap();
    let script = format!(
        "#!/bin/sh\nfor arg in \"$@\"; do\n  if [ \"$arg\" = \"--verify\" ]; then\n    exec head -c 70000000 /dev/zero\n  fi\ndone\nexec \"{}\" \"$@\"\n",
        git_binary.display()
    );
    let shim_git = shim.join("git");
    fs::write(&shim_git, script).unwrap();
    fs::set_permissions(&shim_git, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        shim.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(&repo.path)
        .arg("reconcile")
        .env("PATH", path)
        .output()
        .expect("flow runs");
    assert_eq!(
        out.status.code(),
        Some(1),
        "an unobservable ref needs a human: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = json::parse(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let resolved = out
        .get("resolved")
        .and_then(Json::as_arr)
        .map(|a| a.to_vec())
        .unwrap_or_else(|| panic!("no resolved entries: {out}"));
    assert_eq!(resolved.len(), 1, "{out}");
    assert!(
        resolved[0]
            .get("reason")
            .and_then(Json::as_str)
            .unwrap_or("")
            .starts_with("REF_UNREADABLE"),
        "the reason must name the unobservable ref and its cause: {out}"
    );
    assert_eq!(
        out.get("status").and_then(Json::as_str),
        Some("NEEDS_RECONCILIATION"),
        "{out}"
    );
}

/// The local host token is derived even when no environment variable names the
/// machine: the `hostname` utility and `/proc` are the documented fallbacks, and
/// a lock written by this machine must still be attributed to it.
#[cfg(unix)]
#[test]
fn host_identity_falls_back_to_the_platform_when_no_variable_names_it() {
    let (repo, _, _) = prepared_repo(GREEN);
    let name = fs::read_to_string("/proc/sys/kernel/hostname")
        .or_else(|_| {
            let out = Command::new("hostname").output()?;
            Ok::<String, std::io::Error>(String::from_utf8_lossy(&out.stdout).to_string())
        })
        .expect("the host has a name")
        .trim()
        .to_string();
    let material = repo.path.join("host-material.txt");
    fs::write(
        &material,
        format!("flow-host/1\n{name}\n{}", std::env::consts::OS),
    )
    .unwrap();
    let expected = sha256_of(&material)[..16].to_string();
    let lock = repo.path.join(".flow").join("lock");
    fs::write(
        &lock,
        format!(
            "flow-lock/2 {} 0 fallbacknonce publish host:{expected}\n",
            std::process::id()
        ),
    )
    .unwrap();

    // 1. No host variable at all: the `hostname` utility answers.
    let run = |path: Option<&Path>| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_flow"));
        cmd.current_dir(&repo.path).arg("doctor");
        cmd.env_remove("HOSTNAME").env_remove("COMPUTERNAME");
        if let Some(path) = path {
            cmd.env("PATH", path);
        }
        let out = cmd.output().expect("flow runs");
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        json::parse(&text).unwrap_or_else(|e| panic!("doctor printed non-JSON ({e}):\n{text}"))
    };
    let report = run(None);
    assert!(
        report.to_string().contains("publication_lock"),
        "doctor must report the lock: {report}"
    );
    assert!(
        report.to_string().contains("live"),
        "a lock naming this machine's fallback identity must be attributed to it: {report}"
    );

    // 2. PATH without the `hostname` utility: `/proc` answers instead. Only a
    //    directory holding a real `git` is exposed, so the command still works.
    let shim = repo.path.join("path-with-git-only");
    fs::create_dir_all(&shim).unwrap();
    let git_binary = std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join("git"))
                .find(|candidate| candidate.is_file())
        })
        .expect("the test host has a git binary on PATH");
    std::os::unix::fs::symlink(&git_binary, shim.join("git")).unwrap();
    let report = run(Some(&shim));
    // Linux reads `/proc/sys/kernel/hostname` when the utility is unavailable, so
    // the identity still matches. macOS has no such file: without the utility the
    // machine identity is simply unknown, and the honest answer is "foreign"
    // (never a guess), which is what this branch pins.
    #[cfg(target_os = "linux")]
    assert!(
        report.to_string().contains("live"),
        "the /proc fallback must produce the same identity: {report}"
    );
    #[cfg(not(target_os = "linux"))]
    assert!(
        report.to_string().contains("foreign"),
        "without `hostname` and without /proc, the identity is unknown, not guessed: {report}"
    );
}

/// A `git` that delegates to the real binary, except for the sub-commands named
/// by `FLOW_SHIM_MODE`. The same script (and the same PATH) is used for the
/// `run` and the `publish` of one scenario, so the recorded build context stays
/// identical and the fault is the only difference.
#[cfg(unix)]
fn shim_git(repo: &Path, tag: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let git_binary = std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join("git"))
                .find(|candidate| candidate.is_file())
        })
        .expect("the test host has a git binary on PATH");
    let dir = repo.join(format!("shim-{tag}"));
    fs::create_dir_all(&dir).unwrap();
    let counter = dir.join("show-ref-count");
    let script = format!(
        r#"#!/bin/sh
mode="${{FLOW_SHIM_MODE:-normal}}"
# Flow always invokes `git -C <root> <command> ...`: the command is $3.
if [ "$mode" = "update-ref-fail" ] && [ "$3" = "update-ref" ]; then
  echo "shim: ref update refused" >&2
  exit 1
fi
if [ "$mode" = "update-ref-noop" ] && [ "$3" = "update-ref" ]; then
  exit 0
fi
if [ "$mode" = "show-ref-moves" ] && [ "$3" = "rev-parse" ] && [ "$4" = "--verify" ] && [ "$5" = "--quiet" ]; then
  n=$(cat "{counter}" 2>/dev/null || echo 0)
  n=$((n+1))
  echo "$n" > "{counter}"
  if [ "$n" -ge 3 ]; then
    echo 1111111111111111111111111111111111111111
    exit 0
  fi
fi
exec "{git}" "$@"
"#,
        counter = counter.display(),
        git = git_binary.display()
    );
    let path = dir.join("git");
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

#[cfg(unix)]
fn flow_with_path(repo: &Path, shim: &Path, args: &[&str], mode: &str) -> (i32, Json) {
    let path = format!(
        "{}:{}",
        shim.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(repo)
        .args(args)
        .env("PATH", path)
        .env("FLOW_SHIM_MODE", mode)
        .output()
        .expect("flow runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed = if text.trim().is_empty() {
        Json::Null
    } else {
        json::parse(&text).unwrap_or_else(|e| panic!("non-JSON output ({e}):\n{text}"))
    };
    (out.status.code().unwrap_or(-1), parsed)
}

#[cfg(unix)]
fn code_field(value: &Json) -> String {
    value
        .get("code")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

/// A refused `git update-ref` is reported by name, and the ref is untouched.
#[cfg(unix)]
#[test]
fn publish_reports_an_update_ref_refusal_without_moving_the_ref() {
    let (repo, _, _) = prepared_repo(GREEN);
    let shim = shim_git(&repo.path, "update-ref-fails");
    let (code, run) = flow_with_path(&repo.path, &shim, &["run"], "normal");
    assert_eq!(code, 0, "{run}");
    let before = git(&repo.path, &["rev-parse", "HEAD"]);
    let (code, out) = flow_with_path(&repo.path, &shim, &["publish"], "update-ref-fail");
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_field(&out), "UPDATE_REF_REFUSED", "{out}");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        before,
        "a refused CAS must leave the ref alone"
    );
}

/// `update-ref` answering success without moving the ref is not a publication:
/// the post-CAS observation must refuse to report one.
#[cfg(unix)]
#[test]
fn publish_never_reports_success_when_the_ref_did_not_move() {
    let (repo, _, _) = prepared_repo(GREEN);
    let shim = shim_git(&repo.path, "update-ref-lies");
    let (code, run) = flow_with_path(&repo.path, &shim, &["run"], "normal");
    assert_eq!(code, 0, "{run}");
    let before = git(&repo.path, &["rev-parse", "HEAD"]);
    let (code, out) = flow_with_path(&repo.path, &shim, &["publish"], "update-ref-noop");
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_field(&out), "NEEDS_RECONCILIATION", "{out}");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        before,
        "nothing was published, so the ref must not have moved"
    );
}

/// A ref that moves between the durable intent and the CAS is detected before the
/// CAS, and the intent is closed with an explicit stale outcome.
#[cfg(unix)]
#[test]
fn publish_detects_a_ref_that_moved_between_intent_and_cas() {
    let (repo, _, _) = prepared_repo(GREEN);
    let shim = shim_git(&repo.path, "show-ref-moves");
    let (code, run) = flow_with_path(&repo.path, &shim, &["run"], "normal");
    assert_eq!(code, 0, "{run}");
    let before = git(&repo.path, &["rev-parse", "HEAD"]);
    let (code, out) = flow_with_path(&repo.path, &shim, &["publish"], "show-ref-moves");
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_field(&out), "REF_MOVED", "{out}");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        before,
        "a ref that moved under us must not be overwritten"
    );
    let journal = fs::read_to_string(journal_file(&repo.path)).unwrap();
    assert!(
        journal.contains("REFUSED_STALE"),
        "the intent must be closed with its stale outcome: {journal}"
    );
}

#[test]
fn status_and_explain_qualify_a_tampered_receipt() {
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");
    let tree = run
        .get("tree_oid")
        .and_then(Json::as_str)
        .unwrap()
        .to_string();
    let path = repo
        .path
        .join(".flow")
        .join("receipts")
        .join(format!("{tree}.json"));
    let mut value = read_json_file(&path);
    if let Json::Obj(pairs) = &mut value {
        for (k, v) in pairs.iter_mut() {
            if k == "status" {
                *v = Json::str("PASSED");
            }
        }
        pairs.push(("note".into(), Json::str("edited after the run")));
    }
    write_json_file(&path, &value);

    let (code, status) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{status}");
    let latest = status.get("latest_receipt").cloned().unwrap_or(Json::Null);
    assert_eq!(
        latest.get("trust").and_then(Json::as_str),
        Some("RECEIPT_INTEGRITY_INVALID"),
        "status must qualify the receipt: {status}"
    );
    assert_eq!(
        latest.get("status").and_then(Json::as_str),
        Some("RECEIPT_INTEGRITY_INVALID"),
        "a tampered receipt must not be displayed as PASSED: {status}"
    );

    let (code, explain) = flow(&repo.path, &["explain", "--tree", &tree]);
    assert_eq!(code, 0, "{explain}");
    assert_eq!(
        explain.get("receipt_trust").and_then(Json::as_str),
        Some("RECEIPT_INTEGRITY_INVALID")
    );
    assert_eq!(
        explain.get("receipt_status_display").and_then(Json::as_str),
        Some("RECEIPT_INTEGRITY_INVALID")
    );
    assert_eq!(
        explain.get("evidence_complete").and_then(Json::as_bool),
        Some(false)
    );
}

/// A diagnostic that found a broken guarantee exits non-zero, as documented.
#[test]
fn doctor_exits_nonzero_when_a_guarantee_is_broken() {
    let (repo, _, _) = prepared_repo(GREEN);
    fs::write(journal_file(&repo.path), "{\"truncated\": ").unwrap();
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 1, "doctor must fail on a torn journal: {out}");
    assert_eq!(status_of(&out), "REPORT");
    assert_eq!(out.get("overall").and_then(Json::as_str), Some("FAILED"));
}

/// A receipt whose log digests no longer describe the files it names is not
/// evidence: `explain` must say which log was swapped, and why it cannot answer.
#[test]
fn explain_qualifies_swapped_unverifiable_and_absent_log_digests() {
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");
    let tree = run
        .get("tree_oid")
        .and_then(Json::as_str)
        .unwrap()
        .to_string();
    let receipt_path = repo
        .path
        .join(".flow")
        .join("receipts")
        .join(format!("{tree}.json"));
    let receipt = read_json_file(&receipt_path);

    // 1. A log file whose bytes no longer hash to the recorded digest.
    let logs = receipt
        .get("logs")
        .and_then(Json::as_arr)
        .expect("the receipt names its logs")
        .to_vec();
    let first_log = logs
        .first()
        .and_then(Json::as_str)
        .expect("at least one log")
        .to_string();
    fs::write(repo.path.join(&first_log), "swapped evidence\n").unwrap();
    let (code, explain) = flow(&repo.path, &["explain", "--tree", &tree]);
    assert_eq!(code, 0, "{explain}");
    assert_eq!(
        explain.get("log_integrity").and_then(Json::as_str),
        Some("LOG_INTEGRITY_INVALID"),
        "a swapped log must be reported: {explain}"
    );
    assert!(
        explain
            .get("tampered_logs")
            .and_then(Json::as_arr)
            .map(|a| a.iter().any(|v| v.as_str() == Some(first_log.as_str())))
            .unwrap_or(false),
        "the swapped log must be named: {explain}"
    );

    // 2. A digest entry that no longer records one of its two hashes: the
    //    receipt cannot be verified, so the answer is a refusal, not a pass.
    let mut without_hash = receipt.clone();
    if let Json::Obj(pairs) = &mut without_hash {
        if let Some((_, Json::Arr(entries))) = pairs.iter_mut().find(|(k, _)| k == "log_digests") {
            if let Some(Json::Obj(first)) = entries.first_mut() {
                first.retain(|(k, _)| k != "stdout_sha256");
            }
        }
    }
    write_json_file(&receipt_path, &without_hash);
    let (code, explain) = flow(&repo.path, &["explain", "--tree", &tree]);
    assert_eq!(code, 0, "{explain}");
    assert_eq!(
        explain.get("log_integrity").and_then(Json::as_str),
        Some("LOG_INTEGRITY_INVALID"),
        "an incomplete digest record is not a verification: {explain}"
    );

    // 3. A receipt that records no log digests at all (written before digests
    //    existed) is explicitly unverifiable, never silently valid.
    let mut without_digests = receipt.clone();
    if let Json::Obj(pairs) = &mut without_digests {
        pairs.retain(|(k, _)| k != "log_digests");
    }
    write_json_file(&receipt_path, &without_digests);
    let (code, explain) = flow(&repo.path, &["explain", "--tree", &tree]);
    assert_eq!(code, 0, "{explain}");
    assert_eq!(
        explain.get("log_integrity").and_then(Json::as_str),
        Some("NO_LOG_DIGESTS"),
        "{explain}"
    );
}

/// A retention boundary the CLI cannot apply is a usage error, not a silent
/// clamp: zero would delete everything, and text would mean nothing.
#[test]
fn gc_refuses_retention_values_it_cannot_apply() {
    let repo = new_repo();
    for args in [
        ["gc", "--max-age-days", "0"],
        ["gc", "--max-age-days", "soon"],
        ["gc", "--keep", "0"],
        ["gc", "--keep", "-1"],
    ] {
        let (code, out) = flow(&repo.path, &args);
        assert_eq!(code, 2, "{args:?} must be a usage error: {out}");
    }
}

/// A toolchain probe that answers differently changes what a `PASSED` receipt
/// proves, even when the policy file itself did not change: publication must
/// refuse with `CONTEXT_MISMATCH`, not publish evidence from another context.
#[cfg(unix)]
#[test]
fn a_changed_context_probe_refuses_publication_without_a_policy_change() {
    use std::os::unix::fs::PermissionsExt;
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    let probe = repo.path.join("probe.sh");
    let observed = repo.path.join("probe.txt");
    fs::write(
        &probe,
        format!("#!/bin/sh\ncat \"{}\"\n", observed.display()),
    )
    .unwrap();
    fs::set_permissions(&probe, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(&observed, "toolchain-one\n").unwrap();
    fs::write(
        repo.path.join(".flow/config.json"),
        format!(
            r#"{{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{GREEN}],"context_probes":[["{}"]]}}"#,
            probe.display()
        ),
    )
    .unwrap();
    let (code, approved) = flow(&repo.path, &["approve"]);
    assert_eq!(code, 0, "{approved}");
    fs::write(repo.path.join("verified.txt"), "under verification\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");
    assert_eq!(status_of(&run), "PASSED");

    // The toolchain moved, the policy did not: the recorded context is stale.
    fs::write(&observed, "toolchain-two\n").unwrap();
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.to_string().contains("CONTEXT_MISMATCH"),
        "the refusal must name the context, not the policy: {out}"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head,
        "a stale context must not move the ref"
    );
}

/// `gc` must see a verification that is actually running: the marker is the
/// coordination point, and it is cleared on the run's own exit path.
#[cfg(unix)]
#[test]
fn gc_refuses_while_a_verification_is_actually_running() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(
        &repo.path,
        r#"{"id":"slow","program":"sh","args":["-c","sleep 3"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("verified.txt"), "under verification\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(&repo.path)
        .arg("run")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("flow run starts");
    let runs = repo.path.join(".flow").join("runs");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if let Ok(entries) = fs::read_dir(&runs) {
            if entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().ends_with(".active"))
            {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let (code, out) = flow(&repo.path, &["gc", "--keep", "1"]);
    assert_eq!(code, 1, "the refusal exits non-zero: {out}");
    assert_eq!(
        out.get("code").and_then(Json::as_str),
        Some("RUN_ACTIVE"),
        "gc must refuse while the run is writing evidence: {out}"
    );
    let status = child.wait().expect("the run finishes");
    assert!(
        status.success(),
        "the run itself must still succeed: {status}"
    );
    let (code, out) = flow(&repo.path, &["gc", "--keep", "1"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        status_of(&out),
        "GC",
        "after the run, the marker is gone and gc proceeds: {out}"
    );
}

/// A receipt written before `run_id`, log digests and the context codec existed
/// is not silently reinterpreted as current evidence: it is refused by name.
#[test]
fn a_legacy_receipt_without_a_context_is_refused_not_published() {
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");
    let tree = run
        .get("tree_oid")
        .and_then(Json::as_str)
        .unwrap()
        .to_string();
    let receipt_path = repo
        .path
        .join(".flow")
        .join("receipts")
        .join(format!("{tree}.json"));
    let mut receipt = read_json_file(&receipt_path);
    if let Json::Obj(pairs) = &mut receipt {
        pairs.retain(|(k, _)| k != "context_id" && k != "run_id" && k != "log_digests");
    }
    // Re-seal exactly as `state::seal_record` does, so the refusal under test is
    // the missing context, not a broken integrity digest.
    let mut body = receipt.clone();
    if let Json::Obj(pairs) = &mut body {
        pairs.retain(|(k, _)| k != "integrity");
    }
    let material = repo.path.join("receipt-body.txt");
    fs::write(&material, body.to_string()).unwrap();
    let digest = sha256_of(&material);
    let mut sealed = body;
    if let Json::Obj(pairs) = &mut sealed {
        pairs.push(("integrity".to_string(), Json::str(digest)));
    }
    write_json_file(&receipt_path, &sealed);

    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 1, "{out}");
    let text = out.to_string();
    assert!(
        text.contains("CONTEXT_MISMATCH") || text.contains("RECEIPT"),
        "a legacy receipt must be refused by a named check: {out}"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head,
        "an unusable legacy receipt must not publish anything"
    );
}

/// `status` says which policy is in force and who approved it.
#[test]
fn status_reports_policy_provenance() {
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    let policy = out.get("policy").cloned().unwrap_or(Json::Null);
    assert!(
        policy
            .get("path")
            .and_then(Json::as_str)
            .map(|p| p.ends_with("config.json"))
            .unwrap_or(false),
        "status must show the policy path: {out}"
    );
    assert!(policy
        .get("digest")
        .and_then(Json::as_str)
        .map(|d| d.len() >= 12)
        .unwrap_or(false));
    assert_eq!(policy.get("adopted").and_then(Json::as_bool), Some(true));
    assert!(policy.get("adopted_at").and_then(Json::as_str).is_some());
}

/// Deleted obligation logs are missing evidence, not a silent pass.
#[test]
fn explain_reports_missing_logs_as_incomplete_evidence() {
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");
    let tree = run
        .get("tree_oid")
        .and_then(Json::as_str)
        .unwrap()
        .to_string();
    let log_dir = repo.path.join(".flow").join("logs").join(&tree);
    for entry in fs::read_dir(&log_dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            for file in fs::read_dir(&path).unwrap().flatten() {
                fs::remove_file(file.path()).unwrap();
            }
        } else {
            fs::remove_file(path).unwrap();
        }
    }
    let (code, status) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{status}");
    let latest = status.get("latest_receipt").cloned().unwrap_or(Json::Null);
    assert_eq!(
        latest.get("evidence_complete").and_then(Json::as_bool),
        Some(false),
        "status must also say the evidence is incomplete: {status}"
    );
    let (code, explain) = flow(&repo.path, &["explain", "--tree", &tree]);
    assert_eq!(code, 0, "{explain}");
    assert_eq!(
        explain.get("evidence_complete").and_then(Json::as_bool),
        Some(false)
    );
    assert!(
        explain
            .get("missing_logs")
            .and_then(Json::as_arr)
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "the missing logs must be listed: {explain}"
    );
    // Publication still works: the receipt is intact, the logs are evidence that
    // is displayed as incomplete. This is a documentation decision, not a gate.
    let (code, published) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code, 0,
        "publish does not depend on log retention: {published}"
    );
}

/// A hard-linked journal must never be written through, on any platform.
///
/// The journal is rewritten and renamed, so a hard link is replaced instead of
/// appended to: the outside file keeps its bytes. (The Unix `nlink > 1` check is
/// a second line of defence; Windows has no portable link count in std.)
#[test]
fn a_hardlinked_journal_is_not_written_through() {
    let (repo, _, _) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);
    let outside = std::env::temp_dir().join(format!("flow-hardlink-{}", std::process::id()));
    fs::write(&outside, "outside\n").unwrap();
    let journal = journal_file(&repo.path);
    let _ = fs::remove_file(&journal);
    fs::hard_link(&outside, &journal).unwrap();
    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(
        fs::read_to_string(&outside).unwrap(),
        "outside\n",
        "the outside file must keep its bytes: {out}"
    );
    let outside_text = fs::read_to_string(&outside).unwrap();
    assert!(
        !outside_text.contains("flow-journal/1"),
        "no journal record may reach the outside file"
    );
    // When the publication succeeds, the record landed in the repository's own
    // file: the hard link was replaced, not appended to.
    if code == 0 {
        let in_repo = fs::read_to_string(&journal).unwrap_or_default();
        assert!(
            in_repo.contains("flow-journal/1"),
            "a successful publication must write its record inside the repository: {in_repo}"
        );
    }
    let _ = fs::remove_file(&outside);
}

/// The window before the durable intent: nothing was written, so there is
/// nothing to reconcile and no ref moved.
#[test]
fn crash_before_intent_leaves_no_pending_intent() {
    if skip_without_debug_hook("crash_before_intent_leaves_no_pending_intent") {
        return;
    }
    let (repo, _, head_before) = prepared_repo(GREEN);
    let (code, _) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0);
    let before = fs::read_to_string(journal_file(&repo.path)).unwrap();
    let (crash, _) = flow_with_env(
        &repo.path,
        &["publish"],
        &[("FLOW_TEST_CRASH", "before-intent")],
    );
    assert_eq!(crash, 89, "the simulated crash must not look like success");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(
        fs::read_to_string(journal_file(&repo.path)).unwrap(),
        before,
        "no intent may be appended before the injection point"
    );
    let (code, out) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "RECONCILED");
    assert_eq!(
        out.get("manual_required").and_then(Json::as_bool),
        Some(false)
    );
}

/// A durable record containing a non-finite number must be refused, not echoed
/// back as `inf` (which is not JSON). Found by the seeded fuzzer.
#[test]
fn a_non_finite_number_in_local_state_is_refused_not_printed() {
    let (repo, _, _) = prepared_repo(GREEN);
    let dir = repo.path.join(".flow").join("proposals");
    fs::create_dir_all(&dir).unwrap();
    let id = "0123456789abcdef0123456789abcdef";
    fs::write(
        dir.join(format!("{id}.json")),
        format!(
            r#"{{"schema":"flow-proposal/1","proposal_id":"{id}","agent_id":"a","task_id":"T","base_commit":"0000000000000000000000000000000000000000","base_tree":"0000000000000000000000000000000000000000","tree_oid":"0000000000000000000000000000000000000000","content_digest":"x","patch_bytes":1e999,"created_at":"x","changes":[]}}"#
        ),
    )
    .unwrap();
    let (code, out) = flow(&repo.path, &["proposals", "show", id]);
    assert_eq!(code, 1, "a non-finite number must be refused: {out}");
    let text = out.to_string();
    // The output must still be JSON (the harness parsed it) and must not carry a
    // bare `inf` value; the human message may of course mention infinity.
    assert_eq!(status_of(&out), "ERROR");
    assert!(
        !text.contains(":inf"),
        "the CLI must never print an inf value: {text}"
    );
    assert!(text.contains("not finite"), "{text}");
}

/// Two obligation ids that differ only by case would share one log file on a
/// case-insensitive filesystem and silently lose evidence.
#[test]
fn obligation_ids_that_differ_only_by_case_are_refused() {
    let (repo, _, _) = prepared_repo(GREEN);
    write_config_unapproved(
        &repo.path,
        r#"{"id":"PROBE","program":"git","args":["--version"],"required":true,"timeout_s":30},{"id":"probe","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "case-colliding ids must be refused: {out}");
    assert_eq!(status_of(&out), "REFUSED");
    assert!(out.to_string().contains("differ only by case"), "{out}");
}

/// On Windows a tool installed by npm is a `.cmd` shim, which `CreateProcess`
/// does not find by bare name. The obligation resolver uses PATH + PATHEXT.
#[cfg(windows)]
#[test]
fn a_cmd_shim_on_path_is_found_by_a_bare_program_name() {
    let (repo, _, _) = prepared_repo(GREEN);
    let shim_dir = repo.path.join("shims");
    fs::create_dir_all(&shim_dir).unwrap();
    fs::write(shim_dir.join("flowprobe.cmd"), "@echo off\r\nexit /b 0\r\n").unwrap();
    write_config(
        &repo.path,
        r#"{"id":"shim","program":"flowprobe","args":[],"required":true,"timeout_s":30}"#,
    );
    let path = std::env::var("PATH").unwrap_or_default();
    let prefixed = format!("{};{}", shim_dir.to_string_lossy(), path);
    let (code, out) = flow_with_env(&repo.path, &["run"], &[("PATH", &prefixed)]);
    assert_eq!(
        code, 0,
        "a .cmd shim on PATH must be executable by bare name: {out}"
    );
    assert_eq!(status_of(&out), "PASSED");
}

/// A boolean flag is a token, not a `key=value` pair. `--dry-run=true` used to
/// pass validation and then be ignored by `has_flag`, so an operator who asked
/// for a plan got a real publication: this is the regression that keeps "usage
/// error" ahead of "mutate the ref".
#[test]
fn a_boolean_flag_with_an_inline_value_is_a_usage_error_before_any_mutation() {
    let repo = new_repo();
    flow(&repo.path, &["init"]);
    write_config(
        &repo.path,
        r#"{"id":"git","program":"git","args":["--version"],"required":true,"timeout_s":30}"#,
    );
    fs::write(repo.path.join("p.txt"), "publishable\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    flow(&repo.path, &["run"]);
    let head_before = git(&repo.path, &["rev-parse", "HEAD"]);
    let journal_path = repo.path.join(".flow").join("journal.jsonl");
    let journal_before = fs::read_to_string(&journal_path).unwrap_or_default();

    for spelling in ["--dry-run=true", "--dry-run=false", "--json=1"] {
        let (code, out) = flow(&repo.path, &["publish", spelling]);
        assert_eq!(code, 2, "{spelling} must be a usage error, got {out}");
        assert_eq!(status_of(&out), "ERROR", "{spelling} must refuse: {out}");
        assert!(
            out.get("error")
                .and_then(Json::as_str)
                .unwrap_or("")
                .contains("takes no value"),
            "{spelling} must name the real problem: {out}"
        );
        assert_eq!(
            git(&repo.path, &["rev-parse", "HEAD"]),
            head_before,
            "{spelling} must never move the ref"
        );
        assert_eq!(
            fs::read_to_string(&journal_path).unwrap_or_default(),
            journal_before,
            "{spelling} must not append an intent to the journal"
        );
    }
}

/// A timeout that cannot be applied must not be dropped silently: the receipt
/// would then prove a deadline the run never used.
#[test]
fn a_timeout_that_cannot_be_applied_is_a_usage_error() {
    let (repo, _, _) = prepared_repo(GREEN);
    for bad in ["0", "abc"] {
        let (code, out) = flow(&repo.path, &["run", "--timeout", bad]);
        assert_eq!(code, 2, "`--timeout {bad}` must be a usage error: {out}");
        assert!(
            out.get("error")
                .and_then(Json::as_str)
                .unwrap_or("")
                .contains("--timeout"),
            "the refusal must name the flag: {out}"
        );
    }
}

/// `explain` lists the journal records **of the explained tree**. A record that
/// names another tree must not be attributed to this one: an operator reading
/// "this is the history of the tree I am shipping" must not be shown someone
/// else's outcome.
#[test]
fn explain_lists_only_the_journal_records_of_the_explained_tree() {
    let (repo, tree, _) = prepared_repo(GREEN);
    let (code, run) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "{run}");
    let (code, published) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code, 0,
        "the fixture needs a real publication so the journal names the tree: {published}"
    );

    let journal = journal_file(&repo.path);
    let mut text = fs::read_to_string(&journal).unwrap();
    text.push_str("{\"schema\":\"flow-journal/1\",\"event\":\"outcome\",\"id\":\"foreign\",\"status\":\"PUBLISHED\",\"tree\":\"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\",\"at\":\"2026-11-09T00:00:00Z\"}\n");
    fs::write(&journal, &text).unwrap();

    let (code, explain) = flow(&repo.path, &["explain", "--tree", &tree]);
    assert_eq!(code, 0, "{explain}");
    let related = explain
        .get("journal")
        .and_then(Json::as_arr)
        .expect("explain lists the journal records of the tree");
    assert!(
        related
            .iter()
            .any(|record| record.get("tree").and_then(Json::as_str) == Some(tree.as_str())),
        "the tree's own records must be listed: {explain}"
    );
    assert!(
        !related
            .iter()
            .any(|record| record.get("tree").and_then(Json::as_str)
                == Some("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")),
        "a record of another tree must not be attributed to this one: {explain}"
    );
}

/// On Unix the link count of a state file is visible, so a hard-linked adoption
/// record is refused outright: the atomic rename would replace the link, but an
/// outside writer holding the other name would keep writing to an inode Flow no
/// longer controls. The refusal is the guard's contract, not an incidental
/// outcome of the rename.
#[cfg(unix)]
#[test]
fn a_hardlinked_adoption_record_is_refused_on_unix() {
    let repo = new_repo();
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    write_config(&repo.path, GREEN); // approve writes .flow/adopted.json
    let adopted = repo.path.join(".flow").join("adopted.json");
    assert!(
        adopted.is_file(),
        "the fixture needs the adoption record: {adopted:?}"
    );
    let outside = std::env::temp_dir().join(format!(
        "flow-hardlink-adoption-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::write(&outside, "outside\n").unwrap();
    fs::remove_file(&adopted).unwrap();
    fs::hard_link(&outside, &adopted).unwrap();

    let (code, out) = flow(&repo.path, &["approve"]);
    assert_eq!(
        code, 1,
        "a hard-linked state file must be refused, not written through: {out}"
    );
    assert_eq!(status_of(&out), "ERROR");
    let error = out.get("error").and_then(Json::as_str).unwrap_or("<none>");
    assert!(
        error.contains("hard links"),
        "the refusal must name the hard link: {error}"
    );
    assert_eq!(
        fs::read_to_string(&outside).unwrap(),
        "outside\n",
        "the outside file must keep its bytes"
    );
    let _ = fs::remove_file(&outside);
}

/// `flow gc` is opt-in by construction: a retention policy is stated or nothing
/// happens, an unusable number is a usage error, and a stated policy reports its
/// plan before it removes anything.
#[test]
fn gc_requires_an_explicit_policy_and_reports_its_plan() {
    let repo = new_repo();
    let (code, out) = flow(&repo.path, &["init"]);
    assert_eq!(code, 0, "init: {out}");

    let (code, out) = flow(&repo.path, &["gc"]);
    assert_eq!(code, 2, "no policy is a usage error, not a default: {out}");
    assert_eq!(
        out.get("code").and_then(Json::as_str),
        Some("USAGE"),
        "the refusal must name USAGE: {out}"
    );

    let (code, out) = flow(&repo.path, &["gc", "--keep", "abc"]);
    assert_eq!(code, 2, "an unusable count is a usage error: {out}");

    let (code, out) = flow(&repo.path, &["gc", "--keep", "3", "--dry-run"]);
    assert_eq!(code, 0, "a plan is a success, not a mutation: {out}");
    assert_eq!(status_of(&out), "GC");
    assert_eq!(
        out.get("dry_run").and_then(|value| match value {
            Json::Bool(flag) => Some(*flag),
            _ => None,
        }),
        Some(true),
        "the plan must say it was a dry run: {out}"
    );
}

/// gc must not collect evidence while a live run is still writing it.
#[test]
fn gc_refuses_while_a_live_run_marker_exists() {
    let (repo, _, _) = prepared_repo(GREEN);
    let runs = repo.path.join(".flow").join("runs");
    fs::create_dir_all(&runs).unwrap();
    let marker = runs.join("live.active");
    fs::write(
        &marker,
        format!(
            r#"{{"schema":"flow-run-active/1","run_id":"live","pid":{},"tree":"t"}}"#,
            std::process::id()
        ),
    )
    .unwrap();
    let (code, out) = flow(&repo.path, &["gc", "--keep", "1"]);
    assert_eq!(code, 1, "gc must refuse while a run is active: {out}");
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(out.get("code").and_then(Json::as_str), Some("RUN_ACTIVE"));
    assert!(marker.exists(), "gc must not clear a live marker");
}
