//! Secret-handling gates: what an obligation process inherits, and what reaches
//! the durable evidence.
//!
//! The threat is not "the obligation can read your disk" (there is no sandbox and
//! the product says so); it is that Flow *hands* a credential to untrusted code or
//! *persists* one in its own logs and receipts. Two invariants are asserted here:
//!
//! 1. An obligation runs with a scrubbed environment: a variable that is not on
//!    the documented allowlist is absent from the child, even when the operator's
//!    shell exports it.
//! 2. A secret-shaped value printed by an obligation is redacted **before** it is
//!    written to `.flow/` (logs, receipt previews, journal): no durable file in
//!    Flow's own state may contain it.
//!
//! The canaries are fictitious. Nothing here reads a real credential.

#[path = "../src/json.rs"]
#[allow(dead_code)]
mod json;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use json::Json;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// An environment variable name and value that must never reach a child process.
const CANARY_ENV: &str = "FLOW_TEST_SECRET_CANARY";
const CANARY_VALUE: &str = "super-secret-canary-value-1234";
/// Fictitious GitHub- and AWS-shaped tokens (obvious fakes, correct shapes).
const GITHUB_SHAPED: &str = "ghp_abcdefghijklmnopqrstuvwxyz123456";
const AWS_SHAPED: &str = "AKIAIOSFODNN7EXAMPLE";

const ENV_PROBE: &str = "FLOW_SECRET_ENV_PROBE_CHILD";
const PRINT_PROBE: &str = "FLOW_SECRET_PRINT_PROBE_CHILD";

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

fn new_repo(tag: &str) -> Repo {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("flow-secrets-{tag}-{}-{n}", std::process::id()));
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Secrets Test"]);
    git(
        &path,
        &["config", "user.email", "flow-secrets@example.invalid"],
    );
    git(&path, &["config", "commit.gpgsign", "false"]);
    git(&path, &["config", "core.autocrlf", "false"]);
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

fn flow_with_env(repo: &Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, Json) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flow"));
    cmd.current_dir(repo).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("flow runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
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

fn flow(repo: &Path, args: &[&str]) -> (i32, Json) {
    flow_with_env(repo, args, &[])
}

fn status_of(value: &Json) -> String {
    value
        .get("status")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

/// Adopt a policy whose single required obligation is the given program/args.
fn adopt_policy(repo: &Path, obligation: &str) {
    let dir = repo.join(".flow");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.json"),
        format!(
            r#"{{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{obligation}],"disabled":false}}"#
        ),
    )
    .unwrap();
    let (code, out) = flow(repo, &["approve"]);
    assert_eq!(code, 0, "the policy must be approvable: {out}");
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

/// The test binary itself is the obligation: it is the only program guaranteed to
/// exist on every platform this suite runs on.
fn probe_obligation(test: &str, marker: &str) -> String {
    let exe = std::env::current_exe()
        .expect("test executable")
        .to_string_lossy()
        .replace('\\', "/");
    format!(
        r#"{{"id":"probe","program":"{}","args":["--exact","{test}","--nocapture","{marker}"],"required":true,"timeout_s":60}}"#,
        exe.replace('"', "")
    )
}

/// Recursively collect every file under `dir` (Flow's own state).
fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && !path.is_symlink() {
                files_under(&path, out);
            } else {
                out.push(path);
            }
        }
    }
}

fn assert_no_durable_file_contains(repo: &Path, needle: &str) {
    let mut files = Vec::new();
    files_under(&repo.join(".flow"), &mut files);
    assert!(
        !files.is_empty(),
        "the run must have written evidence under .flow/"
    );
    for path in files {
        let bytes = fs::read(&path).unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains(needle),
            "the value {needle:?} reached the durable file {}: a secret must be redacted before persistence, not after",
            path.display()
        );
    }
}

fn read_logs(repo: &Path, name: &str) -> String {
    let mut text = String::new();
    let dir = repo.join(".flow").join("logs");
    if let Ok(trees) = fs::read_dir(&dir) {
        for tree in trees.flatten() {
            let path = tree.path().join(name);
            if let Ok(content) = fs::read_to_string(&path) {
                text.push_str(&content);
            }
        }
    }
    text
}

// ------------------------------------------------------------- child probes

/// Prints whether the canary variable survived the environment scrub. Returns
/// immediately in a normal test run.
#[test]
fn env_probe_child() {
    if !std::env::args().any(|arg| arg == ENV_PROBE) {
        return;
    }
    match std::env::var(CANARY_ENV) {
        Ok(value) => println!("PRESENT {value}"),
        Err(_) => println!("ABSENT"),
    }
}

/// Prints fictitious secret-shaped values, to prove the capture is redacted
/// before it is persisted.
#[test]
fn print_probe_child() {
    if !std::env::args().any(|arg| arg == PRINT_PROBE) {
        return;
    }
    println!("github-shaped={GITHUB_SHAPED} aws-shaped={AWS_SHAPED} done");
    eprintln!("stderr carries {GITHUB_SHAPED}");
}

// ------------------------------------------------------------------- gates

/// The operator's shell exports a canary; the obligation must not see it. The
/// proof is in the obligation's own captured stdout: it prints `PRESENT <value>`
/// when the variable survived, `ABSENT` otherwise, and no durable file may hold
/// the value either.
#[test]
fn an_unlisted_environment_variable_is_not_inherited_by_an_obligation() {
    let repo = new_repo("env");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    adopt_policy(&repo.path, &probe_obligation("env_probe_child", ENV_PROBE));
    git(&repo.path, &["add", "-A"]);

    let (code, out) = flow_with_env(
        &repo.path,
        &["run"],
        &[(CANARY_ENV, CANARY_VALUE), ("GITHUB_TOKEN", CANARY_VALUE)],
    );
    assert_eq!(code, 0, "the probe obligation must run: {out}");
    assert_eq!(status_of(&out), "PASSED", "{out}");

    let stdout = read_logs(&repo.path, "probe.stdout.log");
    assert!(
        stdout.contains("ABSENT"),
        "the obligation must see the canary variable as absent; captured stdout:\n{stdout}"
    );
    assert_no_durable_file_contains(&repo.path, CANARY_VALUE);
}

/// A secret-shaped value printed by an obligation must be redacted before it is
/// written to `.flow/`: neither the GitHub-shaped nor the AWS-shaped canary may
/// appear in a log, a receipt or the journal, and the log must show the redaction
/// marker instead (so the operator can see that something was removed).
#[test]
fn a_secret_shaped_value_is_redacted_before_it_is_persisted() {
    let repo = new_repo("redact");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    adopt_policy(
        &repo.path,
        &probe_obligation("print_probe_child", PRINT_PROBE),
    );
    git(&repo.path, &["add", "-A"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "the printing obligation must run: {out}");
    assert_eq!(status_of(&out), "PASSED", "{out}");

    assert_no_durable_file_contains(&repo.path, GITHUB_SHAPED);
    assert_no_durable_file_contains(&repo.path, AWS_SHAPED);
    let stdout = read_logs(&repo.path, "probe.stdout.log");
    assert!(
        stdout.contains("[REDACTED]"),
        "the persisted log must show the redaction marker; captured stdout:\n{stdout}"
    );
    let stderr = read_logs(&repo.path, "probe.stderr.log");
    assert!(
        stderr.contains("[REDACTED]"),
        "the persisted stderr log must show the redaction marker:\n{stderr}"
    );
}
