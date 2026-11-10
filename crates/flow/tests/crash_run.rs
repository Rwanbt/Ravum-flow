//! Recovery gate: an execution killed in the middle of a verification.
//!
//! The scenarios the production audit lists as mandatory include "a process
//! killed during a validation" and "an agent restarted after a crash". The
//! publication transaction already has crash tests (`p0c_*`, `crash_*_intent`);
//! this one covers the *verification* phase: `flow run` is killed while an
//! obligation is executing.
//!
//! What must hold afterwards:
//! * no receipt is written — a killed run proves nothing (`PASSED` is never
//!   inferred from an interrupted execution);
//! * no lock is left behind, and no publication intent exists (a run never
//!   appends one);
//! * the repository is not wedged: once the policy is green again, `run` and
//!   `publish` work on the next attempt.
//!
//! What is explicitly **not** guaranteed: the disposable scratch directory of the
//! killed process is not removed (its `Drop` never ran). It is inert, uniquely
//! named `flow-scratch-<pid>-<seq>` under the system temp directory, and contains
//! only the tree the user asked to verify. The test removes it so it does not
//! accumulate on the builder host.

#[path = "../src/json.rs"]
#[allow(dead_code)]
mod json;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use json::Json;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The child probe runs as the obligation: it signals readiness, then waits for
/// the release file so it never outlives the test by accident.
const SLOW_PROBE: &str = "FLOW_CRASH_RUN_SLOW_PROBE";

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
    let path = std::env::temp_dir().join(format!("flow-crashrun-{tag}-{}-{n}", std::process::id()));
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Crash Run Test"]);
    git(
        &path,
        &["config", "user.email", "flow-crashrun@example.invalid"],
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

fn flow(repo: &Path, args: &[&str]) -> (i32, Json) {
    let out = Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(repo)
        .args(args)
        .output()
        .expect("flow runs");
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

fn status_of(value: &Json) -> String {
    value
        .get("status")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

const FAST: &str =
    r#"{"id":"fast","program":"git","args":["--version"],"required":true,"timeout_s":30}"#;

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
    let (code, out) = flow(repo, &["approve"]);
    assert_eq!(code, 0, "the fixture policy must be approvable: {out}");
    assert_eq!(status_of(&out), "APPROVED");
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

/// Child probe: signal readiness, then block until the release file exists. The
/// two marker paths travel as arguments because the obligation environment is
/// scrubbed (an environment variable could not carry them).
#[test]
fn slow_probe_child() {
    let args: Vec<String> = std::env::args().collect();
    let Some(position) = args.iter().position(|arg| arg == SLOW_PROBE) else {
        return;
    };
    let ready = PathBuf::from(args.get(position + 1).expect("ready path argument"));
    let release = PathBuf::from(args.get(position + 2).expect("release path argument"));
    fs::write(&ready, "ready\n").expect("write readiness marker");
    let deadline = Instant::now() + Duration::from_secs(60);
    while !release.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_for(path: &Path, budget: Duration) -> bool {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn remove_leaked_scratch(flow_pid: u32) {
    let prefix = format!("flow-scratch-{flow_pid}-");
    let temp = std::env::temp_dir();
    if let Ok(entries) = fs::read_dir(&temp) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

#[test]
fn a_run_killed_during_verification_leaves_no_proof_and_recovers() {
    let repo = new_repo("kill");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);

    let ready = repo.path.join("probe-ready");
    let release = repo.path.join("probe-release");
    let exe = std::env::current_exe()
        .expect("test executable")
        .to_string_lossy()
        .replace('\\', "/");
    let slash = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let slow = format!(
        r#"{{"id":"slow","program":"{}","args":["--exact","slow_probe_child","--nocapture","{SLOW_PROBE}","{}","{}"],"required":true,"timeout_s":120}}"#,
        exe.replace('"', ""),
        slash(&ready),
        slash(&release)
    );
    write_config(&repo.path, &slow);
    // A staged change so the verified tree is not already the ref's tree (publish
    // would otherwise refuse with ALREADY_PUBLISHED and prove nothing).
    fs::write(repo.path.join("b.txt"), "content under verification\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    let tree = git(&repo.path, &["write-tree"]);

    let mut child: Child = Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(&repo.path)
        .arg("run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("flow run spawns");
    let flow_pid = child.id();

    // Wait until the obligation is actually running, then kill `flow` itself.
    let started = wait_for(&ready, Duration::from_secs(30));
    if !started {
        let _ = child.kill();
        let _ = child.wait();
        remove_leaked_scratch(flow_pid);
        panic!(
            "the slow obligation never signalled readiness; the fixture cannot prove the scenario"
        );
    }
    let killed = child.kill();
    let _ = child.wait();
    assert!(
        killed.is_ok(),
        "the flow process must be killable while the obligation runs: {killed:?}"
    );

    // Let the orphan probe finish so it does not outlive the test, and clean the
    // scratch directory its killed parent never removed.
    fs::write(&release, "release\n").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    remove_leaked_scratch(flow_pid);

    let receipt = repo
        .path
        .join(".flow")
        .join("receipts")
        .join(format!("{tree}.json"));
    assert!(
        !receipt.exists(),
        "a run killed while an obligation was executing must not leave a receipt: {}",
        receipt.display()
    );
    assert!(
        !repo.path.join(".flow").join("lock").exists(),
        "a killed run must not leave a publication lock"
    );
    let journal = repo.path.join(".flow").join("journal.jsonl");
    let journal_text = fs::read_to_string(&journal).unwrap_or_default();
    assert!(
        journal_text.trim().is_empty(),
        "a run never appends a publication intent, so a killed run must leave the journal empty:\n{journal_text}"
    );

    // Recovery: a green policy on the same repository verifies and publishes.
    write_config(&repo.path, FAST);
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 0,
        "the repository must not be wedged after a kill: {out}"
    );
    assert_eq!(status_of(&out), "PASSED", "{out}");
    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 0, "publication after recovery must work: {out}");
    assert_eq!(status_of(&out), "PUBLISHED", "{out}");
}
