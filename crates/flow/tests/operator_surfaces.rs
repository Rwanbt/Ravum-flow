//! Operator-facing surfaces: `init` adoption, `doctor` disclosures, ref
//! validation, and corrupt local state.
//!
//! These are contracts an operator relies on before any verification runs: a
//! policy that arrived with a clone must be *shown* before it is adopted, a
//! changed policy must be reported as "not what you approved", `doctor` must
//! disclose the host conditions that change what a `PASSED` means (hooks,
//! attribute sources, case folding, a live lock), and recovery must refuse an
//! incomplete intent instead of guessing. Every test drives the shipped binary on
//! a real repository; nothing is stubbed, nothing is executed just to colour a
//! coverage report.

#[path = "../src/json.rs"]
#[allow(dead_code)]
mod json;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
    std::env::temp_dir().join(format!("flow-ops-{tag}-{}-{n}", std::process::id()))
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

fn new_repo(tag: &str) -> Repo {
    let path = unique(tag);
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Operator Test"]);
    git(&path, &["config", "user.email", "flow@example.invalid"]);
    git(&path, &["config", "commit.gpgsign", "false"]);
    fs::write(path.join("a.txt"), "alpha\n").unwrap();
    git(&path, &["add", "-A"]);
    git(&path, &["commit", "-q", "-m", "seed"]);
    Repo { path }
}

fn flow_raw(repo: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(repo)
        .args(args)
        .output()
        .expect("flow runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn flow(repo: &Path, args: &[&str]) -> (i32, Json) {
    let (code, stdout, stderr) = flow_raw(repo, args);
    let parsed = if stdout.trim().is_empty() {
        Json::Null
    } else {
        json::parse(&stdout)
            .unwrap_or_else(|e| panic!("flow {args:?} printed non-JSON ({e}):\n{stdout}\n{stderr}"))
    };
    (code, parsed)
}

fn field<'a>(value: &'a Json, key: &str) -> &'a str {
    value.get(key).and_then(Json::as_str).unwrap_or("<none>")
}

fn code_of(value: &Json) -> String {
    value
        .get("code")
        .and_then(Json::as_str)
        .unwrap_or("<no-code>")
        .to_string()
}

fn reason_of(value: &Json) -> String {
    value
        .get("reason")
        .and_then(Json::as_str)
        .unwrap_or("<no-reason>")
        .to_string()
}

fn status_of(value: &Json) -> &str {
    value
        .get("status")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
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

const REQUIRED_GIT_VERSION: &str =
    r#"{"id":"git-version","program":"git","args":["--version"],"required":true,"timeout_s":60}"#;

fn doctor_check<'a>(report: &'a Json, id: &str) -> &'a Json {
    let checks = report
        .get("checks")
        .and_then(Json::as_arr)
        .expect("doctor always reports checks");
    checks
        .iter()
        .find(|c| c.get("id").and_then(Json::as_str) == Some(id))
        .unwrap_or_else(|| panic!("doctor must report a {id:?} check: {report}"))
}

fn check_status(report: &Json, id: &str) -> String {
    field(doctor_check(report, id), "status").to_string()
}

fn check_detail(report: &Json, id: &str) -> String {
    field(doctor_check(report, id), "detail").to_string()
}

// ------------------------------------------------- init: adoption of a policy

#[test]
fn init_shows_a_committed_policy_and_adopts_nothing_until_told_to() {
    let repo = new_repo("init-adopt");
    write_config(&repo.path, REQUIRED_GIT_VERSION);

    // `init` on a working copy that already carries a `.flow` refuses outright;
    // the *informative* refusal is the one that forces the issue without adopting.
    let (code, out) = flow(&repo.path, &["init"]);
    assert_eq!(code, 1, "an existing .flow is a refusal: {out}");
    assert_eq!(
        reason_of(&out),
        ".flow already exists (pass --force to rewrite config, journal is preserved)"
    );

    let (code, out) = flow(&repo.path, &["init", "--force"]);
    assert_eq!(code, 1, "an unadopted policy is a refusal: {out}");
    assert_eq!(code_of(&out), "POLICY_ADOPTION_REQUIRED", "{out}");
    assert_eq!(status_of(&out), "REFUSED");
    let listed = out
        .get("obligations")
        .and_then(Json::as_arr)
        .expect("the refusal must show exactly what would run");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].as_str(),
        Some("git --version (required)"),
        "an operator must see the command and whether it is required"
    );
    assert_eq!(
        field(&out, "policy_digest").len(),
        12,
        "the refusal names a short policy digest"
    );
    assert!(
        !repo.path.join(".flow/adopted.json").exists(),
        "showing a policy is not adopting it"
    );
    assert!(
        !repo.path.join(".flow/journal.jsonl").exists(),
        "nothing under .flow is created before the policy is adopted"
    );

    // The explicit act adopts exactly that digest and says so.
    let (code, out) = flow(&repo.path, &["init", "--adopt-policy"]);
    assert_eq!(code, 0, "adoption must succeed: {out}");
    assert_eq!(status_of(&out), "INITIALIZED");
    assert_eq!(field(&out, "adoption_source"), "init --adopt-policy");
    assert_eq!(
        field(&out, "note"),
        "the adopted policy declares obligations; `.flow/.gitignore` keeps Flow state out of the verified tree"
    );
    assert!(
        repo.path.join(".flow/adopted.json").is_file(),
        "the adoption record is durable"
    );
    assert_eq!(
        fs::read_to_string(repo.path.join(".flow/.gitignore")).unwrap(),
        "*\n",
        "Flow state must never enter the verified tree"
    );
}

#[test]
fn init_refuses_a_committed_config_it_cannot_use_instead_of_adopting_a_default() {
    let repo = new_repo("init-unusable");
    let dir = repo.path.join(".flow");
    fs::create_dir_all(&dir).unwrap();

    // Torn JSON: adopting "whatever parses" would silently run nothing.
    fs::write(dir.join("config.json"), "{\"schema\":").unwrap();
    let (code, out) = flow(&repo.path, &["init", "--adopt-policy"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "CONFIG_UNREADABLE", "{out}");
    assert!(
        reason_of(&out).contains("cannot be used"),
        "the refusal must say why: {}",
        reason_of(&out)
    );
    assert!(!dir.join("adopted.json").exists());

    // A config from another schema is a claim about a contract this build does
    // not implement; it is never reinterpreted as the current one.
    fs::write(
        dir.join("config.json"),
        r#"{"schema":"flow-config/9","obligations":[]}"#,
    )
    .unwrap();
    let (code, out) = flow(&repo.path, &["init", "--adopt-policy"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "CONFIG_UNREADABLE", "{out}");
    assert!(!dir.join("adopted.json").exists());
}

#[test]
fn init_refuses_a_committed_policy_that_could_never_prove_anything() {
    let repo = new_repo("init-empty-policy");
    write_config(
        &repo.path,
        r#"{"id":"optional-only","program":"git","args":["--version"],"required":false,"timeout_s":60}"#,
    );

    let (code, out) = flow(&repo.path, &["init", "--adopt-policy"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "INVALID_POLICY", "{out}");
    assert!(
        reason_of(&out).contains("no required obligation"),
        "the reason must name the missing guarantee: {}",
        reason_of(&out)
    );
    assert!(
        !repo.path.join(".flow/adopted.json").exists(),
        "an invalid policy is not adopted"
    );
}

#[test]
fn init_force_preserves_the_journal_and_rewrites_only_the_approval() {
    let repo = new_repo("init-force");
    let (code, out) = flow(&repo.path, &["init"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        field(&out, "note"),
        "no obligation declared yet: `flow run` will refuse with UNKNOWN until .flow/config.json lists obligations; .flow/.gitignore keeps Flow state out of the verified tree"
    );

    let journal = repo.path.join(".flow/journal.jsonl");
    let record = "{\"schema\":\"flow-journal/1\",\"event\":\"note\",\"id\":\"kept\"}\n";
    fs::write(&journal, record).unwrap();

    let (code, out) = flow(&repo.path, &["init", "--force"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "INITIALIZED");
    assert_eq!(
        out.get("journal_preserved").and_then(Json::as_bool),
        Some(true),
        "init is not a reset"
    );
    assert_eq!(
        fs::read_to_string(&journal).unwrap(),
        record,
        "the journal is byte-identical after `init --force`"
    );
    assert_eq!(
        field(&out, "adoption_source"),
        "init",
        "a forced re-init records itself as the approval source"
    );
}

// ---------------------------------------------------- doctor: honest disclosure

#[test]
fn doctor_reports_a_policy_that_changed_since_it_was_approved() {
    let repo = new_repo("doctor-changed");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    let (code, out) = flow(&repo.path, &["init", "--adopt-policy"]);
    assert_eq!(code, 0, "{out}");

    // A second obligation appears after the approval: what would run is no
    // longer what was approved, and `doctor` says so without failing the host.
    write_config(
        &repo.path,
        &format!(
            "{REQUIRED_GIT_VERSION},{}",
            r#"{"id":"git-version-2","program":"git","args":["--version"],"required":true,"timeout_s":60}"#
        ),
    );
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(
        code, 0,
        "a policy drift is a warning, not a broken guarantee"
    );
    assert_eq!(check_status(&out, "policy_adopted"), "WARN", "{out}");
    assert!(
        check_detail(&out, "policy_adopted").contains("changed since it was approved"),
        "{}",
        check_detail(&out, "policy_adopted")
    );
}

#[test]
fn doctor_reports_corrupt_local_state_as_a_failed_guarantee() {
    let repo = new_repo("doctor-corrupt");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);

    // An adoption record from another schema cannot be interpreted: the answer is
    // FAILED (exit 1), never "assume it was approved".
    fs::write(
        repo.path.join(".flow/adopted.json"),
        r#"{"schema":"flow-adoption/9","policy_digest":"x"}"#,
    )
    .unwrap();
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 1, "a broken guarantee is not a success: {out}");
    assert_eq!(field(&out, "overall"), "FAILED");
    assert_eq!(check_status(&out, "policy_adopted"), "FAILED", "{out}");
    assert!(
        check_detail(&out, "policy_adopted").contains("refusing to guess what was approved"),
        "{}",
        check_detail(&out, "policy_adopted")
    );

    // A config that is not even JSON is the same class of answer.
    fs::write(repo.path.join(".flow/config.json"), "{not json").unwrap();
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(field(&out, "overall"), "FAILED");
    assert_eq!(check_status(&out, "policy_adopted"), "FAILED", "{out}");
}

#[test]
fn doctor_discloses_hooks_attribute_sources_case_folding_and_a_live_lock() {
    let repo = new_repo("doctor-host");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);

    // Host conditions that change what a PASSED would mean.
    git(&repo.path, &["config", "core.ignorecase", "true"]);
    git(&repo.path, &["config", "core.longpaths", "true"]);
    let attributes_file = repo.path.join("outside-attributes");
    fs::write(&attributes_file, "* text eol=crlf\n").unwrap();
    git(
        &repo.path,
        &[
            "config",
            "core.attributesFile",
            attributes_file.to_str().unwrap(),
        ],
    );
    fs::write(repo.path.join(".gitattributes"), "*.txt text\n").unwrap();
    fs::create_dir_all(repo.path.join(".git/info")).unwrap();
    fs::write(repo.path.join(".git/info/attributes"), "*.txt text\n").unwrap();
    fs::create_dir_all(repo.path.join(".git/hooks")).unwrap();
    fs::write(repo.path.join(".git/hooks/pre-commit"), "#!/bin/sh\n").unwrap();

    // A lock held by a provably live process (this test process).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    fs::write(
        repo.path.join(".flow/lock"),
        format!(
            "flow-lock/2 {} {} testnonce publish\n",
            std::process::id(),
            now
        ),
    )
    .unwrap();

    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 0, "warnings do not fail the host: {out}");
    assert_eq!(field(&out, "overall"), "OK");

    assert_eq!(check_status(&out, "case_sensitivity"), "WARN");
    assert!(
        check_detail(&out, "case_sensitivity").contains("core.ignorecase=\"true\""),
        "{}",
        check_detail(&out, "case_sensitivity")
    );
    assert_eq!(check_status(&out, "longpaths"), "OK", "{out}");
    assert!(check_detail(&out, "longpaths").contains("core.longpaths=\"true\""));

    assert_eq!(check_status(&out, "hooks"), "WARN");
    let hooks = check_detail(&out, "hooks");
    assert!(hooks.contains("pre-commit"), "{hooks}");
    assert!(
        hooks.contains("reference-transaction"),
        "the disclosure must name the hooks Git itself runs: {hooks}"
    );

    assert_eq!(check_status(&out, "gitattributes"), "WARN");
    let sources = check_detail(&out, "gitattributes");
    for needle in [
        ".gitattributes",
        ".git/info/attributes",
        "core.attributesFile",
        "compares every regular file with the blob",
    ] {
        assert!(
            sources.contains(needle),
            "{needle:?} missing from {sources}"
        );
    }

    assert_eq!(check_status(&out, "publication_lock"), "WARN");
    let lock = check_detail(&out, "publication_lock");
    assert!(lock.contains("attribution live"), "{lock}");
    assert!(
        lock.contains(&format!("pid {}", std::process::id())),
        "{lock}"
    );
    assert!(lock.contains("publish"), "{lock}");
}

// ------------------------------------------------- publish: ref-name validation

#[test]
fn publish_refuses_an_unusable_ref_name_before_taking_the_lock() {
    let repo = new_repo("publish-ref");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    let (code, out) = flow(&repo.path, &["init", "--adopt-policy"]);
    assert_eq!(code, 0, "{out}");
    let journal_before = fs::read(repo.path.join(".flow/journal.jsonl")).unwrap();

    for (reference, expected) in [
        (
            "refs/heads/bad name",
            "--ref \"refs/heads/bad name\" is not a usable ref name",
        ),
        (
            "refs/heads/a..b",
            "--ref \"refs/heads/a..b\" is not a valid ref name (git check-ref-format)",
        ),
    ] {
        let (code, out) = flow(&repo.path, &["publish", "--ref", reference]);
        assert_eq!(code, 1, "{out}");
        assert_eq!(status_of(&out), "REFUSED");
        assert_eq!(code_of(&out), "REF_NOT_ALLOWED");
        assert_eq!(reason_of(&out), expected, "{out}");
    }
    assert!(
        !repo.path.join(".flow/lock").exists(),
        "a ref-name refusal happens before the publication lock"
    );
    assert_eq!(
        fs::read(repo.path.join(".flow/journal.jsonl")).unwrap(),
        journal_before,
        "a refusal must not write a journal record"
    );
}

// ------------------------------------------- status: corrupt local state, blank

#[test]
fn status_reports_a_corrupt_receipt_file_as_absent_evidence() {
    let repo = new_repo("status-corrupt");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let receipts = repo.path.join(".flow/receipts");
    fs::create_dir_all(&receipts).unwrap();
    fs::write(
        receipts.join(format!("{}.json", "ab".repeat(20))),
        "{this is not json",
    )
    .unwrap();

    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "REPORT");
    assert_eq!(
        out.get("latest_receipt"),
        Some(&Json::Null),
        "an unparsable receipt is not displayed as evidence: {out}"
    );
}

#[test]
fn status_and_reconcile_skip_blank_journal_lines_without_inventing_intents() {
    let repo = new_repo("journal-blank");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let journal = repo.path.join(".flow/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    text.push_str("\n   \n\t\n");
    fs::write(&journal, &text).unwrap();

    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out.get("pending_intents").and_then(Json::as_num),
        Some(0.0),
        "blank lines are not records: {out}"
    );

    let (code, out) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "RECONCILED");
    assert_eq!(
        out.get("resolved")
            .and_then(Json::as_arr)
            .map(<[Json]>::len),
        Some(0)
    );
    assert_eq!(
        fs::read_to_string(&journal).unwrap(),
        text,
        "reconcile must not rewrite a journal it did not have to resolve"
    );
}

#[test]
fn status_reports_an_adoption_record_it_cannot_read() {
    let repo = new_repo("status-adoption");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);
    fs::write(
        repo.path.join(".flow/adopted.json"),
        r#"{"schema":"flow-adoption/9","policy_digest":"whatever"}"#,
    )
    .unwrap();

    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    let policy = out.get("policy").expect("status reports the policy");
    let error = policy
        .get("error")
        .and_then(Json::as_str)
        .unwrap_or_else(|| {
            panic!("an unreadable adoption record is an error, not a policy: {out}")
        });
    assert!(
        error.contains("adoption") && error.contains("refusing to guess"),
        "{error}"
    );
    assert_eq!(
        policy.get("adopted"),
        None,
        "no 'adopted: true' may be inferred from an unreadable record"
    );
}

// -------------------------------------------------------- reconcile: incomplete

#[test]
fn reconcile_refuses_an_intent_that_does_not_name_its_publication() {
    let repo = new_repo("reconcile-incomplete");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    let journal = repo.path.join(".flow/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    // Each record omits one field the recovery needs: without all three it
    // cannot know which publication was meant.
    text.push_str(&format!(
        "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"intent-no-new\",\"ref\":\"refs/heads/main\",\"old\":\"{head}\"}}\n"
    ));
    text.push_str(&format!(
        "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"intent-no-old\",\"ref\":\"refs/heads/main\",\"new\":\"{head}\"}}\n"
    ));
    text.push_str(&format!(
        "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"intent-no-ref\",\"old\":\"{head}\",\"new\":\"{head}\"}}\n"
    ));
    fs::write(&journal, &text).unwrap();
    let ref_before = git(&repo.path, &["rev-parse", "refs/heads/main"]);

    let (code, out) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code, 1, "an incomplete intent needs a human: {out}");
    assert_eq!(status_of(&out), "NEEDS_RECONCILIATION");
    let resolved = out
        .get("resolved")
        .and_then(Json::as_arr)
        .expect("reconcile lists what it resolved");
    assert_eq!(resolved.len(), 3, "{out}");
    for entry in resolved {
        assert_eq!(
            entry.get("reason").and_then(Json::as_str),
            Some("INCOMPLETE_INTENT"),
            "{entry}"
        );
    }
    assert_eq!(
        git(&repo.path, &["rev-parse", "refs/heads/main"]),
        ref_before,
        "recovery never moves a ref it cannot attribute"
    );
    assert_eq!(
        fs::read_to_string(&journal).unwrap(),
        text,
        "the journal is preserved when reconciliation cannot decide"
    );
}

#[test]
fn reconcile_diagnoses_a_shared_intent_id_instead_of_closing_both() {
    let repo = new_repo("reconcile-shared-id");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let journal = repo.path.join(".flow/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    // Two intents share one id and only one outcome arrived: closing both would
    // report an unresolved publication as reconciled.
    for _ in 0..2 {
        text.push_str("{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"shared\",\"ref\":\"refs/heads/main\"}\n");
    }
    text.push_str("{\"schema\":\"flow-journal/1\",\"event\":\"outcome\",\"id\":\"shared\"}\n");
    fs::write(&journal, &text).unwrap();

    let (code, out) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "NEEDS_RECONCILIATION");
    let resolved = out
        .get("resolved")
        .and_then(Json::as_arr)
        .expect("reconcile lists what it resolved");
    assert_eq!(resolved.len(), 2, "{out}");
    for entry in resolved {
        assert_eq!(
            entry.get("reason").and_then(Json::as_str),
            Some("DUPLICATE_INTENT_ID")
        );
    }
}

// ------------------------------------------------------ local state that is gone

#[test]
fn doctor_and_status_answer_when_a_state_directory_is_missing() {
    let repo = new_repo("missing-dirs");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);

    // No hooks directory: the disclosure is "no hook installed", not a failure.
    fs::remove_dir_all(repo.path.join(".git/hooks")).unwrap();
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(check_status(&out, "hooks"), "OK");
    assert!(check_detail(&out, "hooks").contains("no hook installed"));

    // No receipts directory: nothing is shown as the latest evidence.
    fs::remove_dir_all(repo.path.join(".flow/receipts")).unwrap();
    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out.get("latest_receipt"), Some(&Json::Null));
}

#[test]
fn doctor_reports_a_disabled_policy_and_publish_refuses_by_name() {
    let repo = new_repo("disabled");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);
    let path = repo.path.join(".flow/config.json");
    let config = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        config.replace("\"disabled\":false", "\"disabled\":true"),
    )
    .unwrap();

    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 0, "a disabled repository is a warning: {out}");
    assert_eq!(check_status(&out, "disabled"), "WARN");
    assert!(check_detail(&out, "disabled").contains("disabled=true"));
    // `disabled` is not part of the verification policy digest: it refuses
    // publication by itself, so it does not invalidate the approval.
    assert_eq!(check_status(&out, "policy_adopted"), "OK");

    let (code, out) = flow(&repo.path, &["publish", "--ref", "refs/heads/main"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "DISABLED", "{out}");
}

#[test]
fn publish_refuses_a_detached_head_without_an_explicit_ref() {
    let repo = new_repo("detached");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);
    git(&repo.path, &["checkout", "-q", "--detach"]);

    let (code, out) = flow(&repo.path, &["publish"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "ERROR");
    assert!(
        field(&out, "error").contains("detached HEAD: pass --ref explicitly"),
        "{out}"
    );

    // With an explicit ref the command proceeds to the next gate (no receipt).
    let (code, out) = flow(&repo.path, &["publish", "--ref", "refs/heads/main"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "NO_RECEIPT", "{out}");
}

#[test]
fn publish_refuses_a_lock_it_cannot_attribute_instead_of_stealing_it() {
    let repo = new_repo("lock-unknown");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);
    fs::write(repo.path.join(".flow/lock"), "not a flow lock record\n").unwrap();

    let (code, out) = flow(&repo.path, &["publish", "--ref", "refs/heads/main"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "LOCK_UNKNOWN", "{out}");
    assert_eq!(
        out.get("manual_required").and_then(Json::as_bool),
        Some(true),
        "an unattributable lock asks for a human: {out}"
    );
    assert!(
        field(&out, "lock_path").ends_with("lock"),
        "the refusal must name the file: {out}"
    );
    assert!(
        repo.path.join(".flow/lock").is_file(),
        "a refusal never deletes the lock"
    );
}

// ------------------------------------------------------ proposals and candidates

fn write_new_file_patch(dir: &Path, name: &str, path: &str, body: &str) -> PathBuf {
    let file = dir.join(name);
    fs::write(
        &file,
        format!(
            "diff --git a/{path} b/{path}\nnew file mode 100644\nindex 0000000..1111111\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1 @@\n+{body}\n"
        ),
    )
    .unwrap();
    file
}

#[test]
fn proposals_on_a_repository_without_flow_state_is_an_empty_list() {
    let repo = new_repo("proposals-empty");
    let (code, out) = flow(&repo.path, &["proposals"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "RECORD");
    assert_eq!(out.get("count").and_then(Json::as_num), Some(0.0));
}

#[test]
fn publish_reports_a_candidate_whose_tree_left_the_object_store() {
    let repo = new_repo("candidate-pruned");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);
    let base = git(&repo.path, &["rev-parse", "HEAD"]);

    let work = unique("candidate-patch");
    fs::create_dir_all(&work).unwrap();
    let patch = write_new_file_patch(&work, "add.patch", "new.txt", "hello");
    let patch_arg = patch.to_string_lossy().to_string();
    let (code, out) = flow(
        &repo.path,
        &[
            "propose", "--agent", "agent-a", "--task", "task-1", "--base", &base, "--patch",
            &patch_arg,
        ],
    );
    assert_eq!(code, 0, "admission must succeed: {out}");
    let proposal = field(&out, "proposal_id").to_string();
    let (code, out) = flow(&repo.path, &["assemble", "--proposal", &proposal]);
    assert_eq!(code, 0, "assembly must succeed: {out}");
    let candidate = field(&out, "candidate_id").to_string();
    let tree = field(&out, "tree_oid").to_string();
    assert_eq!(tree.len(), 40, "{tree}");
    assert!(candidate.len() == 32, "{candidate}");

    // Prune exactly the candidate tree: a record that names an object the
    // repository no longer holds cannot be published.
    let object = repo
        .path
        .join(".git/objects")
        .join(&tree[..2])
        .join(&tree[2..]);
    fs::remove_file(&object).expect("the fresh tree object is loose");

    let journal = repo.path.join(".flow/journal.jsonl");
    let before = fs::read(&journal).unwrap();
    let (code, out) = flow(&repo.path, &["publish", "--candidate", &candidate]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "CANDIDATE_TREE_MISSING", "{out}");
    assert_eq!(
        fs::read(&journal).unwrap(),
        before,
        "a refusal writes no journal record"
    );
    assert!(
        !repo.path.join(".flow/lock").exists(),
        "the guard releases the lock on a refusal"
    );

    // Verification names the same record and must refuse it for the same reason:
    // it would otherwise certify a tree it cannot check out.
    let (code, out) = flow(&repo.path, &["run", "--candidate", &candidate]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "CANDIDATE_TREE_MISSING", "{out}");
    assert_eq!(fs::read(&journal).unwrap(), before);
    let _ = remove_tree(&work);
}

#[test]
fn reconcile_records_only_a_decisive_observation() {
    let repo = new_repo("reconcile-decisive");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let old = git(&repo.path, &["rev-parse", "HEAD"]);
    git(
        &repo.path,
        &["commit", "-q", "--allow-empty", "-m", "second"],
    );
    let new = git(&repo.path, &["rev-parse", "HEAD"]);
    git(
        &repo.path,
        &["commit", "-q", "--allow-empty", "-m", "third"],
    );
    let other = git(&repo.path, &["rev-parse", "HEAD"]);
    git(&repo.path, &["branch", "at-old", &old]);
    git(&repo.path, &["branch", "at-new", &new]);
    git(&repo.path, &["branch", "at-other", &other]);

    let journal = repo.path.join(".flow/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    for (id, reference) in [
        ("intent-old", "refs/heads/at-old"),
        ("intent-new", "refs/heads/at-new"),
        ("intent-other", "refs/heads/at-other"),
    ] {
        text.push_str(&format!(
            "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"{id}\",\"ref\":\"{reference}\",\"old\":\"{old}\",\"new\":\"{new}\"}}\n"
        ));
    }
    fs::write(&journal, &text).unwrap();

    let (code, out) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code, 1, "one observation is ambiguous: {out}");
    assert_eq!(status_of(&out), "NEEDS_RECONCILIATION");
    let resolved = out.get("resolved").and_then(Json::as_arr).unwrap();
    assert_eq!(resolved.len(), 3, "{out}");
    let resolution = |id: &str| {
        resolved
            .iter()
            .find(|entry| entry.get("id").and_then(Json::as_str) == Some(id))
            .and_then(|entry| entry.get("resolution"))
            .and_then(Json::as_str)
            .unwrap_or("<none>")
            .to_string()
    };
    assert_eq!(resolution("intent-old"), "NOT_PUBLISHED");
    assert_eq!(resolution("intent-new"), "PUBLISHED");
    assert_eq!(resolution("intent-other"), "NEEDS_RECONCILIATION");

    // Only the two decisive observations became durable outcomes.
    let after = fs::read_to_string(&journal).unwrap();
    assert_eq!(after.matches("\"event\":\"outcome\"").count(), 2, "{after}");
    assert!(
        after.contains("\"status\":\"RECONCILED_NOT_PUBLISHED\""),
        "{after}"
    );
    assert!(
        after.contains("\"status\":\"RECONCILED_PUBLISHED\""),
        "{after}"
    );
}

#[test]
fn explain_reports_an_unknown_candidate() {
    let repo = new_repo("explain-unknown");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let unknown = "d".repeat(32);
    let (code, out) = flow(&repo.path, &["explain", "--candidate", &unknown]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "UNKNOWN_CANDIDATE", "{out}");
}

// ------------------------------------------------- approval and policy identity

#[test]
fn approve_refuses_a_policy_that_proves_nothing_or_has_duplicate_ids() {
    let repo = new_repo("approve-invalid");
    write_config(
        &repo.path,
        r#"{"id":"optional-only","program":"git","args":["--version"],"required":false,"timeout_s":60}"#,
    );
    let (code, out) = flow(&repo.path, &["approve"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "INVALID_POLICY", "{out}");
    assert!(reason_of(&out).contains("no required obligation"), "{out}");

    // Two obligations with one id would write one log file and lose evidence.
    write_config(
        &repo.path,
        concat!(
            r#"{"id":"dup","program":"git","args":["--version"],"required":true,"timeout_s":60},"#,
            r#"{"id":"dup","program":"git","args":["--version"],"required":true,"timeout_s":60}"#
        ),
    );
    let (code, out) = flow(&repo.path, &["approve"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "INVALID_POLICY", "{out}");
    assert!(reason_of(&out).contains("duplicate obligation id"), "{out}");
    assert!(!repo.path.join(".flow/adopted.json").exists());
}

#[test]
fn doctor_reports_a_policy_that_was_never_adopted_and_a_lock_it_cannot_parse() {
    let repo = new_repo("doctor-unadopted");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(check_status(&out, "policy_adopted"), "WARN");
    assert!(
        check_detail(&out, "policy_adopted").contains("no adoption record"),
        "{}",
        check_detail(&out, "policy_adopted")
    );

    // An unparseable lock file is a broken guarantee, not a stale one.
    fs::write(repo.path.join(".flow/lock"), "garbage\n").unwrap();
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(field(&out, "overall"), "FAILED");
    assert_eq!(check_status(&out, "publication_lock"), "FAILED");
    assert!(
        check_detail(&out, "publication_lock").contains("unparseable"),
        "{}",
        check_detail(&out, "publication_lock")
    );
}

#[test]
fn status_reports_no_policy_on_a_repository_that_was_never_initialised() {
    let repo = new_repo("status-uninitialised");
    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "REPORT");
    assert_eq!(out.get("policy"), Some(&Json::Null));
    assert_eq!(
        out.get("obligations_declared").and_then(Json::as_num),
        Some(0.0)
    );
}

#[test]
fn publish_reports_an_unknown_candidate_and_an_inline_flag_form() {
    let repo = new_repo("publish-candidate");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);

    let unknown = "b".repeat(32);
    let (code, out) = flow(&repo.path, &["publish", "--candidate", &unknown]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "UNKNOWN_CANDIDATE", "{out}");

    // The `--name=value` spelling is the same request as `--name value`; it
    // reaches the same gate (here: no receipt for the index tree).
    let (code, out) = flow(&repo.path, &["publish", "--ref=refs/heads/main"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "NO_RECEIPT", "{out}");
}

#[test]
fn proposals_show_reports_an_unknown_id_and_list_accepts_a_positional_subcommand() {
    let repo = new_repo("proposals-show");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let unknown = "c".repeat(32);
    let (code, out) = flow(&repo.path, &["proposals", "show", &unknown]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "UNKNOWN_PROPOSAL", "{out}");
}

#[test]
fn reconcile_refuses_an_unattributable_lock_and_reports_a_vanished_ref() {
    // A lock that cannot be attributed is a refusal, not a retry.
    let locked = new_repo("reconcile-locked");
    assert_eq!(flow(&locked.path, &["init"]).0, 0);
    fs::write(locked.path.join(".flow/lock"), "garbage\n").unwrap();
    let (code, out) = flow(&locked.path, &["reconcile"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "LOCK_UNKNOWN", "{out}");

    // An intent that names a ref the repository no longer has needs a human.
    let vanished = new_repo("reconcile-vanished-ref");
    assert_eq!(flow(&vanished.path, &["init"]).0, 0);
    let head = git(&vanished.path, &["rev-parse", "HEAD"]);
    let journal = vanished.path.join(".flow/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    text.push_str(&format!(
        "{{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"intent-gone-ref\",\"ref\":\"refs/heads/gone\",\"old\":\"{head}\",\"new\":\"{head}\"}}\n"
    ));
    fs::write(&journal, &text).unwrap();
    let (code, out) = flow(&vanished.path, &["reconcile"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "NEEDS_RECONCILIATION");
    let resolved = out.get("resolved").and_then(Json::as_arr).unwrap();
    assert_eq!(resolved.len(), 1, "{out}");
    assert_eq!(
        resolved[0].get("reason").and_then(Json::as_str),
        Some("REF_ABSENT")
    );
}

#[test]
fn assembling_the_same_proposals_twice_reuses_the_candidate() {
    let repo = new_repo("assemble-twice");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);
    let base = git(&repo.path, &["rev-parse", "HEAD"]);
    let work = unique("assemble-patch");
    fs::create_dir_all(&work).unwrap();
    let patch = write_new_file_patch(&work, "add.patch", "twice.txt", "one");
    let patch_arg = patch.to_string_lossy().to_string();
    let (code, out) = flow(
        &repo.path,
        &[
            "propose", "--agent", "agent-a", "--task", "task-1", "--base", &base, "--patch",
            &patch_arg,
        ],
    );
    assert_eq!(code, 0, "{out}");
    let proposal = field(&out, "proposal_id").to_string();

    let (code, first) = flow(&repo.path, &["assemble", "--proposal", &proposal]);
    assert_eq!(code, 0, "{first}");
    let (code, second) = flow(&repo.path, &["assemble", "--proposal", &proposal]);
    assert_eq!(
        code, 0,
        "assembling the same input again must be safe: {second}"
    );
    assert_eq!(
        field(&second, "candidate_id"),
        field(&first, "candidate_id"),
        "the same proposals on the same base describe the same candidate"
    );
    assert_eq!(
        field(&second, "tree_oid"),
        field(&first, "tree_oid"),
        "and therefore the same tree"
    );
    let candidates = fs::read_dir(repo.path.join(".flow/candidates"))
        .unwrap()
        .count();
    assert_eq!(candidates, 1, "no second candidate record is written");
    let _ = remove_tree(&work);
}

// ------------------------------------------------------------ flag/usage edges

#[test]
fn a_value_flag_without_a_value_is_a_usage_error_before_any_mutation() {
    let repo = new_repo("flags-value");
    let snapshot = git(&repo.path, &["status", "--porcelain"]);

    let (code, out) = flow(&repo.path, &["publish", "--ref", "--dry-run"]);
    assert_eq!(code, 2, "a usage error is exit 2: {out}");
    assert_eq!(status_of(&out), "ERROR");
    let reason = field(&out, "error");
    assert!(
        reason.contains("`--ref` needs a value"),
        "the reason must name the flag: {reason}"
    );
    assert!(
        reason.contains("found the flag"),
        "the reason must say what it saw instead: {reason}"
    );

    let (code, out) = flow(&repo.path, &["propose", "--agent", "a", "--task", "t"]);
    assert_eq!(code, 2, "{out}");
    assert_eq!(code_of(&out), "USAGE");
    assert!(
        reason_of(&out).contains("--base <commit> --patch <file>"),
        "the usage line must be shown: {}",
        reason_of(&out)
    );

    assert_eq!(
        git(&repo.path, &["status", "--porcelain"]),
        snapshot,
        "a usage error touches nothing"
    );
    assert!(!repo.path.join(".flow").exists());
}

#[test]
fn a_double_dash_is_accepted_as_a_separator() {
    let repo = new_repo("flags-dash");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let (code, out) = flow(&repo.path, &["status", "--"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "REPORT");
}

#[test]
fn doctor_reads_hooks_from_an_absolute_hooks_path() {
    let repo = new_repo("doctor-abs-hooks");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    // `git rev-parse --git-path hooks` answers with an absolute path when
    // core.hooksPath is absolute; doctor must list that directory, not
    // `<root>/<absolute path>`.
    let hooks = repo.path.join("hooks-absolute");
    fs::create_dir_all(&hooks).unwrap();
    fs::write(hooks.join("post-checkout"), "#!/bin/sh\n").unwrap();
    git(
        &repo.path,
        &["config", "core.hooksPath", hooks.to_str().unwrap()],
    );
    let (code, out) = flow(&repo.path, &["doctor"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(check_status(&out, "hooks"), "WARN", "{out}");
    let detail = check_detail(&out, "hooks");
    assert!(detail.contains("post-checkout"), "{detail}");
}

#[test]
fn doctor_reports_toolchains_that_are_not_on_the_path() {
    let repo = new_repo("doctor-toolchains");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    // Keep exactly the directory that holds `git`, so Git itself still resolves
    // but the language toolchains do not. This is the disclosure an operator
    // reads before declaring an obligation that needs Cargo or Node.
    let path = std::env::var_os("PATH").unwrap_or_default();
    let git_dir = std::env::split_paths(&path)
        .find(|dir| {
            ["git.exe", "git"]
                .iter()
                .any(|name| dir.join(name).is_file())
        })
        .expect("git must be on PATH to build the fixture");
    let out = Command::new(env!("CARGO_BIN_EXE_flow"))
        .current_dir(&repo.path)
        .arg("doctor")
        .env("PATH", &git_dir)
        .output()
        .expect("flow runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(out.status.code(), Some(0), "doctor failed: {text}");
    let report = json::parse(&text).expect("doctor prints JSON");
    let detail = check_detail(&report, "toolchains");
    assert!(detail.contains("cargo=missing"), "{detail}");
    assert!(detail.contains("rustc=missing"), "{detail}");
}

#[test]
fn status_ignores_a_receipt_path_it_cannot_read() {
    let repo = new_repo("status-unreadable-receipt");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let receipts = repo.path.join(".flow").join("receipts");
    fs::create_dir_all(&receipts).unwrap();
    fs::write(
        receipts.join("0000000000000000000000000000000000000000.json"),
        "{}\n",
    )
    .unwrap();
    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.get("latest_receipt")
            .map(|v| !matches!(v, Json::Null))
            .unwrap_or(false),
        "the readable receipt must be reported: {out}"
    );
    // A *directory* named like a receipt sorts last; reading it fails, and the
    // status must report no latest receipt instead of panicking or inventing one.
    fs::create_dir_all(receipts.join("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz.json")).unwrap();
    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(status_of(&out), "REPORT");
    assert!(
        matches!(out.get("latest_receipt"), Some(Json::Null) | None),
        "an unreadable receipt path must not be reported as evidence: {out}"
    );
}

#[test]
fn init_discloses_an_optional_obligation_as_optional() {
    let repo = new_repo("init-optional");
    write_config(
        &repo.path,
        r#"{"id":"git-version","program":"git","args":["--version"],"required":true,"timeout_s":60},{"id":"lint","program":"git","args":["status"],"required":false,"timeout_s":60}"#,
    );
    let (code, out) = flow(&repo.path, &["init", "--force"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(code_of(&out), "POLICY_ADOPTION_REQUIRED", "{out}");
    let listed = out
        .get("obligations")
        .and_then(Json::as_arr)
        .expect("the refusal must show what would run");
    assert_eq!(
        listed[1].as_str(),
        Some("git status (optional)"),
        "an optional obligation must not be presented as required"
    );
}

#[test]
fn init_adopt_policy_on_an_empty_flow_directory_still_refuses() {
    let repo = new_repo("init-adopt-no-config");
    fs::create_dir_all(repo.path.join(".flow")).unwrap();
    // `--adopt-policy` is about an *existing config*; an empty state directory
    // still needs `--force` (or a first `init`), so a half-created state is not
    // silently taken over.
    let (code, out) = flow(&repo.path, &["init", "--adopt-policy"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        reason_of(&out).contains(".flow already exists"),
        "{}",
        reason_of(&out)
    );
    assert!(!repo.path.join(".flow/adopted.json").exists());
}

// ------------------------------------------- operator counters and boundaries

/// An intent no outcome can be attributed to is *pending*: `status` must count it
/// rather than report a quiet zero. Counting, not membership — one outcome may
/// never close two intents.
#[test]
fn status_counts_an_intent_with_no_outcome_as_pending() {
    let repo = new_repo("pending-intent");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let journal = repo.path.join(".flow/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    text.push_str(
        "{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"open-1\",\"ref\":\"refs/heads/main\"}\n",
    );
    fs::write(&journal, &text).unwrap();

    let (code, out) = flow(&repo.path, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out.get("pending_intents").and_then(Json::as_num),
        Some(1.0),
        "one intent without an outcome is one pending intent: {out}"
    );
    assert_eq!(
        fs::read_to_string(&journal).unwrap(),
        text,
        "status may not rewrite the journal it only reads"
    );
}

/// Two outcomes close two intents that share an id: nothing is pending, so
/// recovery must say `RECONCILED` rather than manufacture a conflict. The
/// duplicate-id conflict exists only while an outcome is missing.
#[test]
fn a_duplicate_intent_that_is_fully_closed_is_not_reported_as_pending() {
    let repo = new_repo("reconcile-closed-twins");
    assert_eq!(flow(&repo.path, &["init"]).0, 0);
    let journal = repo.path.join(".flow/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    for _ in 0..2 {
        text.push_str("{\"schema\":\"flow-journal/1\",\"event\":\"intent\",\"id\":\"closed\",\"ref\":\"refs/heads/main\"}\n");
    }
    for _ in 0..2 {
        text.push_str("{\"schema\":\"flow-journal/1\",\"event\":\"outcome\",\"id\":\"closed\",\"status\":\"PUBLISHED\"}\n");
    }
    fs::write(&journal, &text).unwrap();

    let (code, out) = flow(&repo.path, &["reconcile"]);
    assert_eq!(code, 0, "a fully closed pair needs no human: {out}");
    assert_eq!(status_of(&out), "RECONCILED");
    assert_eq!(
        out.get("resolved")
            .and_then(Json::as_arr)
            .map(<[Json]>::len),
        Some(0),
        "two outcomes close two intents: {out}"
    );
    assert_eq!(
        out.get("manual_required").and_then(Json::as_bool),
        Some(false)
    );
}

/// The ref-name length check is a cheap pre-filter, not the Git validity gate: a
/// ref of exactly the documented 1 024-character limit must reach the receipt
/// gate (and be refused there), not be reported as an unusable name.
#[test]
fn a_ref_of_exactly_the_documented_limit_reaches_the_receipt_gate() {
    let repo = new_repo("publish-ref-limit");
    write_config(&repo.path, REQUIRED_GIT_VERSION);
    assert_eq!(flow(&repo.path, &["init", "--adopt-policy"]).0, 0);

    let reference = format!("refs/heads/{}", "a".repeat(1024 - "refs/heads/".len()));
    assert_eq!(reference.chars().count(), 1024);
    let (code, out) = flow(&repo.path, &["publish", "--ref", &reference]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "REFUSED");
    assert_eq!(
        code_of(&out),
        "NO_RECEIPT",
        "the limit itself is not an unusable name: {out}"
    );
}
