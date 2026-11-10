//! Error-path contracts: what Flow *refuses*, and what it leaves untouched.
//!
//! Every test here drives the shipped binary the way an agent or an operator
//! would (real Git, real files, real exit codes) and asserts an observable
//! contract: a refusal code, a non-zero exit status, a record that was not
//! written, a pre-existing file that was not consumed. None of them executes
//! code only to colour a coverage report.
//!
//! Scope:
//! * `proposal.rs` — admission refusals (identity, base, patch bounds, symlink
//!   escape, gitlink, case collision, idempotency) and assembly refusals
//!   (duplicate/incompatible operations, corrupt or vanished records, candidate
//!   immutability).
//! * `scratch.rs` — a scratch base the host environment redirects into the
//!   repository, and a checkout whose `.git` link a hook removed: both are
//!   refused before an obligation can run on a tree Flow cannot account for.
//! * usage/version contracts that must not touch the repository.
//!
//! Fixtures live under the system temp directory and are removed on drop. The
//! tests are cross-platform: `git`, the `flow` binary and `std` only, no shell
//! command of the host, no `#[ignore]`.

#[path = "../src/json.rs"]
#[allow(dead_code)]
mod json;
// The test crate seals a tampered receipt exactly as `state::seal_receipt` does:
// the same SHA-256 over the same compact serialization. It is a helper for
// building persisted state, not a replacement for the code under test — the
// publish checks run in the shipped binary either way.
#[path = "../src/sha256.rs"]
#[allow(dead_code)]
mod sha256;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use json::Json;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// ------------------------------------------------------------------ fixtures

struct Repo {
    path: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = remove_tree(&self.path);
    }
}

/// Remove a fixture, clearing the read-only bits Git may set on its objects.
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

fn unique(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("flow-err-{tag}-{}-{n}", std::process::id()))
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

fn git_raw(repo: &Path, args: &[&str]) -> Vec<u8> {
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
    out.stdout
}

// ------------------------------------------------------------ flow invocation

fn flow_raw(repo: &Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flow"));
    cmd.current_dir(repo).args(args);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let out = cmd.output().expect("flow runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn flow_with_env(repo: &Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, Json) {
    let (code, stdout, stderr) = flow_raw(repo, args, envs);
    let parsed = if stdout.trim().is_empty() {
        Json::Null
    } else {
        json::parse(&stdout)
            .unwrap_or_else(|e| panic!("flow {args:?} printed non-JSON ({e}):\n{stdout}\n{stderr}"))
    };
    (code, parsed)
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

fn code_of(value: &Json) -> String {
    value
        .get("code")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

fn field(value: &Json, key: &str) -> String {
    value
        .get(key)
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string()
}

fn error_of(value: &Json) -> String {
    value
        .get("error")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string()
}

/// A refusal is a JSON document with status REFUSED and the exact code the
/// contract names; a bare exit status would not be a contract an agent can act
/// on.
fn assert_refused(out: &Json, expected_code: &str) {
    assert_eq!(status_of(out), "REFUSED", "expected a refusal, got: {out}");
    assert_eq!(code_of(out), expected_code, "refusal code: {out}");
}

fn assert_error_mentions(out: &Json, needle: &str) {
    assert_eq!(status_of(out), "ERROR", "expected an error, got: {out}");
    let message = error_of(out);
    assert!(
        message.contains(needle),
        "error must mention {needle:?}, got: {message}"
    );
}

/// A disposable Git repository with a seeded base commit.
fn new_repo(tag: &str) -> Repo {
    let path = unique(tag);
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Error Path Test"]);
    git(
        &path,
        &["config", "user.email", "flow-error@example.invalid"],
    );
    git(&path, &["config", "commit.gpgsign", "false"]);
    fs::write(path.join("alpha.txt"), "one\ntwo\nthree\n").unwrap();
    fs::write(path.join("shared.txt"), "red\ngreen\nblue\n").unwrap();
    git(&path, &["add", "-A"]);
    git(&path, &["commit", "-q", "-m", "base"]);
    Repo { path }
}

fn head(repo: &Path) -> String {
    git(repo, &["rev-parse", "HEAD"])
}

/// Build a binary patch by editing a throw-away worktree: exactly what an
/// independent agent would hand to `flow propose`.
fn make_patch(repo: &Path, base: &str, edits: &[(&str, Option<&str>)]) -> PathBuf {
    let work = unique("agent");
    let _ = remove_tree(&work);
    git(
        repo,
        &[
            "worktree",
            "add",
            "--detach",
            "-q",
            &work.to_string_lossy(),
            base,
        ],
    );
    for (name, content) in edits {
        let target = work.join(name);
        match content {
            Some(text) => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&target, text).unwrap();
            }
            None => {
                let _ = fs::remove_file(&target);
            }
        }
    }
    git(&work, &["add", "-A"]);
    let patch = git_raw(&work, &["diff", "--cached", "--binary", "--no-color", base]);
    let patch_path = unique("patch").with_extension("patch");
    fs::write(&patch_path, patch).unwrap();
    git(
        repo,
        &["worktree", "remove", "--force", &work.to_string_lossy()],
    );
    let _ = remove_tree(&work);
    patch_path
}

fn write_patch(tag: &str, text: &str) -> PathBuf {
    let path = unique(tag).with_extension("patch");
    fs::write(&path, text).unwrap();
    path
}

/// A hand-written patch that adds one file with an explicit Git mode and body.
///
/// Hand-written on purpose: the interesting modes (120000 symlink, 160000
/// gitlink) cannot be produced through a worktree on a host whose filesystem
/// does not support them, but `git apply --cached` writes them to the index on
/// every supported host. The index line is not verified for a new file.
fn new_file_patch(mode: &str, path: &str, body: &str) -> String {
    format!(
        "diff --git a/{path} b/{path}\nnew file mode {mode}\nindex 0000000..1111111\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1 @@\n+{body}\n"
    )
}

fn propose(
    repo: &Path,
    agent: &str,
    task: &str,
    base: &str,
    patch: &Path,
    key: Option<&str>,
) -> (i32, Json) {
    let patch_text = patch.to_string_lossy().to_string();
    let mut args: Vec<&str> = vec![
        "propose",
        "--agent",
        agent,
        "--task",
        task,
        "--base",
        base,
        "--patch",
        &patch_text,
    ];
    if let Some(key) = key {
        args.push("--key");
        args.push(key);
    }
    flow(repo, &args)
}

fn assembled(repo: &Path, ids: &[&str]) -> (i32, Json) {
    let mut args: Vec<&str> = vec!["assemble"];
    for id in ids {
        args.push("--proposal");
        args.push(id);
    }
    flow(repo, &args)
}

fn proposal_id(value: &Json) -> String {
    field(value, "proposal_id")
}

fn candidate_file(repo: &Path, id: &str) -> PathBuf {
    repo.join(".flow")
        .join("candidates")
        .join(format!("{id}.json"))
}

// ================================================================ proposals
//
// `flow proposals list` / `show` are the read side of the proposal store. A
// corrupt record must be an error, never silently skipped: a list that hides a
// record is a list an agent would act on without seeing it.

#[test]
fn proposals_list_sorts_and_ignores_files_that_are_not_records() {
    let repo = new_repo("list-sorted");
    let base = head(&repo.path);
    let first = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let second = make_patch(&repo.path, &base, &[("two.txt", Some("2\n"))]);
    let (_, a) = propose(&repo.path, "agent-a", "T1", &base, &first, Some("key-one"));
    let (_, b) = propose(&repo.path, "agent-b", "T2", &base, &second, Some("key-two"));
    assert_eq!(status_of(&a), "ADMITTED", "{a}");
    assert_eq!(status_of(&b), "ADMITTED", "{b}");

    // A foreign file in the directory is not a proposal and must be ignored.
    let proposals_dir = repo.path.join(".flow").join("proposals");
    fs::write(proposals_dir.join("notes.txt"), "not a proposal\n").unwrap();

    let (code, out) = flow(&repo.path, &["proposals", "list"]);
    assert_eq!(code, 0, "a readable store lists successfully: {out}");
    assert_eq!(status_of(&out), "RECORD");
    let listed = out
        .get("proposals")
        .and_then(Json::as_arr)
        .expect("proposals array");
    assert_eq!(listed.len(), 2, "only `.json` proposal records are listed");
    let ids: Vec<String> = listed.iter().map(proposal_id).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "the listing is ordered by proposal id");
    assert_eq!(
        out.get("count").and_then(Json::as_num),
        Some(2.0),
        "count matches the listing"
    );
}

#[test]
fn proposals_list_refuses_a_corrupt_record_instead_of_hiding_it() {
    let repo = new_repo("list-corrupt");
    let base = head(&repo.path);
    let patch = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let (_, admitted) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("key"));
    assert_eq!(status_of(&admitted), "ADMITTED", "{admitted}");

    // A record id is a 32-character hex name; the body is not JSON at all.
    let corrupt = repo
        .path
        .join(".flow")
        .join("proposals")
        .join(format!("{}.json", "a".repeat(32)));
    fs::write(&corrupt, "{ this is not json").unwrap();

    let (code, out) = flow(&repo.path, &["proposals", "list"]);
    assert_eq!(
        code, 1,
        "a corrupt record must not be reported as a listing"
    );
    assert_error_mentions(&out, "cannot load proposal");
}

#[test]
fn proposals_show_refuses_an_unknown_or_malformed_id() {
    let repo = new_repo("show");
    let unknown = "b".repeat(32);
    let (code, out) = flow(&repo.path, &["proposals", "show", &unknown]);
    assert_eq!(code, 1);
    assert_refused(&out, "UNKNOWN_PROPOSAL");

    let (code, out) = flow(&repo.path, &["proposals", "show", "not-a-record-id"]);
    assert_eq!(code, 1);
    assert_error_mentions(&out, "INVALID_RECORD_ID");
}

#[test]
fn propose_refuses_invalid_identity_without_recording_anything() {
    let repo = new_repo("ident");
    let base = head(&repo.path);
    let patch = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let long = "a".repeat(129);

    let cases: &[(&str, &str, Option<&str>)] = &[
        ("", "T1", None),
        ("..", "T1", None),
        ("bad/agent", "T1", None),
        (long.as_str(), "T1", None),
        ("agent-a", "bad task", None),
        ("agent-a", "T1", Some("..")),
        ("agent-a", "T1", Some("bad key")),
    ];
    for (agent, task, key) in cases {
        let (code, out) = propose(&repo.path, agent, task, &base, &patch, *key);
        assert_eq!(
            code, 1,
            "invalid identity ({agent:?}, {task:?}, {key:?}) must be refused: {out}"
        );
        assert_refused(&out, "INVALID_IDENTITY");
    }

    let proposals = repo.path.join(".flow").join("proposals");
    let recorded = fs::read_dir(&proposals)
        .map(|entries| entries.flatten().count())
        .unwrap_or(0);
    assert_eq!(
        recorded, 0,
        "a refused identity must not leave a proposal record behind"
    );
}

#[test]
fn propose_refuses_a_base_that_is_not_a_commit_in_this_repository() {
    let repo = new_repo("base");
    let patch = make_patch(&repo.path, &head(&repo.path), &[("one.txt", Some("1\n"))]);

    let (code, out) = propose(
        &repo.path,
        "agent-a",
        "T1",
        "not-an-oid",
        &patch,
        Some("k1"),
    );
    assert_eq!(code, 1);
    assert_refused(&out, "BASE_INVALID");

    // Syntactically a 40-hex OID, but this object store never held it: the
    // refusal must happen at admission, not as a late Git error.
    let ghost = "c".repeat(40);
    let (code, out) = propose(&repo.path, "agent-a", "T1", &ghost, &patch, Some("k2"));
    assert_eq!(code, 1);
    assert_refused(&out, "BASE_INVALID");
    assert!(
        field(&out, "reason").contains("not a commit"),
        "the reason must say the base is not a commit here: {out}"
    );
}

#[test]
fn propose_refuses_an_unreadable_patch_or_a_directory() {
    let repo = new_repo("patch-read");
    let base = head(&repo.path);

    let missing = unique("missing").with_extension("patch");
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &missing, Some("k1"));
    assert_eq!(code, 1);
    assert_refused(&out, "PATCH_UNREADABLE");

    // A directory stats successfully and then cannot be read as bytes: the
    // refusal must be a named code, not a panic or a generic error.
    let dir = unique("patch-dir");
    fs::create_dir_all(&dir).unwrap();
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &dir, Some("k2"));
    assert_eq!(code, 1);
    assert_refused(&out, "PATCH_UNREADABLE");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn propose_refuses_a_patch_above_the_admission_bound_before_reading_it() {
    let repo = new_repo("patch-large");
    let base = head(&repo.path);
    let large = unique("large").with_extension("patch");
    // Sparse on purpose: the bound is on the declared size, so the test must not
    // need 32 MiB of real I/O to exercise it.
    let file = fs::File::create(&large).unwrap();
    file.set_len(32 * 1024 * 1024 + 1).unwrap();
    drop(file);

    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &large, Some("k"));
    assert_eq!(code, 1);
    assert_refused(&out, "PATCH_TOO_LARGE");
    assert!(
        !repo.path.join(".flow").join("proposals").exists(),
        "an over-bound patch must not create a proposal store"
    );
    let _ = fs::remove_file(&large);
}

#[test]
fn propose_refuses_a_symlink_that_escapes_the_tree() {
    let repo = new_repo("symlink-escape");
    let base = head(&repo.path);

    // Every absolute/escaping shape the contract names: Unix-absolute, host
    // separator-absolute, drive-absolute, and a parent-relative target.
    for (tag, target) in [
        ("unix-abs", "/outside"),
        ("win-abs", "\\\\outside"),
        ("drive-abs", "C:/outside"),
        ("parent", "../outside"),
    ] {
        let patch = write_patch(tag, &new_file_patch("120000", "link", target));
        let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some(tag));
        assert_eq!(code, 1, "{tag}: {out}");
        assert_refused(&out, "SYMLINK_ESCAPE");
        assert!(
            field(&out, "reason").contains("symlink"),
            "{tag}: the refusal must name the symlink: {out}"
        );
    }

    // Control: a relative target that stays inside the tree is admitted, so the
    // refusals above are not "a symlink is always refused".
    let patch = write_patch("safe", &new_file_patch("120000", "link", "sibling.txt"));
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("safe"));
    assert_eq!(
        code, 0,
        "a non-escaping symlink is a proposal like any other: {out}"
    );
    assert_eq!(status_of(&out), "ADMITTED");
    let modes: Vec<String> = out
        .get("changes")
        .and_then(Json::as_arr)
        .expect("changes")
        .iter()
        .map(|c| field(c, "new_mode"))
        .collect();
    assert!(
        modes.iter().any(|m| m == "120000"),
        "the admitted change is recorded as a symlink: {out}"
    );
}

#[test]
fn propose_refuses_a_symlink_target_above_the_read_bound() {
    let repo = new_repo("symlink-long");
    let base = head(&repo.path);
    let target = "a".repeat(4097);
    let patch = write_patch("long-target", &new_file_patch("120000", "link", &target));
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    assert_eq!(code, 1);
    assert_refused(&out, "SYMLINK_UNREADABLE");
    assert!(
        field(&out, "reason").contains("4096"),
        "the refusal must name the bound: {out}"
    );
}

#[test]
fn propose_refuses_a_tree_whose_paths_collide_only_by_case() {
    let repo = new_repo("case");
    let base = head(&repo.path);
    // One patch adds both spellings to the temporary index: Git accepts them,
    // a case-insensitive host cannot materialise both, so admission refuses.
    let patch_text = format!(
        "{}{}",
        new_file_patch("100644", "Collide.txt", "upper"),
        new_file_patch("100644", "collide.txt", "lower")
    );
    let patch = write_patch("case-collide", &patch_text);
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    assert_eq!(code, 1);
    assert_refused(&out, "CASE_COLLISION");
    assert!(
        !repo.path.join(".flow").join("proposals").exists(),
        "a case-colliding tree must not be recorded"
    );
}

#[test]
fn propose_refuses_a_gitlink() {
    let repo = new_repo("gitlink");
    let base = head(&repo.path);
    let body = "Subproject commit 1111111111111111111111111111111111111111";
    let patch = write_patch("gitlink", &new_file_patch("160000", "sub", body));
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    assert_eq!(code, 1);
    assert_refused(&out, "GITLINK_UNSUPPORTED");
}

#[test]
fn propose_without_an_explicit_key_is_idempotent_on_the_same_bytes() {
    let repo = new_repo("no-key");
    let base = head(&repo.path);
    let patch = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);

    let (code, first) = propose(&repo.path, "agent-a", "T1", &base, &patch, None);
    assert_eq!(code, 0, "{first}");
    assert_eq!(status_of(&first), "ADMITTED");

    let (code, second) = propose(&repo.path, "agent-a", "T1", &base, &patch, None);
    assert_eq!(
        code, 0,
        "resubmitting the same bytes is idempotent: {second}"
    );
    assert_eq!(
        proposal_id(&first),
        proposal_id(&second),
        "content is the key when no key is given"
    );
    assert_eq!(
        second.get("idempotent").and_then(Json::as_bool),
        Some(true),
        "the second admission is reported as idempotent: {second}"
    );
}

// ================================================================= assembly

#[test]
fn assemble_without_a_proposal_is_refused() {
    let repo = new_repo("assemble-empty");
    let (code, out) = flow(&repo.path, &["assemble"]);
    assert_eq!(code, 1);
    assert_refused(&out, "NO_PROPOSAL");
}

#[test]
fn assemble_refuses_a_delete_and_a_modify_of_the_same_path() {
    let repo = new_repo("assemble-delete-modify");
    let base = head(&repo.path);
    let deleting = make_patch(&repo.path, &base, &[("shared.txt", None)]);
    let modifying = make_patch(
        &repo.path,
        &base,
        &[("shared.txt", Some("red\ngreen\nBLUE\n"))],
    );
    let (_, a) = propose(&repo.path, "agent-a", "T1", &base, &deleting, Some("a"));
    let (_, b) = propose(&repo.path, "agent-b", "T2", &base, &modifying, Some("b"));
    assert_eq!(status_of(&a), "ADMITTED", "{a}");
    assert_eq!(status_of(&b), "ADMITTED", "{b}");

    let (code, out) = assembled(&repo.path, &[&proposal_id(&a), &proposal_id(&b)]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "PATH_OPERATION_CONFLICT");
    let reason = field(&out, "reason");
    assert!(
        reason.contains(&proposal_id(&a)) || reason.contains(&proposal_id(&b)),
        "the conflict must name the proposals it refuses to merge: {reason}"
    );
    assert!(
        !repo.path.join(".flow").join("candidates").exists(),
        "a refused assembly must not write a candidate record"
    );
}

#[test]
fn assemble_refuses_two_different_modes_for_the_same_path() {
    let repo = new_repo("assemble-mode");
    let base = head(&repo.path);
    let content = make_patch(
        &repo.path,
        &base,
        &[("alpha.txt", Some("ONE\ntwo\nthree\n"))],
    );
    // Mode only: `git apply --cached` records the executable bit in the index on
    // every host, including one whose worktree does not track modes.
    let mode = write_patch(
        "mode",
        "diff --git a/alpha.txt b/alpha.txt\nold mode 100644\nnew mode 100755\n",
    );
    let (_, a) = propose(&repo.path, "agent-a", "T1", &base, &content, Some("a"));
    let (_, b) = propose(&repo.path, "agent-b", "T2", &base, &mode, Some("b"));
    assert_eq!(status_of(&a), "ADMITTED", "{a}");
    assert_eq!(status_of(&b), "ADMITTED", "{b}");

    let (code, out) = assembled(&repo.path, &[&proposal_id(&a), &proposal_id(&b)]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "PATH_OPERATION_CONFLICT");
    assert!(
        field(&out, "reason").contains("mode"),
        "the diagnostic must say the modes differ: {out}"
    );
}

#[test]
fn assemble_refuses_when_a_recorded_patch_vanished() {
    let repo = new_repo("assemble-vanished");
    let base = head(&repo.path);
    let patch = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let (_, admitted) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    assert_eq!(status_of(&admitted), "ADMITTED", "{admitted}");
    let id = proposal_id(&admitted);
    fs::remove_file(
        repo.path
            .join(".flow")
            .join("proposals")
            .join(format!("{id}.patch")),
    )
    .expect("remove the recorded patch");

    let (code, out) = assembled(&repo.path, &[&id]);
    assert_eq!(code, 1);
    assert_error_mentions(&out, "patch is unreadable");
    assert!(
        !repo.path.join(".flow").join("candidates").exists(),
        "a vanished patch must not produce a candidate"
    );
}

#[test]
fn assemble_refuses_to_reinterpret_a_candidate_record_from_another_schema() {
    let repo = new_repo("candidate-schema");
    let base = head(&repo.path);
    let patch = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let (_, admitted) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    let id = proposal_id(&admitted);
    let (code, candidate) = assembled(&repo.path, &[&id]);
    assert_eq!(code, 0, "the fixture candidate assembles: {candidate}");
    let candidate_id = field(&candidate, "candidate_id");

    let path = candidate_file(&repo.path, &candidate_id);
    let original = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        original.replace("flow-candidate/1", "flow-candidate/0"),
    )
    .unwrap();
    let tampered = fs::read_to_string(&path).unwrap();

    let (code, out) = assembled(&repo.path, &[&id]);
    assert_eq!(code, 1);
    assert_error_mentions(&out, "refusing to reinterpret");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        tampered,
        "a refused load must not rewrite the record it refused"
    );
}

#[test]
fn assemble_refuses_to_overwrite_an_immutable_candidate() {
    let repo = new_repo("candidate-immutable");
    let base = head(&repo.path);
    let patch = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let (_, admitted) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    let id = proposal_id(&admitted);
    let (code, candidate) = assembled(&repo.path, &[&id]);
    assert_eq!(code, 0, "{candidate}");
    let candidate_id = field(&candidate, "candidate_id");
    let tree = field(&candidate, "tree_oid");

    let path = candidate_file(&repo.path, &candidate_id);
    let original = fs::read_to_string(&path).unwrap();
    let zeros = "0".repeat(tree.len());
    fs::write(&path, original.replacen(&tree, &zeros, 1)).unwrap();
    let tampered = fs::read_to_string(&path).unwrap();

    let (code, out) = assembled(&repo.path, &[&id]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "CANDIDATE_IMMUTABLE");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        tampered,
        "the existing candidate must be left exactly as it was"
    );
}

#[test]
fn assemble_reports_a_patch_that_does_not_apply_without_writing_a_candidate() {
    let repo = new_repo("assemble-apply");
    let base = head(&repo.path);
    let first = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let second = make_patch(&repo.path, &base, &[("two.txt", Some("2\n"))]);
    let (_, a) = propose(&repo.path, "agent-a", "T1", &base, &first, Some("a"));
    let (_, b) = propose(&repo.path, "agent-b", "T2", &base, &second, Some("b"));
    let (ida, idb) = (proposal_id(&a), proposal_id(&b));

    // A recorded patch that no longer matches its record: applying it after the
    // first proposal asks Git to add a path that already exists.
    let first_bytes = fs::read(
        repo.path
            .join(".flow")
            .join("proposals")
            .join(format!("{ida}.patch")),
    )
    .unwrap();
    fs::write(
        repo.path
            .join(".flow")
            .join("proposals")
            .join(format!("{idb}.patch")),
        &first_bytes,
    )
    .unwrap();
    let (code, out) = assembled(&repo.path, &[&ida, &idb]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "CONFLICT");
    assert!(
        field(&out, "reason").contains("PATH_CONFLICT"),
        "the conflict must be classified, not reported as a generic refusal: {out}"
    );

    // Garbage in the recorded slot has no known conflict shape: it is still a
    // refusal, classified as APPLY_REFUSED, and never a candidate.
    fs::write(
        repo.path
            .join(".flow")
            .join("proposals")
            .join(format!("{idb}.patch")),
        b"this is not a patch\n",
    )
    .unwrap();
    let (code, out) = assembled(&repo.path, &[&ida, &idb]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "CONFLICT");
    assert!(
        field(&out, "reason").contains("APPLY_REFUSED"),
        "an unclassifiable apply failure is APPLY_REFUSED: {out}"
    );
    assert!(
        !repo.path.join(".flow").join("candidates").exists(),
        "no candidate is written when the patches do not apply"
    );
}

#[test]
fn assemble_refuses_a_candidate_tree_whose_paths_collide_only_by_case() {
    let repo = new_repo("assemble-case");
    let base = head(&repo.path);
    let upper = write_patch("upper", &new_file_patch("100644", "Clash.txt", "upper"));
    let lower = write_patch("lower", &new_file_patch("100644", "clash.txt", "lower"));
    let (_, a) = propose(&repo.path, "agent-a", "T1", &base, &upper, Some("a"));
    let (_, b) = propose(&repo.path, "agent-b", "T2", &base, &lower, Some("b"));
    assert_eq!(status_of(&a), "ADMITTED", "{a}");
    assert_eq!(status_of(&b), "ADMITTED", "{b}");

    let (code, out) = assembled(&repo.path, &[&proposal_id(&a), &proposal_id(&b)]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "CASE_COLLISION");
    assert!(
        !repo.path.join(".flow").join("candidates").exists(),
        "a case-colliding candidate is diagnosed, never recorded"
    );
}

// ==================================================================== usage
//
// Usage errors are exit 2 and must not touch the repository: an operator who
// mistyped a flag has not asked for state to be created.

#[test]
fn usage_and_version_contracts_do_not_touch_the_repository() {
    let repo = new_repo("usage");
    let (code, stdout, stderr) = flow_raw(&repo.path, &[], &[]);
    assert_eq!(code, 2, "no command is a usage error");
    assert!(
        stdout.contains("PROPOSALS") && stdout.contains("propose --agent"),
        "the usage text is printed: {stdout} / {stderr}"
    );

    let (code, stdout, _) = flow_raw(&repo.path, &["help"], &[]);
    assert_eq!(code, 0, "`flow help` is not an error");
    assert!(stdout.contains("VERIFICATION AND PUBLICATION"));

    let (code, version) = flow(&repo.path, &["version"]);
    assert_eq!(code, 0, "{version}");
    assert_eq!(status_of(&version), "VERSION");
    assert!(field(&version, "tool").starts_with("flow "));
    assert!(
        field(&version, "schemas").contains("flow-receipt/2"),
        "version publishes every schema: {version}"
    );

    let (code, out) = flow(&repo.path, &["definitely-not-a-command"]);
    assert_eq!(code, 2);
    assert_error_mentions(&out, "unknown command");

    // A value flag without a value, an unparseable timeout, an unknown inline
    // value and an unknown option are all usage errors before any mutation.
    for args in [
        vec!["propose", "--agent"],
        vec!["run", "--timeout", "0"],
        vec!["publish", "--dry-run=true"],
        vec!["run", "--not-a-flag"],
    ] {
        let (code, out) = flow(&repo.path, &args);
        assert_eq!(code, 2, "{args:?} must be a usage error: {out}");
        assert_eq!(status_of(&out), "ERROR", "{args:?}: {out}");
    }
    for args in [vec!["proposals", "show"], vec!["proposals", "bogus"]] {
        let (code, out) = flow(&repo.path, &args);
        assert_eq!(code, 2, "{args:?} must be a usage error: {out}");
        assert_eq!(status_of(&out), "ERROR", "{args:?}: {out}");
    }

    assert!(
        !repo.path.join(".flow").exists(),
        "usage errors must not create Flow state"
    );
    assert_eq!(
        git(&repo.path, &["status", "--porcelain"]),
        "",
        "usage errors must leave the working tree clean"
    );
}

#[test]
fn commands_that_need_a_repository_fail_outside_one() {
    let dir = unique("no-repo");
    let _ = remove_tree(&dir);
    fs::create_dir_all(&dir).unwrap();
    let (code, out) = flow(&dir, &["status"]);
    assert_eq!(code, 1, "a repository command outside a repository fails");
    assert_error_mentions(&out, "not inside a Git work tree");
    let _ = remove_tree(&dir);
}

// ================================================================== scratch
//
// The scratch directory is created under the host temp directory. An operator
// (or a host profile) can redirect that directory into the repository; the
// refusal is the contract, and nothing may be created inside the repository.

const GREEN: &str =
    r#"{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}"#;

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

/// Initialize Flow and adopt a policy that always passes, so a run reaches the
/// scratch/layout decisions instead of stopping at policy admission.
fn prepared_repo(tag: &str) -> Repo {
    let repo = new_repo(tag);
    let (code, out) = flow(&repo.path, &["init"]);
    assert_eq!(code, 0, "flow init: {out}");
    exclude_flow(&repo.path);
    let dir = repo.path.join(".flow");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.json"),
        format!(
            r#"{{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{GREEN}],"disabled":false}}"#
        ),
    )
    .unwrap();
    let (code, out) = flow(&repo.path, &["approve"]);
    assert_eq!(code, 0, "the fixture policy must be adopted: {out}");
    repo
}

fn receipts_dir(repo: &Path) -> PathBuf {
    repo.join(".flow").join("receipts")
}

fn worktree_count(repo: &Path) -> usize {
    let listing = git(repo, &["worktree", "list", "--porcelain"]);
    listing
        .lines()
        .filter(|line| line.starts_with("worktree "))
        .count()
}

#[test]
fn run_refuses_a_scratch_base_the_host_redirected_into_the_repository() {
    let repo = prepared_repo("scratch-tmpdir");
    let inside = repo.path.join("scratch-base");
    fs::create_dir(&inside).unwrap();
    let tree = git(&repo.path, &["write-tree"]);
    let inside_text = inside.to_string_lossy().to_string();
    let envs = [
        ("TMPDIR", inside_text.as_str()),
        ("TMP", inside_text.as_str()),
        ("TEMP", inside_text.as_str()),
    ];

    let (code, out) = flow_with_env(&repo.path, &["run"], &envs);
    assert_eq!(
        code, 1,
        "a scratch base inside the repository must be refused: {out}"
    );
    assert_refused(&out, "SCRATCH_REFUSED");
    assert!(
        field(&out, "reason").contains("inside the repository"),
        "the refusal must name the boundary: {out}"
    );
    assert_eq!(
        fs::read_dir(&inside).unwrap().count(),
        0,
        "a refused scratch base must not be populated"
    );
    assert!(
        !receipts_dir(&repo.path)
            .join(format!("{tree}.json"))
            .exists(),
        "a refused scratch must not write the receipt slot"
    );
    assert_eq!(
        worktree_count(&repo.path),
        1,
        "a refused scratch must not leave a worktree registration behind"
    );
}

/// A repository can install a `post-checkout` hook that `git worktree add` runs
/// inside the disposable checkout. A hook that removes the `.git` link would
/// leave a tree that still looks materialised while the repository discovery
/// link is gone: `materialise` must fail closed, never return that directory as
/// owned, and never write a receipt.
fn install_tracked_hook(repo: &Repo, body: &str) {
    let hooks = repo.path.join(".hooks");
    fs::create_dir_all(&hooks).unwrap();
    let path = hooks.join("post-checkout");
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    git(&repo.path, &["add", "-A"]);
    git(
        &repo.path,
        &["update-index", "--chmod=+x", "--", ".hooks/post-checkout"],
    );
    git(&repo.path, &["commit", "-q", "-m", "fixture hook"]);
    git(&repo.path, &["config", "core.hooksPath", ".hooks"]);
}

#[test]
fn run_refuses_a_scratch_whose_git_link_a_hook_removed() {
    let repo = prepared_repo("scratch-hook");
    install_tracked_hook(&repo, "rm -f .git\n");
    let tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 1,
        "a checkout without its .git link must not be certified: {out}"
    );
    assert_refused(&out, "SCRATCH_REFUSED");
    assert!(
        !field(&out, "reason").is_empty(),
        "the refusal must explain what could not be materialised: {out}"
    );
    assert!(
        !receipts_dir(&repo.path)
            .join(format!("{tree}.json"))
            .exists(),
        "a refused scratch must not write the receipt slot"
    );
    assert_eq!(
        worktree_count(&repo.path),
        1,
        "the abandoned worktree must be pruned, not left registered"
    );
    assert_eq!(
        git(&repo.path, &["status", "--porcelain"]),
        "",
        "a refused run must leave the repository clean"
    );
}

// ============================================================ receipt checks
//
// A receipt is Flow's proof that a tree was verified under a policy. Publishing
// re-derives every field it trusts from the current configuration and refuses a
// receipt that no longer matches. Each case below tampers exactly one field and
// re-seals the integrity digest, so the refusal is the *semantic* check named in
// the contract, not the generic "someone edited the file" check.

fn set_str_field(value: &Json, key: &str, new_value: &str) -> Json {
    let mut out = value.clone();
    if let Json::Obj(pairs) = &mut out {
        match pairs.iter_mut().find(|(candidate, _)| candidate == key) {
            Some((_, existing)) => *existing = Json::str(new_value),
            None => pairs.push((key.to_string(), Json::str(new_value))),
        }
    }
    out
}

fn set_nested_str(value: &Json, parent: &str, child: &str, new_value: &str) -> Json {
    let mut out = value.clone();
    let nested = set_str_field(
        &value.get(parent).cloned().unwrap_or(Json::obj(vec![])),
        child,
        new_value,
    );
    if let Json::Obj(pairs) = &mut out {
        match pairs.iter_mut().find(|(candidate, _)| candidate == parent) {
            Some((_, existing)) => *existing = nested,
            None => pairs.push((parent.to_string(), nested)),
        }
    }
    out
}

fn set_nested_arr(value: &Json, parent: &str, child: &str, items: Vec<Json>) -> Json {
    let mut nested = value
        .get(parent)
        .cloned()
        .unwrap_or_else(|| Json::obj(vec![]));
    if let Json::Obj(pairs) = &mut nested {
        match pairs.iter_mut().find(|(candidate, _)| candidate == child) {
            Some((_, existing)) => *existing = Json::Arr(items),
            None => pairs.push((child.to_string(), Json::Arr(items))),
        }
    }
    let mut out = value.clone();
    if let Json::Obj(pairs) = &mut out {
        match pairs.iter_mut().find(|(candidate, _)| candidate == parent) {
            Some((_, existing)) => *existing = nested,
            None => pairs.push((parent.to_string(), nested)),
        }
    }
    out
}

fn map_obligations(value: &Json, update: impl Fn(&Json) -> Option<Json>) -> Json {
    let mut out = value.clone();
    if let Json::Obj(pairs) = &mut out {
        if let Some((_, Json::Arr(items))) = pairs
            .iter_mut()
            .find(|(candidate, _)| candidate == "obligations")
        {
            *items = items.iter().filter_map(&update).collect();
        }
    }
    out
}

/// The same sealing rule `state::seal_receipt` applies: remove `integrity`, hash
/// the compact serialization, append the digest.
fn seal(value: &Json) -> Json {
    let mut body = value.clone();
    if let Json::Obj(pairs) = &mut body {
        pairs.retain(|(key, _)| key != "integrity");
    }
    let digest = sha256::sha256_hex(body.to_string().as_bytes());
    if let Json::Obj(pairs) = &mut body {
        pairs.push(("integrity".to_string(), Json::str(digest)));
    }
    body
}

fn git_with_stdin(repo: &Path, args: &[&str], input: &[u8]) -> String {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git runs");
    child
        .stdin
        .take()
        .expect("git stdin")
        .write_all(input)
        .expect("write git stdin");
    let out = child.wait_with_output().expect("git output");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Stage one change, verify it, and hand back the repository plus the verified
/// tree and its receipt path.
fn verified_tree(tag: &str) -> (Repo, String, PathBuf) {
    let repo = prepared_repo(tag);
    fs::write(repo.path.join("alpha.txt"), "ONE\ntwo\nthree\n").unwrap();
    git(&repo.path, &["add", "alpha.txt"]);
    let tree = git(&repo.path, &["write-tree"]);
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 0, "the fixture run must pass: {out}");
    assert_eq!(status_of(&out), "PASSED", "{out}");
    let receipt = receipts_dir(&repo.path).join(format!("{tree}.json"));
    assert!(receipt.exists(), "a PASSED run writes the receipt slot");
    (repo, tree, receipt)
}

#[test]
fn publish_refuses_every_tampered_receipt_field_with_its_named_check() {
    let (repo, tree, receipt_path) = verified_tree("receipt-fields");
    let original = fs::read_to_string(&receipt_path).expect("receipt");
    let value = json::parse(&original).expect("receipt is JSON");
    let old = head(&repo.path);

    // A dangling commit whose tree is *not* the verified tree, to exercise the
    // "receipt names a commit that carries another tree" check.
    let empty_tree = git_with_stdin(&repo.path, &["mktree"], b"");
    let other_commit = git(
        &repo.path,
        &["commit-tree", &empty_tree, "-p", &old, "-m", "other tree"],
    );

    let digest_len = field(&value, "policy_digest").len();
    let ctx_len = field(&value, "context_id").len();
    let zeros_digest = "0".repeat(digest_len.max(1));
    let zeros_ctx = "0".repeat(ctx_len.max(1));
    let zeros_tree = "0".repeat(tree.len());

    let cases: Vec<(&str, Json, &str, &str)> = vec![
        (
            "schema",
            seal(&set_str_field(&value, "schema", "flow-receipt/1")),
            "RECEIPT_SCHEMA_STALE",
            "re-run",
        ),
        (
            "integrity",
            set_str_field(&value, "integrity", "deadbeef"),
            "RECEIPT_INTEGRITY",
            "integrity",
        ),
        (
            "tree name",
            seal(&set_str_field(&value, "tree_oid", &zeros_tree)),
            "TREE_MISMATCH",
            "names tree",
        ),
        (
            "selective scope",
            seal(&set_nested_str(
                &value,
                "verification_scope",
                "mode",
                "selected",
            )),
            "SCOPE_INCOMPLETE",
            "selective",
        ),
        (
            "missing required",
            seal(&set_nested_arr(
                &value,
                "verification_scope",
                "missing_required",
                vec![Json::str("green")],
            )),
            "MISSING_REQUIRED",
            "required obligations",
        ),
        (
            "policy digest",
            seal(&set_str_field(&value, "policy_digest", &zeros_digest)),
            "STALE_POLICY",
            "current policy",
        ),
        (
            "policy version",
            seal(&set_str_field(&value, "policy_version", "FLOW-POLICY-0")),
            "POLICY_VERSION_MISMATCH",
            "policy_version",
        ),
        (
            "execution boundary",
            seal(&set_str_field(&value, "execution_boundary", "untrusted")),
            "EXECUTION_BOUNDARY_MISMATCH",
            "execution_boundary",
        ),
        (
            "context",
            seal(&set_str_field(&value, "context_id", &zeros_ctx)),
            "CONTEXT_MISMATCH",
            "context",
        ),
        (
            "required obligation absent",
            seal(&map_obligations(&value, |entry| {
                if field(entry, "id") == "green" {
                    None
                } else {
                    Some(entry.clone())
                }
            })),
            "MISSING_REQUIRED",
            "absent from the receipt",
        ),
        (
            "required obligation failed",
            seal(&map_obligations(&value, |entry| {
                if field(entry, "id") == "green" {
                    Some(set_str_field(entry, "status", "FAILED"))
                } else {
                    Some(entry.clone())
                }
            })),
            "REQUIRED_NOT_PASSED",
            "status",
        ),
        (
            "receipt status",
            seal(&set_str_field(&value, "status", "FAILED")),
            "STATUS_NOT_PASSED",
            "non-PASSED",
        ),
        (
            "commit tree",
            seal(&set_str_field(&value, "commit_oid", &other_commit)),
            "RECEIPT_COMMIT_MISMATCH",
            "carries tree",
        ),
    ];

    for (name, tampered, expected_code, needle) in cases {
        fs::write(&receipt_path, tampered.to_string_pretty() + "\n").unwrap();
        let (code, out) = flow(&repo.path, &["publish", "--expected-old", &old]);
        assert_eq!(code, 1, "case {name} must not publish: {out}");
        assert_refused(&out, expected_code);
        assert!(
            field(&out, "reason").contains(needle),
            "case {name} must explain {needle:?}: {out}"
        );
    }

    // No case may have moved the branch: every refusal happens before the ref
    // update, so the repository still points at the base commit.
    assert_eq!(head(&repo.path), old, "refusals must not move the branch");
}

#[test]
fn publish_refuses_a_second_publication_of_the_same_tree() {
    let (repo, tree, receipt_path) = verified_tree("publish-twice");
    assert!(receipt_path.exists());
    let base = head(&repo.path);

    let (code, out) = flow(&repo.path, &["publish", "--expected-old", &base]);
    assert_eq!(code, 0, "the fixture publication must succeed: {out}");
    assert_eq!(status_of(&out), "PUBLISHED", "{out}");
    let published = head(&repo.path);
    assert_ne!(published, base, "publication moves the branch");

    // The index tree is now exactly the published tree: publishing again would
    // only add an empty commit, so it is refused before any ref update.
    let (code, out) = flow(&repo.path, &["publish", "--expected-old", &published]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "ALREADY_PUBLISHED");
    assert_eq!(
        git(&repo.path, &["rev-parse", &format!("{published}^{{tree}}")]),
        tree,
        "the second publication must not move the branch"
    );
    assert_eq!(head(&repo.path), published);
}

#[test]
fn publish_refuses_refs_that_are_not_real_branch_refs() {
    let (repo, _tree, _receipt) = verified_tree("publish-refs");
    let old = head(&repo.path);

    // HEAD, a tag name and a remote-style ref are not full branch refs.
    for bad in ["HEAD", "main", "refs/tags/v1", "refs/remotes/origin/main"] {
        let (code, out) = flow(
            &repo.path,
            &["publish", "--ref", bad, "--expected-old", &old],
        );
        assert_eq!(code, 1, "--ref {bad:?} must be refused: {out}");
        assert_refused(&out, "REF_NOT_ALLOWED");
    }

    // A full branch ref with a byte Git cannot accept, and one above the length
    // bound, are refused by the same check.
    for bad in [
        "refs/heads/has space".to_string(),
        format!("refs/heads/{}", "a".repeat(1024)),
    ] {
        let (code, out) = flow(
            &repo.path,
            &["publish", "--ref", &bad, "--expected-old", &old],
        );
        assert_eq!(code, 1, "--ref {bad:?} must be refused: {out}");
        assert_refused(&out, "REF_NOT_ALLOWED");
    }

    // A syntactically valid branch that does not exist cannot supply the old
    // value: the refusal names it instead of guessing.
    let (code, out) = flow(&repo.path, &["publish", "--ref", "refs/heads/absent"]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "UNKNOWN_REF");
}

#[test]
fn propose_admits_a_patch_between_one_mebibyte_and_the_declared_bound() {
    let repo = new_repo("patch-medium");
    let base = head(&repo.path);
    // One mebibyte of payload: comfortably inside the documented 32 MiB
    // admission bound, and far above any "tiny patch only" shortcut. The patch
    // is a legitimate proposal and must be admitted.
    let body = "x".repeat(1024 * 1024);
    let patch = make_patch(&repo.path, &base, &[("medium.bin", Some(&body))]);
    let patch_len = fs::metadata(&patch).unwrap().len();
    assert!(
        patch_len > 1024 * 1024,
        "the fixture patch must be larger than 1 MiB, got {patch_len}"
    );
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("medium"));
    assert_eq!(
        code, 0,
        "a 1 MiB patch is inside the admission bound and must be admitted: {out}"
    );
    assert_eq!(status_of(&out), "ADMITTED");
}

#[test]
fn propose_refuses_a_patch_that_changes_nothing() {
    let repo = new_repo("empty-patch");
    let base = head(&repo.path);
    // An empty patch is refused by Git itself before admission can observe an
    // empty diff: nothing is recorded, and the refusal names the stage.
    let patch = write_patch("empty-patch", "");
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "PATCH_REFUSED");
    let proposals = repo.path.join(".flow").join("proposals");
    assert!(
        !proposals.exists() || fs::read_dir(&proposals).unwrap().next().is_none(),
        "a patch that changes nothing must not be recorded"
    );
}

#[test]
fn assemble_refuses_a_candidate_record_whose_proposal_list_changed() {
    let repo = new_repo("candidate-proposals-tamper");
    let base = head(&repo.path);
    let patch = make_patch(&repo.path, &base, &[("one.txt", Some("1\n"))]);
    let (_, admitted) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    let id = proposal_id(&admitted);
    let (code, candidate) = assembled(&repo.path, &[&id]);
    assert_eq!(code, 0, "{candidate}");
    let candidate_id = field(&candidate, "candidate_id");
    let path = candidate_file(&repo.path, &candidate_id);

    // Same candidate id and same tree, different attribution list: the record is
    // immutable, so this is a refusal rather than a silent overwrite.
    let original = fs::read_to_string(&path).unwrap();
    let tampered = original.replacen(
        &format!("\"{id}\""),
        "\"0123456789abcdef0123456789abcdef\"",
        1,
    );
    assert_ne!(
        tampered, original,
        "the proposals list must be in the record"
    );
    fs::write(&path, &tampered).unwrap();

    let (code, out) = assembled(&repo.path, &[&id]);
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "CANDIDATE_IMMUTABLE");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        tampered,
        "the tampered record must be left exactly as it was found"
    );
}

#[test]
fn propose_accepts_a_symlink_target_exactly_at_the_read_bound() {
    let repo = new_repo("symlink-at-bound");
    let base = head(&repo.path);
    // `git cat-file blob` returns the target plus the newline the patch line
    // carries, so a 4095-character body is exactly the 4096-byte read bound: the
    // check is `>`, not `>=`, and the contract says "1..=MAX".
    let body = "a".repeat(4095);
    let patch = write_patch("symlink-at-bound", &new_file_patch("120000", "link", &body));
    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &patch, Some("k"));
    assert_eq!(code, 0, "a target exactly at the bound is admitted: {out}");
    assert_eq!(status_of(&out), "ADMITTED");
    assert!(field(&out, "proposal_id").len() == 32, "{out}");

    // One byte more is refused by the same check, with its own reason.
    let body = "a".repeat(4096);
    let patch = write_patch(
        "symlink-over-bound",
        &new_file_patch("120000", "link", &body),
    );
    let (code, out) = propose(&repo.path, "agent-b", "T2", &base, &patch, Some("k2"));
    assert_eq!(code, 1, "{out}");
    assert_refused(&out, "SYMLINK_UNREADABLE");
    assert!(
        field(&out, "reason").contains("above 4096 bytes"),
        "{}",
        field(&out, "reason")
    );
}

#[test]
fn propose_accepts_a_patch_exactly_at_the_admission_bound() {
    let repo = new_repo("patch-at-bound");
    let base = head(&repo.path);
    // The documented bound is "above 32 MiB is refused", so exactly 32 MiB is
    // read and refused later, as a patch — not as PATCH_TOO_LARGE. Sparse: the
    // bound is on the declared size, not on 32 MiB of real disk.
    let large = unique("large-at-bound").with_extension("patch");
    let file = fs::File::create(&large).unwrap();
    file.set_len(32 * 1024 * 1024).unwrap();
    drop(file);

    let (code, out) = propose(&repo.path, "agent-a", "T1", &base, &large, Some("k"));
    assert_eq!(code, 1, "{out}");
    assert_ne!(
        code_of(&out),
        "PATCH_TOO_LARGE",
        "exactly at the bound must not be refused by the size check: {out}"
    );
    assert_refused(&out, "PATCH_REFUSED");
    let _ = fs::remove_file(&large);
}
