//! Concurrency tests against the shipped CLI: publication at-most-once, and the
//! `flow-lock/2` attribution rules that make a second writer stop instead of
//! interleaving.
//!
//! Every test uses real processes and a real `git`: a lock that only looks right
//! in-process proves nothing about two writers. Fixtures are disposable
//! repositories under the system temp directory, exactly as `tests/cli.rs`
//! builds them.
//!
//! A skipped test is not a pass: anything that cannot be proven on this host
//! prints `SKIP <test>: <reason>` and asserts nothing else.

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

/// Upper bound for one test's concurrent phase. The lock budget is 3 s, so a
/// healthy run finishes far below this; the deadline exists to fail a wedged
/// process instead of hanging the suite.
const CONCURRENCY_DEADLINE: Duration = Duration::from_secs(60);

struct Repo {
    path: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = remove_tree(&self.path);
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
    // A test fixture must be deletable, so read-only bits Git sets on object
    // files are cleared before removing the tree.
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
        "flow-conc-{tag}-{}-{}",
        std::process::id(),
        next_id()
    ));
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Concurrency Test"]);
    git(
        &path,
        &["config", "user.email", "flow-concurrency@example.invalid"],
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

/// Flow's own state must never enter the verified tree. `flow init` grows the
/// same exclusion in `.flow/.gitignore`; the fixture sets it directly so the
/// tree is stable before the first run.
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

/// A repository with a staged change that differs from `HEAD`: a verified tree
/// that is already the ref's tree has nothing to publish.
fn prepared_repo(tag: &str) -> (Repo, String, String) {
    let repo = new_repo(tag);
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
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

fn journal_lines(repo: &Path) -> Vec<Json> {
    let path = repo.join(".flow").join("journal.jsonl");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            json::parse(line).unwrap_or_else(|e| panic!("journal line is not JSON ({e}): {line}"))
        })
        .collect()
}

fn count_event(records: &[Json], event: &str) -> usize {
    records
        .iter()
        .filter(|r| r.get("event").and_then(Json::as_str) == Some(event))
        .count()
}

fn spawn_flow(repo: &Path, args: &'static [&'static str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(repo)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("flow child spawns")
}

/// Wait for every child, killing the whole batch if the deadline expires. A
/// hanging test masks a lock defect, so the deadline is enforced, not hoped for.
fn wait_all(mut children: Vec<Child>) -> Vec<std::process::Output> {
    let started = Instant::now();
    loop {
        let mut exited = true;
        for child in &mut children {
            if child.try_wait().expect("try_wait").is_none() {
                exited = false;
            }
        }
        if exited {
            break;
        }
        if started.elapsed() > CONCURRENCY_DEADLINE {
            for child in &mut children {
                let _ = child.kill();
            }
            panic!("concurrent writers did not finish within {CONCURRENCY_DEADLINE:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    children
        .into_iter()
        .map(|child| child.wait_with_output().expect("child output"))
        .collect()
}

fn parse_output(label: &str, out: &std::process::Output) -> Json {
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    json::parse(&text).unwrap_or_else(|e| {
        panic!(
            "{label} printed non-JSON ({e}):\n{text}\n{}",
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

// ------------------------------------------------------------------- tests

/// Eight cooperative writers, one ref, one verified tree: exactly one may move
/// the ref, every other must report a refusal, and the journal must hold one
/// intent/outcome pair. This is the 8-process version of the two-process test
/// in `tests/cli.rs`; the extra six widen the race window around the lock.
#[test]
fn eight_concurrent_publishes_publish_at_most_once() {
    let (repo, tree, head_before) = prepared_repo("eight");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "precondition: run must pass: {out}");
    assert_eq!(status_of(&out), "PASSED");

    let children: Vec<Child> = (0..8)
        .map(|_| spawn_flow(&repo.path, &["publish"]))
        .collect();
    let outputs = wait_all(children);
    assert_eq!(outputs.len(), 8);

    let mut published = 0;
    let mut refusals = 0;
    for (index, output) in outputs.iter().enumerate() {
        let value = parse_output(&format!("publish[{index}]"), output);
        let status = status_of(&value);
        if status == "PUBLISHED" {
            published += 1;
        } else {
            let exit = output.status.code().unwrap_or(-1);
            assert_ne!(
                exit, 0,
                "a non-published writer must exit nonzero, got {exit}: {value}"
            );
            assert_eq!(
                status, "REFUSED",
                "a loser must be refused explicitly, not crash: {value}"
            );
            let code = code_of(&value);
            assert!(
                code == "LOCKED" || code == "ALREADY_PUBLISHED",
                "a loser must carry a refusal code (LOCKED or ALREADY_PUBLISHED), got {code:?}: {value}"
            );
            refusals += 1;
        }
    }
    assert_eq!(
        published, 1,
        "exactly one cooperative writer may publish the same tree; got {published} winners"
    );
    assert_eq!(refusals, 7, "every other writer must be refused");

    // One commit, carrying exactly the verified tree.
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    assert_ne!(head, head_before, "the verified tree must be published");
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD^{tree}"]),
        tree,
        "the published commit must carry exactly the verified tree"
    );
    assert_eq!(
        git(&repo.path, &["rev-list", "--count", "HEAD"]),
        "2",
        "exactly one commit may be added to the seed commit"
    );

    // Exactly one intent/outcome pair: interleaved or duplicated appends would
    // make the journal ambiguous even when the ref itself looks correct.
    let records = journal_lines(&repo.path);
    assert_eq!(
        count_event(&records, "intent"),
        1,
        "one publication attempt may be journaled: {records:?}"
    );
    let outcomes: Vec<&Json> = records
        .iter()
        .filter(|r| r.get("event").and_then(Json::as_str) == Some("outcome"))
        .collect();
    assert_eq!(outcomes.len(), 1, "exactly one outcome: {records:?}");
    assert_eq!(
        outcomes[0].get("status").and_then(Json::as_str),
        Some("PUBLISHED")
    );

    // And the journal is still consumable by the CLI.
    let (explain_code, explain) = flow(&repo.path, &["explain"]);
    assert_eq!(explain_code, 0, "journal must stay readable: {explain}");
}

/// A live owner is never displaced, whatever its age. Timestamp 0 is older than
/// every documented threshold, so only liveness can explain a refusal.
#[test]
fn a_live_old_lock_is_never_stolen() {
    let (repo, tree, head_before) = prepared_repo("live-lock");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "precondition: run must pass: {out}");
    assert!(
        receipt_for(&repo.path, &tree).is_file(),
        "precondition receipt"
    );

    let lock = repo.path.join(".flow").join("lock");
    let record = format!(
        "flow-lock/2 {} 0 canary-live-nonce publish\n",
        std::process::id()
    );
    fs::write(&lock, &record).unwrap();

    let (publish_code, publish) = flow(&repo.path, &["publish"]);
    assert_eq!(
        publish_code, 1,
        "a live owner must block publication: {publish}"
    );
    assert_eq!(status_of(&publish), "REFUSED");
    assert_eq!(
        code_of(&publish),
        "LOCKED",
        "a live owner is LOCKED, not unattributable: {publish}"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "a refused publication must not move the ref"
    );
    assert_eq!(
        fs::read_to_string(&lock).unwrap(),
        record,
        "a refusal must not rewrite or delete the live lock"
    );
}

/// A provably dead owner's lock is recovered, so a crashed writer cannot wedge
/// the repository forever.
#[test]
fn a_dead_owners_lock_is_recovered() {
    let (repo, tree, head_before) = prepared_repo("dead-lock");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "precondition: run must pass: {out}");
    assert!(
        receipt_for(&repo.path, &tree).is_file(),
        "precondition receipt"
    );

    let pid = spawn_and_reap_short_lived();
    match pid_is_dead(pid) {
        Some(true) => {}
        other => {
            println!(
                "SKIP a_dead_owners_lock_is_recovered: cannot prove pid {pid} is dead on this host (liveness probe said {other:?}), so a recovered lock would not prove anything"
            );
            return;
        }
    }

    let lock = repo.path.join(".flow").join("lock");
    fs::write(
        &lock,
        format!("flow-lock/2 {pid} 0 canary-dead-nonce publish\n"),
    )
    .unwrap();

    let (publish_code, publish) = flow(&repo.path, &["publish"]);
    assert_eq!(
        publish_code, 0,
        "a provably dead owner's lock must be recovered: {publish}"
    );
    assert_eq!(status_of(&publish), "PUBLISHED");
    assert!(
        !lock.exists(),
        "the recovered lock must be gone after the guard is released"
    );
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    assert_ne!(head, head_before, "the recovered writer must publish");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD^{tree}"]), tree);
}

/// An unattributable lock is a refusal, never a guess, and the file is left for
/// a human: an empty file may be a crash between create and write, a foreign
/// record may be another tool's.
#[test]
fn an_unparseable_lock_is_refused_and_preserved() {
    let repo = new_repo("unparseable");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    let lock = repo.path.join(".flow").join("lock");

    for (label, contents) in [
        ("empty", String::new()),
        (
            "foreign",
            "another-tool/9 4242 0 nonce something\n".to_string(),
        ),
        ("garbage", "not a lock record at all\n".to_string()),
    ] {
        fs::write(&lock, &contents).unwrap();
        let (code, out) = flow(&repo.path, &["publish"]);
        assert_eq!(code, 1, "{label} lock must refuse publication: {out}");
        assert_eq!(status_of(&out), "REFUSED", "{label}: {out}");
        assert_eq!(
            code_of(&out),
            "LOCK_UNKNOWN",
            "{label} lock is unattributable, not merely held: {out}"
        );
        assert_eq!(
            out.get("manual_required").and_then(Json::as_bool),
            Some(true),
            "{label} lock must ask for a human: {out}"
        );
        assert_eq!(
            fs::read_to_string(&lock).unwrap(),
            contents,
            "{label} lock must be left exactly in place"
        );
    }
}

/// Reconcile takes the same lock. Two concurrent runs may serialise or one may
/// be refused, but they must never append the same outcome twice.
#[test]
fn reconcile_is_serialized_and_never_duplicates_outcomes() {
    let repo = new_repo("reconcile");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);

    // One pending intent whose ref observation is decisive: `main` points at
    // `new`, so reconcile must record RECONCILED_PUBLISHED exactly once.
    let old = git(&repo.path, &["rev-parse", "HEAD"]);
    git(
        &repo.path,
        &["commit", "-q", "--allow-empty", "-m", "second"],
    );
    let new = git(&repo.path, &["rev-parse", "HEAD"]);
    let tree = git(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let journal = repo.path.join(".flow").join("journal.jsonl");
    fs::write(
        &journal,
        format!(
            "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"race-1\",\"ref\":\"refs/heads/main\",\"old\":\"{old}\",\"new\":\"{new}\",\"tree\":\"{tree}\",\"at\":\"2026-11-09T00:00:00Z\"}}\n"
        ),
    )
    .unwrap();

    let children: Vec<Child> = (0..2)
        .map(|_| spawn_flow(&repo.path, &["reconcile"]))
        .collect();
    let outputs = wait_all(children);

    let mut succeeded = 0;
    for (index, output) in outputs.iter().enumerate() {
        let value = parse_output(&format!("reconcile[{index}]"), output);
        let exit = output.status.code().unwrap_or(-1);
        if exit == 0 {
            succeeded += 1;
            let status = status_of(&value);
            assert!(
                status == "RECONCILED" || status == "NEEDS_RECONCILIATION",
                "a successful reconcile must report a reconciliation status: {value}"
            );
        } else {
            assert_eq!(status_of(&value), "REFUSED", "reconcile[{index}]: {value}");
            assert_eq!(
                code_of(&value),
                "LOCKED",
                "a losing reconcile must be LOCKED, not crash: {value}"
            );
        }
    }
    assert!(
        succeeded >= 1,
        "at least one concurrent reconcile must acquire the lock: {outputs:?}"
    );

    // The journal is parseable and the outcome was appended exactly once.
    let records = journal_lines(&repo.path);
    let outcomes: Vec<&Json> = records
        .iter()
        .filter(|r| r.get("event").and_then(Json::as_str) == Some("outcome"))
        .collect();
    assert_eq!(
        outcomes.len(),
        1,
        "serialisation must prevent a duplicate outcome append: {records:?}"
    );
    let status = outcomes[0]
        .get("status")
        .and_then(Json::as_str)
        .unwrap_or("");
    assert!(
        status.starts_with("RECONCILED_"),
        "the outcome must record the observed resolution, got {status:?}"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        new,
        "reconcile must never move a ref"
    );
}

// ------------------------------------------------------------ host probes

#[cfg(windows)]
fn spawn_and_reap_short_lived() -> u32 {
    let mut child = Command::new("cmd")
        .args(["/C", "exit", "0"])
        .spawn()
        .expect("short-lived child spawns");
    let pid = child.id();
    child.wait().expect("short-lived child reaped");
    pid
}

#[cfg(unix)]
fn spawn_and_reap_short_lived() -> u32 {
    let mut child = Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("short-lived child spawns");
    let pid = child.id();
    child.wait().expect("short-lived child reaped");
    pid
}

/// `Some(false)` only when the host proves the PID is gone; `None` when the
/// probe itself is unavailable, because ignorance must never become a steal.
#[cfg(windows)]
fn pid_is_dead(pid: u32) -> Option<bool> {
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(!(text.contains(&format!("\"{pid}\"")) || text.contains(&format!(",{pid},"))))
}

#[cfg(unix)]
fn pid_is_dead(pid: u32) -> Option<bool> {
    let status = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .ok()?;
    Some(!status.success())
}
