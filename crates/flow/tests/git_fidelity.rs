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
//! Each test asserts the refusal code `UNSUPPORTED_CHECKOUT` (or, for a write that
//! happens after the check, `CHECKOUT_MUTATED_DURING_RUN`) and that no receipt was
//! written: a checkout that is not the tree is refused, never repaired, and no
//! obligation is executed on it. On the pre-fix build every test here reports
//! `PASSED` instead, which is the reproduction.
//!
//! Scope note: the fidelity check observes the checkout **when it runs**. A
//! `post-checkout` hook that spawns a writer changing a file *later* cannot be seen
//! by that check; the run re-proves the declared files after the last obligation
//! and refuses (`a_delayed_write_during_the_obligations_is_refused`). A write that
//! is reverted before that re-check, or happens after it, is not detectable without
//! a snapshot — the limit is documented, not claimed away.

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

/// Marker argument of the smudge-filter probe (never exported: the filter command
/// carries it as a positional argument, and an obligation cannot see it).
const SMUDGE_PROBE: &str = "FLOW_FIDELITY_SMUDGE_PROBE";

/// Child probe used as a Git `smudge` driver: it proves it ran by writing the
/// marker, then rewrites the blob (upper-cased) so the checkout cannot equal the
/// tree. Returns immediately in a normal `cargo test` run.
#[test]
fn smudge_probe_child() {
    let args: Vec<String> = std::env::args().collect();
    let Some(position) = args.iter().position(|arg| arg == SMUDGE_PROBE) else {
        return;
    };
    let marker = std::path::PathBuf::from(args.get(position + 1).expect("marker path"));
    fs::write(&marker, "ran\n").expect("probe marker");
    let mut input = Vec::new();
    std::io::Read::read_to_end(&mut std::io::stdin(), &mut input).expect("read stdin");
    for byte in &mut input {
        byte.make_ascii_uppercase();
    }
    std::io::Write::write_all(&mut std::io::stdout(), &input).expect("write stdout");
}

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

/// A `smudge` driver named only by a **local** attribute file. The driver really
/// runs (the probe writes a marker) and rewrites the content, which is exactly the
/// case a size or presence check cannot see.
#[test]
fn a_local_smudge_filter_is_refused_not_certified() {
    let (repo, tree) = prepared_repo("smudge");
    let exe = std::env::current_exe().expect("test executable");
    let exe = exe.to_string_lossy();
    // `current_exe` can be a `\\?\` verbatim path on Windows; Git runs the filter
    // through its own shell, which wants an ordinary path.
    let exe = exe.strip_prefix(r"\\?\").unwrap_or(&exe).replace('\\', "/");
    let marker = repo.path.join("smudge-ran");
    let marker = marker.to_string_lossy().replace('\\', "/");
    let command =
        format!("\"{exe}\" --exact smudge_probe_child --nocapture {SMUDGE_PROBE} \"{marker}\"");
    git(&repo.path, &["config", "filter.flowevil.smudge", &command]);
    fs::write(
        repo.path.join(".git").join("info").join("attributes"),
        "a.txt filter=flowevil\n",
    )
    .unwrap();
    assert_checkout_refused(&repo, &tree, "smudge filter from .git/info/attributes");
    assert!(
        repo.path.join("smudge-ran").exists(),
        "the fixture is only meaningful if the smudge driver actually ran"
    );
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

/// A hook that changes the **executable bit**. The bytes are identical, so only a
/// mode check can see it; on a host that carries the bit, an obligation would run
/// a different file than the tree declares. Windows does not represent the bit,
/// so the gate is Unix-only and the limit is documented.
#[cfg(unix)]
#[test]
fn a_hook_that_changes_the_executable_bit_is_refused_not_certified() {
    let repo = new_repo("hook-chmod");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    install_tracked_hook(&repo, "chmod +x a.txt\n");
    let tree = git(&repo.path, &["write-tree"]);
    assert_checkout_refused(&repo, &tree, "executable bit changed by post-checkout");
}

/// A hook that creates a file whose name merely *starts with* Flow's scratch
/// prefix. Only the ownership marker itself is Flow's; any other such name is a
/// file the tree does not declare.
#[test]
fn a_hook_file_named_like_a_scratch_marker_is_refused_not_certified() {
    let repo = new_repo("hook-scratch-prefix");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    install_tracked_hook(&repo, "printf 'x\\n' > .flow-scratch-injected.txt\n");
    let tree = git(&repo.path, &["write-tree"]);
    assert_checkout_refused(&repo, &tree, "extra file named like a scratch marker");
}

/// A hook that creates an **empty directory**. Git trees do not represent empty
/// directories, but an obligation that discovers directories (`find`, a build
/// cache, a plugin scanner) sees one, so it is a path the tree does not declare.
#[test]
fn a_hook_that_creates_an_empty_directory_is_refused_not_certified() {
    let repo = new_repo("hook-mkdir");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    install_tracked_hook(&repo, "mkdir .build-cache\n");
    let tree = git(&repo.path, &["write-tree"]);
    assert_checkout_refused(&repo, &tree, "empty directory created by post-checkout");
}

/// A **delayed** write: the hook spawns a writer that changes `a.txt` after the
/// fidelity check has already passed, and the obligation waits until it actually
/// saw the changed bytes. The pre-check cannot see this; the post-run re-check
/// must refuse the run, so no receipt can claim the tree passed.
#[test]
fn a_delayed_write_during_the_obligations_is_refused() {
    let repo = new_repo("hook-delayed");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    // The writer blocks until the obligation creates `gate`, then rewrites a.txt
    // with the same byte length. No timer is involved, so the test is not racy:
    // the obligation only exits after it has read the rewritten content.
    install_tracked_hook(
        &repo,
        "( while [ ! -f gate ]; do :; done; printf 'ALPHA\\n' > a.txt ) >/dev/null 2>&1 &\n",
    );
    let exe = std::env::current_exe()
        .expect("test executable")
        .to_string_lossy()
        .replace('\\', "/");
    let obligation = format!(
        r#"{{"id":"delayed","program":"{}","args":["--exact","delayed_write_probe","--nocapture","{DELAYED_PROBE}"],"required":true,"timeout_s":60}}"#,
        exe.replace('"', "")
    );
    write_config(&repo.path, &obligation);
    let tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 1,
        "a run whose declared file changed while the obligations ran must not exit 0: {out}"
    );
    assert_eq!(status_of(&out), "REFUSED", "{out}");
    assert_eq!(
        code_of(&out),
        "CHECKOUT_MUTATED_DURING_RUN",
        "the post-run re-check must refuse the run: {out}"
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "a run whose checkout changed must not write the receipt for {tree}"
    );
}

const DELAYED_PROBE: &str = "FLOW_FIDELITY_DELAYED_WRITE_PROBE";

/// Child probe: create the gate the background writer waits for, then wait until
/// the declared file really contains the rewritten bytes, and exit 0. If the
/// rewrite never happens, fail loudly (`exit 3`) instead of reporting a pass.
#[test]
fn delayed_write_probe() {
    if !std::env::args().any(|arg| arg == DELAYED_PROBE) {
        return;
    }
    fs::write("gate", "go\n").expect("write the gate file");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if let Ok(bytes) = fs::read("a.txt") {
            if bytes == b"ALPHA\n" {
                return;
            }
        }
        if std::time::Instant::now() >= deadline {
            std::process::exit(3);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// A tree path that is not valid UTF-8 cannot be represented faithfully on this
/// platform, and must be refused with *that* reason — not misdiagnosed as a case
/// collision, which is what a lossy conversion to `U+FFFD` produces.
#[cfg(unix)]
#[test]
fn a_tree_path_that_is_not_valid_utf8_is_refused_with_its_own_reason() {
    use std::os::unix::ffi::OsStringExt;
    let repo = new_repo("non-utf8");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    let name = std::ffi::OsString::from_vec(b"bad-\xff-name.txt".to_vec());
    fs::write(repo.path.join(&name), "content\n").unwrap();
    git(&repo.path, &["add", "-A"]);
    git(&repo.path, &["commit", "-q", "-m", "non-utf8 path"]);
    let tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 1,
        "a non-UTF-8 tree path must not be certified: {out}"
    );
    assert_eq!(status_of(&out), "REFUSED", "{out}");
    assert_eq!(
        code_of(&out),
        "UNSUPPORTED_CHECKOUT",
        "the refusal must name the checkout gate: {out}"
    );
    assert!(
        reason_of(&out).contains("not valid UTF-8"),
        "the reason must name the real cause, not a lossy case collision: {}",
        reason_of(&out)
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "a refused tree must not gain a receipt"
    );
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

/// A tree entry that declares a symlink must appear as a symlink in the
/// checkout. On a host with `core.symlinks=false` (the Windows default) Git
/// writes the link target as a regular file: the obligations would then read a
/// file where the tree says symlink, so the checkout is refused.
#[test]
fn a_symlink_entry_checked_out_as_a_regular_file_is_refused() {
    let (repo, _) = prepared_repo("symlink-as-file");
    // Force the Windows default on every platform, so the same refusal is
    // exercised on Linux too (where symlinks are otherwise materialised).
    git(&repo.path, &["config", "core.symlinks", "false"]);
    let blob = git(&repo.path, &["hash-object", "-w", "a.txt"]);
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
    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "REFUSED", "{out}");
    assert_eq!(code_of(&out), "UNSUPPORTED_CHECKOUT", "{out}");
    assert!(
        reason_of(&out).contains("checked out as a regular file"),
        "the refusal must name the type mismatch: {}",
        reason_of(&out)
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "an unfaithful checkout must never produce a receipt"
    );
}

// ------------------------------------------- type fidelity at the checkout gate

/// A tree that names a gitlink must be refused **for that reason**. A checkout
/// never materialises submodule content, so without the explicit gitlink gate the
/// refusal surfaces later as "the checkout is missing N tree entries", which
/// names the symptom and hides the cause.
#[test]
fn a_tree_with_a_gitlink_is_refused_with_its_own_reason() {
    let (repo, _) = prepared_repo("gitlink");
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    git(
        &repo.path,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{head},sub"),
        ],
    );
    let tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "a gitlink cannot be certified: {out}");
    assert_eq!(status_of(&out), "REFUSED", "{out}");
    assert_eq!(code_of(&out), "UNSUPPORTED_CHECKOUT", "{out}");
    assert!(
        reason_of(&out).contains("is a gitlink"),
        "the refusal must name the gitlink, not a missing entry: {}",
        reason_of(&out)
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "a gitlink tree must not gain a receipt"
    );
}

/// Control test for the symlink type guard: a symlink that the checkout really
/// materialises as a symlink is certifiable. Without it, a gate that refused
/// every symlink would still pass `a_symlink_entry_checked_out_as_a_regular_file_is_refused`.
#[cfg(unix)]
#[test]
fn a_symlink_entry_checked_out_as_a_symlink_is_certified() {
    use std::os::unix::fs::symlink;
    let (repo, _) = prepared_repo("symlink-as-link");
    symlink("a.txt", repo.path.join("link")).unwrap();
    git(&repo.path, &["add", "link"]);
    let tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(
        code, 0,
        "a faithfully materialised symlink is certifiable: {out}"
    );
    assert_eq!(status_of(&out), "PASSED", "{out}");
    assert!(
        receipt_file(&repo.path, &tree).exists(),
        "a PASSED run must write the receipt for {tree}"
    );
}

/// A hook that turns a declared **regular file** into a symlink. The path still
/// exists, so the presence check is satisfied; only the type guard can see that
/// the obligations would read something the tree does not declare, and the
/// refusal must say *that*, not report a mode or content mismatch later.
#[cfg(unix)]
#[test]
fn a_hook_that_replaces_a_file_with_a_symlink_is_refused_with_its_own_reason() {
    let repo = new_repo("hook-symlink-for-file");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    install_tracked_hook(&repo, "rm -f a.txt && ln -s nowhere a.txt\n");
    let tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "REFUSED", "{out}");
    assert_eq!(code_of(&out), "UNSUPPORTED_CHECKOUT", "{out}");
    assert!(
        reason_of(&out).contains("was not checked out as a regular file"),
        "the refusal must name the type mismatch: {}",
        reason_of(&out)
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "an unfaithful checkout must never produce a receipt"
    );
}

/// A hook that **removes** the executable bit from a file the tree declares
/// executable. The bytes are identical, so only the mode proof can see it — and
/// the proof must not skip the one mode it exists to check.
#[cfg(unix)]
#[test]
fn a_hook_that_removes_the_executable_bit_from_a_declared_script_is_refused() {
    let repo = new_repo("hook-chmod-off");
    flow(&repo.path, &["init"]);
    exclude_flow(&repo.path);
    write_config(&repo.path, GREEN);
    fs::write(repo.path.join("script.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    git(&repo.path, &["add", "script.sh"]);
    // The bit is forced through the index, as the tree is what declares it.
    git(
        &repo.path,
        &["update-index", "--chmod=+x", "--", "script.sh"],
    );
    install_tracked_hook(&repo, "chmod -x script.sh\n");
    // `install_tracked_hook` stages everything with `git add -A`, which re-stages
    // `script.sh` from the 0644 worktree file and would reset the index mode to
    // 100644: the tree would then declare a non-executable file and the hook's
    // `chmod -x` would be a no-op, making this test pass vacuously. The bit is
    // forced **after** the hook is staged, so the tree really declares 100755.
    git(
        &repo.path,
        &["update-index", "--chmod=+x", "--", "script.sh"],
    );
    let tree = git(&repo.path, &["write-tree"]);

    let (code, out) = flow(&repo.path, &["run"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(status_of(&out), "REFUSED", "{out}");
    assert_eq!(code_of(&out), "UNSUPPORTED_CHECKOUT", "{out}");
    assert!(
        reason_of(&out).contains("non-executable"),
        "the refusal must name the lost executable bit: {}",
        reason_of(&out)
    );
    assert!(
        !receipt_file(&repo.path, &tree).exists(),
        "a checkout whose mode changed must never produce a receipt"
    );
}
