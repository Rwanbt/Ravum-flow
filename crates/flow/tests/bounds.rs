//! Boundary tests for the process and Git-execution edges of the CLI:
//!
//! * an obligation that floods stdout is bounded, and the receipt says so;
//! * hooks installed by the user are actually dispatched by the Git plumbing
//!   Flow calls (proven with canaries, never assumed), and a blocking hook's
//!   refusal propagates instead of being swallowed;
//! * a hostile `GIT_*` environment cannot redirect the CLI at another index or
//!   repository;
//! * a missing toolchain is `UNSUPPORTED`, never a pass.
//!
//! The last group attacks `run_bounded` directly. The crate is a binary without
//! a library target, so the module is included with the same `#[path]` trick as
//! `tests/isolation.rs`; nothing here modifies the production code.
//!
//! Measured boundaries are reported on stderr and asserted; a `SKIP` line is
//! only printed when the host cannot prove the property, never to hide a pass.

#[path = "../src/gitcli.rs"]
#[allow(dead_code)]
mod gitcli;
#[path = "../src/json.rs"]
#[allow(dead_code)]
mod json;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use gitcli::Git;
use json::Json;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The obligation cap is 1 MiB. The fixture writes ten times that so a stream
/// that is buffered without a bound would be impossible to miss.
const FLOOD_BYTES: usize = 10 * 1024 * 1024;
const OBLIGATION_CAP: usize = 1024 * 1024;

struct Repo {
    path: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = remove_tree(&self.path);
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
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

fn next_id() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn new_repo(tag: &str) -> Repo {
    let path = std::env::temp_dir().join(format!(
        "flow-bounds-{tag}-{}-{}",
        std::process::id(),
        next_id()
    ));
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Bounds Test"]);
    git(
        &path,
        &["config", "user.email", "flow-bounds@example.invalid"],
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

fn flow(repo: &Path, args: &[&str]) -> (i32, Json) {
    flow_env(repo, args, &[])
}

fn flow_env(repo: &Path, args: &[&str], envs: &[(&str, String)]) -> (i32, Json) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flow"));
    cmd.current_dir(repo).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("flow runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed = json::parse(&text).unwrap_or_else(|e| {
        panic!(
            "flow {args:?} printed non-JSON ({e}):\n{text}\n{}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code().unwrap_or(-1), parsed)
}

fn status_of(value: &Json) -> String {
    value
        .get("status")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

fn code_of(value: &Json) -> String {
    value
        .get("code")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

fn json_escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
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
    // The adoption gate: a policy is not executed until this working copy
    // approved its digest. Tests adopt it the way an operator would.
    let (code, out) = flow(repo, &["approve"]);
    assert_eq!(code, 0, "policy must be adoptable: {out}");
}

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

const GREEN: &str =
    r#"{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}"#;

/// A repository with staged content, `.flow/` excluded and the given policy.
fn prepared_repo(tag: &str, obligations: &str) -> (Repo, String, String) {
    let repo = new_repo(tag);
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

fn receipt_for(repo: &Path, tree: &str) -> PathBuf {
    repo.join(".flow")
        .join("receipts")
        .join(format!("{tree}.json"))
}

fn read_json_file(path: &Path) -> Json {
    json::parse(&fs::read_to_string(path).unwrap()).unwrap()
}

// --------------------------------------------------------- child probes
//
// The test binary is re-invoked by the CLI as the obligation program; the marker
// arrives through `std::env::args()` because an obligation's environment is
// scrubbed to an allowlist and could not carry a signal.
const FLOOD_ARG: &str = "FLOW_BOUNDS_FLOOD_CHILD";
const INDEX_PROBE_ARG: &str = "FLOW_BOUNDS_INDEX_PROBE_CHILD";

fn arg_present(marker: &str) -> bool {
    std::env::args().any(|arg| arg == marker)
}

/// Writes ten times the obligation cap to stdout and exits 0.
#[test]
fn flood_child_probe() {
    if !arg_present(FLOOD_ARG) {
        return;
    }
    use std::io::Write;
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

/// Proves, from a child process whose *ambient* environment carries a hostile
/// `GIT_INDEX_FILE`, that only the explicit `index` parameter reaches Git.
#[test]
fn index_probe_child() {
    if !arg_present(INDEX_PROBE_ARG) {
        return;
    }
    let repo = PathBuf::from(std::env::var_os("FLOW_BOUNDS_REPO").expect("probe repo"));
    let expected_own = std::env::var("FLOW_BOUNDS_OWN_TREE").expect("probe own tree");
    let expected_head = std::env::var("FLOW_BOUNDS_HEAD_TREE").expect("probe head tree");
    let explicit = PathBuf::from(std::env::var_os("FLOW_BOUNDS_EXPLICIT_INDEX").expect("index"));
    assert!(
        std::env::var_os("GIT_INDEX_FILE").is_some(),
        "the parent must hand the child a hostile ambient GIT_INDEX_FILE"
    );

    let git = Git::new(&repo);
    // No explicit index: the repository's own index (the staged change) must be
    // used even though the ambient environment points elsewhere.
    let own = git.write_tree().expect("write_tree must not be redirected");
    assert_eq!(
        own, expected_own,
        "ambient GIT_INDEX_FILE redirected write_tree"
    );

    // Explicit index: honoured, and for this invocation only.
    let read = git
        .run_bounded(&["read-tree", "HEAD"], None, Some(&explicit))
        .expect("read-tree into the explicit index");
    assert_eq!(read.status, 0, "read-tree failed: {}", read.stderr);
    let head_tree = git
        .run_bounded(&["write-tree"], None, Some(&explicit))
        .expect("write-tree from the explicit index");
    assert_eq!(
        head_tree.status, 0,
        "write-tree failed: {}",
        head_tree.stderr
    );
    assert_eq!(
        head_tree.stdout.trim(),
        expected_head,
        "the explicit index override was not honoured"
    );
    eprintln!(
        "MEASURED index: ambient ignored (own tree {own}), explicit honoured (head tree {})",
        head_tree.stdout.trim()
    );
}

fn install_hook(repo: &Path, name: &str, body: &str) {
    let hooks = repo.join(".git").join("hooks");
    fs::create_dir_all(&hooks).unwrap();
    let path = hooks.join(name);
    fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn logging_hook(canary: &Path, marker: &str, exit: i32) -> String {
    let path = canary.to_string_lossy().replace('\\', "/");
    format!("#!/bin/sh\necho \"{marker} $*\" >> \"{path}\"\nexit {exit}\n")
}

fn canary_text(canary: &Path) -> String {
    fs::read_to_string(canary).unwrap_or_default()
}

fn count_with_prefix(text: &str, prefix: &str) -> usize {
    text.lines().filter(|line| line.starts_with(prefix)).count()
}

// ------------------------------------------------------------------- tests

/// Ten MiB of obligation stdout must be drained and bounded, and the receipt
/// must admit the truncation instead of presenting clipped output as evidence.
#[test]
fn obligation_stdout_is_bounded_through_the_cli() {
    let exe = std::env::current_exe()
        .expect("test executable")
        .to_string_lossy()
        .into_owned();
    let obligation = format!(
        r#"{{"id":"flood","program":"{}","args":["--exact","flood_child_probe","--nocapture","{FLOOD_ARG}"],"required":true,"timeout_s":60}}"#,
        json_escape(&exe)
    );
    let (repo, tree, _) = prepared_repo("flood", &obligation);

    let started = Instant::now();
    let (code, out) = flow(&repo.path, &["run"]);
    let elapsed = started.elapsed();
    assert_eq!(code, 0, "a flood must not fail the obligation: {out}");
    assert_eq!(status_of(&out), "PASSED");
    assert!(
        elapsed < Duration::from_secs(30),
        "the flooded run took {elapsed:?}; it must terminate promptly"
    );

    let recorded = out
        .get("obligations")
        .and_then(Json::as_arr)
        .and_then(|arr| arr.first())
        .unwrap_or_else(|| panic!("receipt must record the obligation: {out}"));
    assert_eq!(
        recorded.get("stdout_truncated").and_then(Json::as_bool),
        Some(true),
        "the run output must flag stdout truncation: {out}"
    );

    let receipt = read_json_file(&receipt_for(&repo.path, &tree));
    let recorded = receipt
        .get("obligations")
        .and_then(Json::as_arr)
        .and_then(|arr| arr.first())
        .unwrap_or_else(|| panic!("receipt must record the obligation: {receipt}"));
    assert_eq!(
        recorded.get("stdout_truncated").and_then(Json::as_bool),
        Some(true),
        "the receipt must record stdout_truncated: {receipt}"
    );

    let log = repo
        .path
        .join(".flow")
        .join("logs")
        .join(&tree)
        .join("flood.stdout.log");
    let bytes = fs::metadata(&log).expect("stdout log exists").len() as usize;
    assert!(
        bytes <= OBLIGATION_CAP + 128,
        "the retained log must stay bounded, got {bytes} bytes"
    );
    eprintln!(
        "MEASURED flood: child_bytes={FLOOD_BYTES} log_bytes={bytes} elapsed_ms={}",
        elapsed.as_millis()
    );
}

/// With ordinary (non-blocking) hooks installed, both hooks are dispatched by
/// the Git plumbing Flow calls: `post-checkout` by `git worktree add` during
/// `flow run`, `reference-transaction` by `git update-ref` during `flow publish`.
/// Flow itself dispatches no hook. **By design**: the plumbing's hooks run and
/// their refusals propagate; Flow does not silently skip them.
#[test]
fn nonblocking_hooks_are_dispatched_by_run_and_publish() {
    let (repo, _, head_before) = prepared_repo("hooks-log", GREEN);
    let canary = repo.path.join(".flow").join("hook-canary.log");
    install_hook(
        &repo.path,
        "post-checkout",
        &logging_hook(&canary, "post-checkout", 0),
    );
    install_hook(
        &repo.path,
        "reference-transaction",
        &logging_hook(&canary, "reference-transaction", 0),
    );

    let (run_code, run_out) = flow(&repo.path, &["run"]);
    assert_eq!(
        run_code, 0,
        "non-blocking hooks must not fail the run: {run_out}"
    );
    let after_run = canary_text(&canary);
    assert!(
        count_with_prefix(&after_run, "post-checkout") >= 1,
        "`git worktree add` during flow run must dispatch post-checkout; canary:\n{after_run}"
    );

    let refs_before = count_with_prefix(&after_run, "reference-transaction");
    let (publish_code, publish_out) = flow(&repo.path, &["publish"]);
    assert_eq!(
        publish_code, 0,
        "non-blocking hooks must not fail publication: {publish_out}"
    );
    let after_publish = canary_text(&canary);
    assert!(
        count_with_prefix(&after_publish, "reference-transaction") > refs_before,
        "`git update-ref` during flow publish must dispatch reference-transaction; canary:\n{after_publish}"
    );
    assert!(
        after_publish.contains("reference-transaction prepared"),
        "the reference-transaction hook must observe the prepared state; canary:\n{after_publish}"
    );
    assert_ne!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

/// A blocking `post-checkout` makes `git worktree add` fail, so `flow run`
/// refuses `SCRATCH_REFUSED`: **by design**, Flow never verifies a tree it
/// could not materialise. The canary proves the hook ran; the refusal proves the
/// failure was not swallowed.
#[test]
fn blocking_post_checkout_makes_run_refuse_the_scratch() {
    let (repo, tree, head_before) = prepared_repo("hooks-post-checkout", GREEN);
    let canary = repo.path.join(".flow").join("hook-canary.log");
    install_hook(
        &repo.path,
        "post-checkout",
        &logging_hook(&canary, "post-checkout", 1),
    );

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "a blocking checkout hook must fail the run: {out}");
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(
        code_of(&out),
        "SCRATCH_REFUSED",
        "the refusal must name the failed materialisation, not a generic error: {out}"
    );
    assert!(
        count_with_prefix(&canary_text(&canary), "post-checkout") >= 1,
        "the hook must have run before the refusal"
    );
    assert!(
        !receipt_for(&repo.path, &tree).is_file(),
        "no receipt may be written when the tree could not be materialised"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "a refused run must not move the ref"
    );
}

/// A blocking `reference-transaction` aborts `git update-ref`, so publish is
/// refused without moving the ref: **by design**, the journal records the
/// refusal and the lock is released, so the repository is not wedged.
#[test]
fn blocking_reference_transaction_aborts_publish_without_moving_the_ref() {
    let (repo, tree, head_before) = prepared_repo("hooks-ref-tx", GREEN);
    let (run_code, run_out) = flow(&repo.path, &["run"]);
    assert_eq!(run_code, 0, "precondition: run must pass: {run_out}");
    assert!(
        receipt_for(&repo.path, &tree).is_file(),
        "precondition receipt"
    );

    let canary = repo.path.join(".flow").join("hook-canary.log");
    let canary_path = canary.to_string_lossy().replace('\\', "/");
    let hook = format!(
        "#!/bin/sh\necho \"reference-transaction $*\" >> \"{canary_path}\"\ncase \"$1\" in prepared) exit 1;; esac\nexit 0\n"
    );
    install_hook(&repo.path, "reference-transaction", &hook);

    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(
        code, 1,
        "a blocking transaction hook must abort publish: {out}"
    );
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(
        code_of(&out),
        "UPDATE_REF_REFUSED",
        "the refusal must name the refused update-ref: {out}"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "the ref must not move when the hook aborts the transaction"
    );
    let canary = canary_text(&canary);
    assert!(
        canary.contains("reference-transaction prepared"),
        "the hook must observe the prepared state; canary:\n{canary}"
    );
    assert!(
        canary.contains("reference-transaction aborted"),
        "git must tell the hook the transaction aborted; canary:\n{canary}"
    );
    assert!(
        !repo.path.join(".flow").join("lock").exists(),
        "the publication lock must be released after a refused update-ref"
    );
}

/// A hostile ambient `GIT_*` environment must not redirect the CLI. A control
/// proves the variables are effective for an unscrubbed Git, so a green result
/// cannot be vacuous.
#[test]
fn hijacked_git_environment_does_not_redirect_the_cli() {
    let (repo, tree, head) = prepared_repo("hijack", GREEN);
    let decoy = new_repo("hijack-decoy");
    fs::write(decoy.path.join("a.txt"), "decoy content\n").unwrap();
    git(&decoy.path, &["add", "-A"]);
    git(&decoy.path, &["commit", "-q", "-m", "decoy"]);

    let decoy_git = decoy.path.join(".git");
    // Control: an unscrubbed Git *is* redirected by the same variables.
    let control = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .args(["write-tree"])
        .env("GIT_DIR", &decoy_git)
        .env("GIT_WORK_TREE", &decoy.path)
        .env("GIT_INDEX_FILE", decoy_git.join("index"))
        .env("GIT_OBJECT_DIRECTORY", decoy_git.join("objects"))
        .output()
        .expect("control git runs");
    let control_tree = String::from_utf8_lossy(&control.stdout).trim().to_string();
    assert!(
        !control.status.success() || control_tree != tree,
        "control failed: an unscrubbed git was not redirected (tree {control_tree:?})"
    );

    let envs: Vec<(&str, String)> = vec![
        ("GIT_DIR", decoy_git.to_string_lossy().into_owned()),
        ("GIT_WORK_TREE", decoy.path.to_string_lossy().into_owned()),
        (
            "GIT_INDEX_FILE",
            decoy_git.join("index").to_string_lossy().into_owned(),
        ),
        (
            "GIT_OBJECT_DIRECTORY",
            decoy_git.join("objects").to_string_lossy().into_owned(),
        ),
        (
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            decoy_git.join("objects").to_string_lossy().into_owned(),
        ),
        ("GIT_COMMON_DIR", decoy_git.to_string_lossy().into_owned()),
        ("GIT_NAMESPACE", "hijack".to_string()),
        (
            "GIT_CONFIG",
            decoy
                .path
                .join("hostile-config")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "GIT_CONFIG_GLOBAL",
            decoy
                .path
                .join("hostile-global.gitconfig")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "GIT_CONFIG_SYSTEM",
            decoy
                .path
                .join("hostile-system.gitconfig")
                .to_string_lossy()
                .into_owned(),
        ),
        ("GIT_CONFIG_COUNT", "1".to_string()),
        ("GIT_CONFIG_KEY_0", "core.hooksPath".to_string()),
        (
            "GIT_CONFIG_VALUE_0",
            decoy.path.join("hooks").to_string_lossy().into_owned(),
        ),
        (
            "GIT_CEILING_DIRECTORIES",
            repo.path.to_string_lossy().into_owned(),
        ),
        ("GIT_DISCOVERY_ACROSS_FILESYSTEM", "1".to_string()),
    ];

    let (code, out) = flow_env(&repo.path, &["run"], &envs);
    assert_eq!(
        code, 0,
        "the scrubbed CLI must still verify the real repo: {out}"
    );
    assert_eq!(status_of(&out), "PASSED");
    assert_eq!(
        out.get("tree_oid").and_then(Json::as_str),
        Some(tree.as_str()),
        "the hostile index/repository must be ignored: {out}"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head,
        "verification must not touch the ref"
    );
}

/// A missing toolchain is an explicit `UNSUPPORTED`, never a green run, and its
/// receipt is never publishable.
#[test]
fn missing_toolchain_is_unsupported_and_never_publishable() {
    let obligation = r#"{"id":"absent","program":"definitely-not-a-real-flow-bounds-tool","args":[],"required":true,"timeout_s":5}"#;
    let (repo, tree, _) = prepared_repo("missing-tool", obligation);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "a missing toolchain must not pass: {out}");
    assert_eq!(
        status_of(&out),
        "UNSUPPORTED",
        "a missing program is UNSUPPORTED, not FAILED/UNKNOWN: {out}"
    );
    assert!(
        receipt_for(&repo.path, &tree).is_file(),
        "the unsupported outcome must still be recorded in a receipt"
    );

    let (publish_code, publish) = flow(&repo.path, &["publish"]);
    assert_eq!(
        publish_code, 1,
        "an UNSUPPORTED receipt is not publishable: {publish}"
    );
    assert_eq!(
        code_of(&publish),
        "REQUIRED_NOT_PASSED",
        "publish must name the non-passed required obligation: {publish}"
    );
    assert!(
        !repo.path.join(".flow").join("lock").exists(),
        "the publication lock must be released after a refusal"
    );
}

// ------------------------------------------------- run_bounded, directly

/// A Git command producing more than 64 MiB on stdout is refused, not clipped,
/// and the call returns in bounded time instead of leaving a writer running.
#[test]
fn run_bounded_refuses_more_than_64_mib() {
    let repo = new_repo("big-git-output");
    let big = repo.path.join("big.bin");
    let chunk = vec![b'x'; 1024 * 1024];
    {
        use std::io::Write;
        let mut file = fs::File::create(&big).unwrap();
        for _ in 0..(gitcli::MAX_GIT_STREAM_BYTES / chunk.len() + 1) {
            file.write_all(&chunk).unwrap();
        }
        file.sync_all().unwrap();
    }
    let oid = git(&repo.path, &["hash-object", "-w", "big.bin"]);

    let started = Instant::now();
    let err = match Git::new(&repo.path).run_bounded(&["cat-file", "blob", &oid], None, None) {
        Err(e) => e,
        Ok(output) => panic!(
            "more than MAX_GIT_STREAM_BYTES on stdout must be refused, got status {}",
            output.status
        ),
    };
    let elapsed = started.elapsed();
    assert!(
        err.contains("GIT_OUTPUT_TOO_LARGE"),
        "the error must be typed and name the stream: {err}"
    );
    assert!(
        err.contains("stdout"),
        "the message must name stdout: {err}"
    );
    assert!(
        elapsed < Duration::from_secs(60),
        "the refusal took {elapsed:?}; a flood must be drained and refused promptly"
    );
    // The call returns only after the child was reaped (try_wait loop), and the
    // next invocation works: a surviving writer holding the pipe would wedge
    // the following read.
    let after = Git::new(&repo.path)
        .run_bounded(&["rev-parse", "HEAD"], None, None)
        .expect("a bounded call must work after a refused one");
    assert_eq!(after.status, 0);
    eprintln!(
        "MEASURED git cap: child_bytes={} elapsed_ms={} error={err}",
        fs::metadata(&big).unwrap().len(),
        elapsed.as_millis()
    );
}

/// `stdin` is written to the child and closed, so a plumbing command that reads
/// its input (`hash-object --stdin`) works.
#[test]
fn run_bounded_feeds_stdin_to_git() {
    let repo = new_repo("stdin");
    let git = Git::new(&repo.path);
    let input = b"canary-object-body\n";
    let hashed = git
        .run_bounded(&["hash-object", "-w", "--stdin"], Some(input), None)
        .expect("hash-object --stdin must run");
    assert_eq!(hashed.status, 0, "hash-object failed: {}", hashed.stderr);
    let oid = hashed.stdout.trim().to_string();
    assert_eq!(oid.len(), 40, "expected a full object id, got {oid:?}");
    let stored = git
        .run_bounded(&["cat-file", "blob", &oid], None, None)
        .expect("the object must have been stored");
    assert_eq!(stored.status, 0);
    assert_eq!(
        stored.stdout.as_bytes(),
        input,
        "stdin bytes must round-trip"
    );
}

/// The explicit `index` parameter is honoured; an ambient `GIT_INDEX_FILE` is
/// scrubbed. The ambient case runs in a child process because mutating this
/// process's environment would race the rest of the suite.
#[test]
fn run_bounded_ignores_ambient_index_and_honours_the_explicit_one() {
    let (repo, own_tree, _head_oid) = prepared_repo("index", GREEN);
    let head_tree = git(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let decoy_index = std::env::temp_dir().join(format!(
        "flow-bounds-decoy-index-{}-{}",
        std::process::id(),
        next_id()
    ));
    // The decoy index encodes HEAD's tree (no staged change); `own_tree` is the
    // staged tree, so the two are distinguishable.
    assert_ne!(own_tree, head_tree);
    let created = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .args(["read-tree", "HEAD"])
        .env("GIT_INDEX_FILE", &decoy_index)
        .status()
        .expect("read-tree into the decoy index runs");
    assert!(created.success(), "cannot build the decoy index");

    let explicit_index = std::env::temp_dir().join(format!(
        "flow-bounds-explicit-index-{}-{}",
        std::process::id(),
        next_id()
    ));
    let _ = fs::remove_file(&explicit_index);

    let exe = std::env::current_exe().expect("test executable");
    let out = Command::new(exe)
        .args([
            "--exact",
            "index_probe_child",
            "--nocapture",
            INDEX_PROBE_ARG,
        ])
        .env("FLOW_BOUNDS_REPO", &repo.path)
        .env("FLOW_BOUNDS_OWN_TREE", &own_tree)
        .env("FLOW_BOUNDS_HEAD_TREE", &head_tree)
        .env("FLOW_BOUNDS_EXPLICIT_INDEX", &explicit_index)
        .env("GIT_INDEX_FILE", &decoy_index)
        .stdin(Stdio::null())
        .output()
        .expect("index probe child runs");
    assert!(
        out.status.success(),
        "index probe failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = fs::remove_file(&decoy_index);
    let _ = fs::remove_file(&explicit_index);
}
