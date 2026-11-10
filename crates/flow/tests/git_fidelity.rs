//! P0 tree-fidelity gates: the checkout the obligations run in must be *exactly*
//! the verified tree, whatever the local Git configuration and hooks do.
//!
//! These are regression tests for two weaknesses reported by the production
//! audit and closed in this wave:
//!
//! 1. `verify_checkout_content` used to *skip* the byte comparison when neither
//!    `core.autocrlf`, `core.eol` nor an in-tree `.gitattributes` made a
//!    conversion "possible". A conversion driven by a **local** attribute source
//!    (`.git/info/attributes`, `core.attributesFile`, a local `filter.*` driver)
//!    was therefore invisible while `git worktree add` converted the checkout:
//!    the receipt said PASSED for content the obligations never saw.
//! 2. `verify_checkout_fidelity` only rejected **missing** entries. A
//!    `post-checkout` hook (which `git worktree add` runs, including through
//!    `core.hooksPath` pointing at a tracked directory) could create or rewrite
//!    files and still be certified.
//!
//! Each test asserts the refusal code `UNSUPPORTED_CHECKOUT` and that no receipt
//! was written: a checkout that is not the tree is refused, never repaired, and
//! no obligation is executed on it. On the pre-fix build every test here prints
//! `PASSED` instead, which is the reproduction.

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

/// A fixture repository with `core.autocrlf=false` set **locally**, so a
/// conversion observed later can only come from the attribute source the test
/// installs (a host-wide `core.autocrlf=true` would otherwise mask the bug).
fn new_repo(tag: &str) -> Repo {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("flow-fidelity-{tag}-{}-{n}", std::process::id()));
    let _ = remove_tree(&path);
    fs::create_dir_all(&path).expect("temp dir");
    git(&path, &["init", "-q", "-b", "main"]);
    git(&path, &["config", "user.name", "Flow Fidelity Test"]);
    git(
        &path,
        &["config", "user.email", "flow-fidelity@example.invalid"],
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

fn code_of(value: &Json) -> String {
    value
        .get("code")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

fn reason_of(value: &Json) -> String {
    value
        .get("reason")
        .and_then(Json::as_str)
        .unwrap_or("<none>")
        .to_string()
}

/// A policy that always passes, so the only thing under test is the checkout
/// gate: if the gate is missing, the run reports `PASSED`.
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

fn receipt_file(repo: &Path, tree: &str) -> PathBuf {
    repo.join(".flow")
        .join("receipts")
        .join(format!("{tree}.json"))
}

/// Prepare a repository whose index tree is committed and stable, with a
/// passing policy adopted, ready for the checkout gate to be exercised.
fn prepared_repo(tag: &str) -> (Repo, String) {
    let repo = new_repo(tag);
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    git(&repo.path, &["add", "-A"]);
    let tree = git(&repo.path, &["write-tree"]);
    (repo, tree)
}

/// Assert the run refused the checkout *before* writing a receipt, and say so
/// with the reason, so a failure of this test is self-explanatory.
fn assert_checkout_refused(repo: &Repo, tree: &str, expectation: &str) {
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 1,
        "a run on a checkout that is not the verified tree must exit 1, not report a proof: {out}"
    );
    assert_eq!(
        status_of(&out),
        "REFUSED",
        "a refused run must be reported as a refusal, not as a proof: {out}"
    );
    assert_eq!(
        code_of(&out),
        "UNSUPPORTED_CHECKOUT",
        "the checkout gate must fire (expected: {expectation}): {out}"
    );
    assert!(
        !receipt_file(&repo.path, tree).exists(),
        "a refused checkout must not write the complete receipt slot {tree}.json"
    );
}

// ------------------------------------------------- local attribute sources

/// `.git/info/attributes` is a *local* file: it is not in the tree, so the old
/// `conversion_possible` heuristic (in-tree `.gitattributes`, `core.autocrlf`,
/// `core.eol`) did not see it, while `git worktree add` still applied it.
#[test]
fn a_local_info_attributes_conversion_is_refused_not_certified() {
    let (repo, tree) = prepared_repo("info-attributes");
    fs::write(
        repo.path.join(".git").join("info").join("attributes"),
        "a.txt text eol=crlf\n",
    )
    .unwrap();
    assert_checkout_refused(&repo, &tree, "eol conversion from .git/info/attributes");
}

/// `core.attributesFile` is another local attribute source, and it is inherited
/// from the host configuration, so the repository content alone could not be
/// used to predict the conversion.
#[test]
fn a_core_attributes_file_conversion_is_refused_not_certified() {
    let (repo, tree) = prepared_repo("core-attributes-file");
    let attrs = repo.path.join("host-attributes");
    fs::write(&attrs, "a.txt text eol=crlf\n").unwrap();
    let attrs = attrs.to_string_lossy().replace('\\', "/");
    git(&repo.path, &["config", "core.attributesFile", &attrs]);
    assert_checkout_refused(&repo, &tree, "eol conversion from core.attributesFile");
}

// ---------------------------------------------------- hook side effects

/// Write a shell hook into the tree at mode `100755` (forced through the index
/// so the file mode does not depend on the host filesystem), point
/// `core.hooksPath` at the tracked directory, then let `git worktree add` run it.
fn install_tracked_hook(repo: &Repo, body: &str) {
    let hooks = repo.path.join(".hooks");
    fs::create_dir_all(&hooks).unwrap();
    let path = hooks.join("post-checkout");
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    // Unix runs a hook only when the file on disk is executable. A clone does
    // that from the tree mode; this fixture was written directly, so the bit is
    // set explicitly (a no-op on Windows, where the mode is not consulted).
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

/// A hook that adds a file the tree does not declare. The extra file can change
/// what an obligation sees (a toolchain file, a test, a build configuration)
/// without touching one byte the tree names, so it must be refused.
#[test]
fn a_hook_that_adds_a_file_is_refused_not_certified() {
    let repo = new_repo("hook-add");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    install_tracked_hook(&repo, "printf 'injected\\n' > injected.txt\n");
    let tree = git(&repo.path, &["write-tree"]);
    assert_checkout_refused(&repo, &tree, "extra file created by post-checkout");
}

/// A hook that rewrites a checked-out file with the same byte length: this is
/// the case a size comparison cannot see, and the case the old heuristic skipped
/// whenever no conversion looked possible.
#[test]
fn a_hook_that_rewrites_a_file_is_refused_not_certified() {
    let repo = new_repo("hook-rewrite");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    install_tracked_hook(&repo, "printf 'ALPHA\\n' > a.txt\n");
    let tree = git(&repo.path, &["write-tree"]);
    assert_checkout_refused(&repo, &tree, "same-length rewrite by post-checkout");
}

/// A hook that deletes a declared file: already caught by the presence check,
/// kept as a gate so a refactor cannot lose it.
#[test]
fn a_hook_that_deletes_a_file_is_refused_not_certified() {
    let repo = new_repo("hook-delete");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    install_tracked_hook(&repo, "rm -f a.txt\n");
    let tree = git(&repo.path, &["write-tree"]);
    assert_checkout_refused(&repo, &tree, "missing file after post-checkout");
}

/// Control test: with no local attribute source and no hook, the same fixture
/// must still be certifiable. A gate that refuses everything would pass the
/// tests above while proving nothing.
#[test]
fn a_faithful_checkout_is_still_certified() {
    let (repo, tree) = prepared_repo("clean");
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 0,
        "a faithful checkout must still be certifiable: {out}"
    );
    assert_eq!(status_of(&out), "PASSED", "{out}");
    assert!(
        receipt_file(&repo.path, &tree).exists(),
        "a PASSED run must write the receipt for {tree}"
    );
    let reason = reason_of(&out);
    assert!(
        reason.is_empty() || reason == "<none>",
        "a PASSED run must not carry a refusal reason: {reason}"
    );
}
