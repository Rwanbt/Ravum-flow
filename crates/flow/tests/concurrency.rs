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

#[path = "../src/sha256.rs"]
#[allow(dead_code)]
mod sha256;

#[path = "../src/host.rs"]
#[allow(dead_code)]
mod host;

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
    // An unapproved policy is never executed, so the fixture approves it first.
    let (code, out) = flow(repo, &["approve"]);
    assert_eq!(code, 0, "policy must be adoptable: {out}");
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
    let record = format!(
        "flow-lock/2 {pid} 0 canary-dead-nonce publish host:{}\n",
        local_host_token()
    );
    fs::write(&lock, &record).unwrap();

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

// ------------------------------------------------------------ G1 (FBL-001)

/// The exact patch bytes a G1 writer submits: the same key, eight different
/// contents.
fn g1_patch(writer: usize) -> String {
    format!(
        "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-alpha\n+beta{writer}\n"
    )
}

/// G1 — eight real processes race on the same `--agent/--task/--base/--key`
/// with different patches, fifteen times. Exactly one may be `ADMITTED`, every
/// other must be a named `IDEMPOTENCY_CONFLICT`, and the bytes on disk must
/// still hash to the winner's recorded `content_digest`.
#[test]
fn g1_concurrent_proposals_same_key_have_exactly_one_winner() {
    let repo = new_repo("g1");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    let base = git(&repo.path, &["rev-parse", "HEAD"]);
    let patches = repo.path.join("g1-patches");
    fs::create_dir_all(&patches).unwrap();
    let mut patch_paths = Vec::new();
    for writer in 0..8 {
        let path = patches.join(format!("p{writer}.patch"));
        fs::write(&path, g1_patch(writer)).unwrap();
        patch_paths.push(path);
    }

    for round in 0..15 {
        let key = format!("g1-key-{round}");
        let children: Vec<Child> = patch_paths
            .iter()
            .map(|patch| {
                Command::new(env!("CARGO_BIN_EXE_flow"))
                    .current_dir(&repo.path)
                    .args([
                        "propose",
                        "--agent",
                        "agent-a",
                        "--task",
                        "T1",
                        "--base",
                        &base,
                        "--key",
                        &key,
                        "--patch",
                        patch.to_str().unwrap(),
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("propose child spawns")
            })
            .collect();
        let outputs = wait_all(children);

        let mut admitted = 0;
        let mut conflicts = 0;
        let mut winner = None;
        for (index, output) in outputs.iter().enumerate() {
            let value = parse_output(&format!("g1[{round}][{index}]"), output);
            match status_of(&value).as_str() {
                "ADMITTED" => {
                    admitted += 1;
                    let idempotent = value.get("idempotent").and_then(Json::as_bool);
                    assert_ne!(
                        idempotent,
                        Some(true),
                        "a distinct patch may not be reported idempotent: {value}"
                    );
                    winner = Some(value);
                }
                "REFUSED" => {
                    assert_eq!(
                        code_of(&value),
                        "IDEMPOTENCY_CONFLICT",
                        "a losing writer must be a named conflict: {value}"
                    );
                    conflicts += 1;
                }
                other => panic!("unexpected proposal status {other:?}: {value}"),
            }
        }
        assert_eq!(admitted, 1, "round {round}: exactly one winner");
        assert_eq!(conflicts, 7, "round {round}: every other writer conflicts");

        // The winner's bytes are on disk under their digest, and a second read
        // of the same key is idempotent for exactly that content.
        let winner = winner.expect("one winner");
        let digest = winner
            .get("content_digest")
            .and_then(Json::as_str)
            .expect("winner digest");
        let blob = repo
            .path
            .join(".flow")
            .join("blobs")
            .join(&digest[..2])
            .join(digest);
        let bytes = fs::read(&blob).expect("winner blob exists");
        assert_eq!(
            sha256::sha256_hex(&bytes),
            digest,
            "round {round}: the stored bytes must hash to the recorded digest"
        );
        let winner_patch = winner
            .get("patch_blob")
            .and_then(Json::as_str)
            .expect("winner patch_blob");
        assert_eq!(
            winner_patch,
            format!(".flow/blobs/{}/{}", &digest[..2], digest),
            "round {round}: the record must point at the content-addressed blob"
        );

        if round == 0 {
            // Re-submitting the winner's exact bytes is idempotent, not a new
            // admission and not a conflict.
            let winning_writer = (0..8)
                .find(|w| sha256::sha256_hex(g1_patch(*w).as_bytes()) == digest)
                .expect("the winner is one of the eight writers");
            let (code, again) = flow(
                &repo.path,
                &[
                    "propose",
                    "--agent",
                    "agent-a",
                    "--task",
                    "T1",
                    "--base",
                    &base,
                    "--key",
                    &key,
                    "--patch",
                    patch_paths[winning_writer].to_str().unwrap(),
                ],
            );
            assert_eq!(code, 0, "same bytes must be idempotent: {again}");
            assert_eq!(
                again.get("idempotent").and_then(Json::as_bool),
                Some(true),
                "{again}"
            );
        }
    }
}

/// The host token the product writes: `sha256("flow-host/1\n<name>\n<os>")[..16]`.
/// A test that rewrites a lock record must speak the same dialect.
fn local_host_token() -> String {
    host::local_host_id().expect("the test host has an identity")
}

/// G3 — a provably dead owner's lock and eight real `publish` processes: exactly
/// one publication, every loser refused by name, one intent/outcome pair, and no
/// lock artifact left behind. The steal is serialized by a per-generation marker,
/// so no two publishers can both believe they own the critical section.
#[test]
fn g3_dead_lock_recovery_with_eight_publishers_keeps_one_holder() {
    let (repo, tree, head_before) = prepared_repo("g3-steal");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "precondition: run must pass: {out}");
    assert_eq!(status_of(&out), "PASSED");

    let pid = spawn_and_reap_short_lived();
    match pid_is_dead(pid) {
        Some(true) => {}
        other => {
            println!(
                "SKIP g3_dead_lock_recovery_with_eight_publishers_keeps_one_holder: cannot prove pid {pid} is dead (liveness probe said {other:?})"
            );
            return;
        }
    }
    let lock = repo.path.join(".flow").join("lock");
    fs::write(
        &lock,
        format!(
            "flow-lock/2 {pid} 0 canary-g3-nonce publish host:{}\n",
            local_host_token()
        ),
    )
    .unwrap();

    let children: Vec<Child> = (0..8)
        .map(|_| spawn_flow(&repo.path, &["publish"]))
        .collect();
    let outputs = wait_all(children);
    assert_eq!(outputs.len(), 8);

    let mut published = 0;
    for (index, output) in outputs.iter().enumerate() {
        let value = parse_output(&format!("g3[{index}]"), output);
        let exit = output.status.code().unwrap_or(-1);
        match status_of(&value).as_str() {
            "PUBLISHED" => published += 1,
            "REFUSED" => {
                assert_ne!(exit, 0, "a refusal exits nonzero: {value}");
                let code = code_of(&value);
                assert!(
                    code == "LOCKED" || code == "ALREADY_PUBLISHED",
                    "a loser must be a named refusal, got {code:?}: {value}"
                );
            }
            other => panic!("unexpected publish status {other:?}: {value}"),
        }
    }
    assert_eq!(published, 1, "exactly one recovered writer may publish");

    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    assert_ne!(head, head_before, "the recovered writer must publish");
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD^{tree}"]), tree);

    // One intent, one outcome: a double holder would interleave or duplicate
    // these read-modify-write appends.
    let records = journal_lines(&repo.path);
    assert_eq!(count_event(&records, "intent"), 1, "{records:?}");
    assert_eq!(count_event(&records, "outcome"), 1, "{records:?}");

    assert!(!lock.exists(), "the recovered lock must be released");
    let leftovers: Vec<String> = fs::read_dir(repo.path.join(".flow"))
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.starts_with("lock"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "no lock or steal artifact may survive: {leftovers:?}"
    );
}

/// A lock record written by another host is never stolen, even when its PID is
/// provably dead on this machine: the PID spaces are not comparable.
#[test]
fn a_foreign_host_lock_is_never_stolen() {
    let (repo, tree, head_before) = prepared_repo("foreign-lock");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "precondition: run must pass: {out}");
    assert!(receipt_for(&repo.path, &tree).is_file());

    let pid = spawn_and_reap_short_lived();
    match pid_is_dead(pid) {
        Some(true) => {}
        other => {
            println!(
                "SKIP a_foreign_host_lock_is_never_stolen: cannot prove pid {pid} is dead (liveness probe said {other:?})"
            );
            return;
        }
    }
    let lock = repo.path.join(".flow").join("lock");
    let record = format!("flow-lock/2 {pid} 0 foreign-nonce publish host:0000000000000000\n");
    fs::write(&lock, &record).unwrap();

    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(
        code_of(&out),
        "LOCK_UNKNOWN",
        "a foreign lock is unattributable, not dead: {out}"
    );
    assert_eq!(
        out.get("manual_required").and_then(Json::as_bool),
        Some(true)
    );
    assert_eq!(
        fs::read_to_string(&lock).unwrap(),
        record,
        "a foreign lock must be left exactly in place"
    );
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
}

/// A dead lock written before the host token existed cannot be attributed to
/// this machine, so it is refused (a named LOCK_UNKNOWN) rather than stolen.
#[test]
fn a_hostless_dead_lock_is_refused_not_stolen() {
    let (repo, tree, _) = prepared_repo("hostless-lock");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "precondition: run must pass: {out}");
    assert!(receipt_for(&repo.path, &tree).is_file());

    let pid = spawn_and_reap_short_lived();
    match pid_is_dead(pid) {
        Some(true) => {}
        other => {
            println!(
                "SKIP a_hostless_dead_lock_is_refused_not_stolen: cannot prove pid {pid} is dead (liveness probe said {other:?})"
            );
            return;
        }
    }
    let lock = repo.path.join(".flow").join("lock");
    let record = format!("flow-lock/2 {pid} 0 legacy-nonce publish\n");
    fs::write(&lock, &record).unwrap();

    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(code_of(&out), "LOCK_UNKNOWN", "{out}");
    assert_eq!(fs::read_to_string(&lock).unwrap(), record);
}

// ------------------------------------------------------------ G2 (FBL-002)

/// G2 — two concurrent `run`s on the same tree: each execution writes its own
/// log directory, the surviving receipt binds every log by sha256, and a later
/// tamper is reported instead of being accepted as evidence.
#[test]
fn g2_concurrent_runs_keep_separate_integrity_bound_evidence() {
    let (repo, tree, _) = prepared_repo("g2");
    let children: Vec<Child> = (0..2).map(|_| spawn_flow(&repo.path, &["run"])).collect();
    let outputs = wait_all(children);
    for (index, output) in outputs.iter().enumerate() {
        let value = parse_output(&format!("g2[{index}]"), output);
        assert_eq!(
            status_of(&value),
            "PASSED",
            "both runs must pass on this fixture: {value}"
        );
    }

    let logs = repo.path.join(".flow").join("logs").join(&tree);
    let run_dirs: Vec<PathBuf> = fs::read_dir(&logs)
        .expect("logs dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(
        run_dirs.len(),
        2,
        "each execution must have its own log directory: {run_dirs:?}"
    );

    let receipt_path = receipt_for(&repo.path, &tree);
    let receipt = json::parse(&fs::read_to_string(&receipt_path).expect("receipt exists"))
        .expect("receipt parses");
    let run_id = receipt
        .get("run_id")
        .and_then(Json::as_str)
        .expect("receipt records its run id");
    assert!(!run_id.is_empty());
    assert!(
        run_dirs
            .iter()
            .any(|dir| dir.file_name().and_then(|n| n.to_str()) == Some(run_id)),
        "the receipt's run directory must exist: {run_dirs:?}"
    );

    let digests = receipt
        .get("log_digests")
        .and_then(Json::as_arr)
        .expect("receipt records log digests");
    assert!(!digests.is_empty(), "at least one obligation log");
    let logs_list: Vec<String> = receipt
        .get("logs")
        .and_then(Json::as_arr)
        .expect("receipt records logs")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    for entry in digests {
        let id = entry.get("id").and_then(Json::as_str).expect("log id");
        for (suffix, key) in [("stdout", "stdout_sha256"), ("stderr", "stderr_sha256")] {
            let recorded = entry.get(key).and_then(Json::as_str).expect("log digest");
            let wanted = format!("/{id}.{suffix}.log");
            let rel = logs_list
                .iter()
                .find(|p| p.ends_with(&wanted))
                .unwrap_or_else(|| panic!("no {wanted} in {logs_list:?}"));
            let bytes = fs::read(repo.path.join(rel)).expect("log exists");
            assert_eq!(
                sha256::sha256_hex(&bytes),
                recorded,
                "the log {rel} must still hash to the receipt digest"
            );
        }
    }

    // Swap one log: explain must report incomplete evidence, never PASSED.
    let victim = logs_list
        .iter()
        .find(|p| p.ends_with(".stdout.log"))
        .expect("a stdout log");
    fs::write(repo.path.join(victim), b"tampered evidence\n").unwrap();
    let (code, explain) = flow(&repo.path, &["explain", "--tree", &tree]);
    assert_eq!(code, 0, "{explain}");
    assert_eq!(
        explain.get("log_integrity").and_then(Json::as_str),
        Some("LOG_INTEGRITY_INVALID"),
        "{explain}"
    );
    assert_eq!(
        explain.get("evidence_complete").and_then(Json::as_bool),
        Some(false),
        "{explain}"
    );
}

// ------------------------------------------------------------ G4 (FBL-004)

/// G4 — an allowlisted environment change between `run` and `publish` makes the
/// receipt stale: the publication is refused with CONTEXT_MISMATCH and the ref
/// does not move.
#[test]
fn g4_environment_change_between_run_and_publish_refuses_context_mismatch() {
    let (repo, tree, head_before) = prepared_repo("g4");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "precondition run: {out}");
    assert_eq!(status_of(&out), "PASSED");

    let separator = if cfg!(windows) { ';' } else { ':' };
    let changed_path = format!(
        "{}{separator}{}",
        std::env::var("PATH").unwrap_or_default(),
        repo.path.display()
    );
    let (code, out) = flow_with_env(&repo.path, &["publish"], &[("PATH", &changed_path)]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(
        code_of(&out),
        "CONTEXT_MISMATCH",
        "a changed build environment must refuse the receipt: {out}"
    );
    assert_eq!(
        git(&repo.path, &["rev-parse", "HEAD"]),
        head_before,
        "a refused publication must not move the ref"
    );
    assert!(
        receipt_for(&repo.path, &tree).is_file(),
        "the run receipt stays on disk as a record"
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
