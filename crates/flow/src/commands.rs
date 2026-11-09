//! Flow CLI commands: init, doctor, status, run, publish, reconcile, explain,
//! disable/enable. Everything returns structured JSON; refusals are explicit.

use std::fs;
use std::path::{Path, PathBuf};

use crate::gitcli::{self, Git, Obligation};
use crate::json::{self, Json};
use crate::sha256;
use crate::state::{self, Config, Receipt};

fn refused(why: impl Into<String>) -> Json {
    Json::obj(vec![
        ("status", Json::str("REFUSED")),
        ("reason", Json::str(why.into())),
    ])
}

fn current_ref(git: &Git) -> Result<String, String> {
    let out = git.run(&["symbolic-ref", "-q", "HEAD"])?;
    if out.status != 0 {
        return Err("detached HEAD: pass --ref explicitly".into());
    }
    Ok(out.stdout.trim().to_string())
}

// ------------------------------------------------------------------- init
pub fn cmd_init(root: &Path, force: bool) -> Result<Json, String> {
    let git = Git::new(root);
    let _ = git.version();
    let dir = state::flow_dir(root);
    if dir.exists() && !force {
        return Ok(refused(
            ".flow already exists (pass --force to rewrite config, journal is preserved)",
        ));
    }
    fs::create_dir_all(state::receipts_dir(root))
        .map_err(|e| format!("cannot create receipts dir: {e}"))?;
    fs::create_dir_all(state::logs_dir(root))
        .map_err(|e| format!("cannot create logs dir: {e}"))?;
    if !state::journal_path(root).exists() {
        state::write_atomic(&state::journal_path(root), "")?;
    }
    let config = Config::default();
    state::save_config(root, &config)?;
    Ok(Json::obj(vec![
        ("status", Json::str("INITIALIZED")),
        ("flow_dir", Json::str(".flow")),
        ("policy_version", Json::str(state::POLICY_VERSION)),
        ("note", Json::str("no obligation declared yet: `flow run` will refuse with UNKNOWN until .flow/config.json lists obligations")),
    ]))
}

// ----------------------------------------------------------------- doctor
pub fn cmd_doctor(root: &Path) -> Result<Json, String> {
    let git = Git::new(root);
    let mut checks: Vec<Json> = Vec::new();
    let mut push = |id: &str, status: &str, detail: String| {
        checks.push(Json::obj(vec![
            ("id", Json::str(id)),
            ("status", Json::str(status)),
            ("detail", Json::str(detail)),
        ]));
    };
    let version = git.version();
    let git_ok = version.starts_with("git version");
    push("git", if git_ok { "OK" } else { "FAILED" }, version);

    push("repository", "OK", format!("toplevel {}", root.display()));

    let initialized = state::config_path(root).exists();
    push(
        "flow_initialized",
        if initialized { "OK" } else { "WARN" },
        if initialized {
            "config present".to_string()
        } else {
            "run `flow init`".to_string()
        },
    );

    // Hooks: the MVP does not execute them; the user must know.
    let hooks_dir = PathBuf::from(git.capture(&["rev-parse", "--git-path", "hooks"])?);
    let hooks_dir = if hooks_dir.is_absolute() {
        hooks_dir
    } else {
        root.join(hooks_dir)
    };
    let mut hooks: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&hooks_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".sample") {
                hooks.push(name);
            }
        }
    }
    hooks.sort();
    push(
        "hooks",
        if hooks.is_empty() { "OK" } else { "WARN" },
        if hooks.is_empty() {
            "no hook installed".to_string()
        } else {
            format!(
                "hooks present ({}) are NOT executed by Flow: obligations are declared in .flow/config.json",
                hooks.join(", ")
            )
        },
    );

    let longpaths = git
        .run(&["config", "--get", "core.longpaths"])
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    push(
        "longpaths",
        if cfg!(windows) && longpaths != "true" {
            "WARN"
        } else {
            "OK"
        },
        format!("core.longpaths={longpaths:?}"),
    );

    let ignorecase = git
        .run(&["config", "--get", "core.ignorecase"])
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    push(
        "case_sensitivity",
        if ignorecase == "true" { "WARN" } else { "OK" },
        format!(
            "core.ignorecase={ignorecase:?}: a case-insensitive worktree cannot hold two paths differing only by case"
        ),
    );

    let attrs = root.join(".gitattributes");
    push(
        "gitattributes",
        if attrs.exists() { "WARN" } else { "OK" },
        if attrs.exists() {
            ".gitattributes present: eol/filter rules can change bytes; the receipt binds the Git tree OID, not the worktree bytes".to_string()
        } else {
            "absent".to_string()
        },
    );

    let mut tools = Vec::new();
    for program in ["cargo", "rustc", "node", "python", "python3"] {
        tools.push(format!(
            "{program}={}",
            if program_on_path(program) {
                "found"
            } else {
                "missing"
            }
        ));
    }
    push("toolchains", "OK", tools.join(" "));

    push(
        "execution_boundary",
        "WARN",
        "local-trusted: obligations run with your user rights, with a scrubbed environment, but WITHOUT an OS sandbox. Untrusted code execution is UNSUPPORTED.".to_string(),
    );
    push(
        "platform",
        if cfg!(windows) { "WARN" } else { "OK" },
        format!(
            "os={}: AppContainer/Job Object containment is not qualified for rustc/cargo/node",
            std::env::consts::OS
        ),
    );
    let disabled = state::load_config(root)
        .map(|c| c.disabled)
        .unwrap_or(false);
    push(
        "disabled",
        if disabled { "WARN" } else { "OK" },
        format!("disabled={disabled}"),
    );

    Ok(Json::obj(vec![
        ("status", Json::str("REPORT")),
        ("overall", Json::str(if checks.iter().any(|c| c.get("status").and_then(Json::as_str) == Some("FAILED")) { "FAILED" } else { "OK" })),
        ("checks", Json::Arr(checks)),
        (
            "guarantees",
            Json::Arr(vec![
                Json::str("receipts bind an exact Git tree OID"),
                Json::str("publication uses ref CAS with a durable intent/outcome journal"),
                Json::str("a required obligation that cannot run makes the result UNSUPPORTED, never PASSED"),
            ]),
        ),
        (
            "non_guarantees",
            Json::Arr(vec![
                Json::str("no OS-level isolation of obligations"),
                Json::str("no execution of Git hooks"),
                Json::str("no completeness proof of the build context"),
            ]),
        ),
    ]))
}

fn program_on_path(program: &str) -> bool {
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
            .split(';')
            .map(|s| s.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let candidate = dir.join(format!("{program}{ext}"));
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

// ----------------------------------------------------------------- status
pub fn cmd_status(root: &Path) -> Result<Json, String> {
    let git = Git::new(root);
    let head = git.head_oid().unwrap_or_default();
    let tree = git.head_tree().unwrap_or_default();
    let branch = git
        .run(&["symbolic-ref", "--short", "-q", "HEAD"])
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    let status = git.status_porcelain()?;
    let dirty = status.iter().filter(|l| !l.starts_with("??")).count();
    let untracked = status.iter().filter(|l| l.starts_with("??")).count();
    let config = state::load_config(root).ok();
    let disabled = config.as_ref().map(|c| c.disabled).unwrap_or(false);
    let obligations = config.as_ref().map(|c| c.obligations.len()).unwrap_or(0);

    let mut latest = Json::Null;
    if let Ok(entries) = fs::read_dir(state::receipts_dir(root)) {
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
            .collect();
        files.sort();
        if let Some(last) = files.last() {
            if let Ok(text) = fs::read_to_string(last) {
                if let Ok(value) = json::parse(&text) {
                    latest = Json::obj(vec![
                        (
                            "tree_oid",
                            value.get("tree_oid").cloned().unwrap_or(Json::Null),
                        ),
                        ("status", value.get("status").cloned().unwrap_or(Json::Null)),
                        (
                            "context_id",
                            value.get("context_id").cloned().unwrap_or(Json::Null),
                        ),
                        (
                            "finished_at",
                            value.get("finished_at").cloned().unwrap_or(Json::Null),
                        ),
                    ]);
                }
            }
        }
    }
    let pending = pending_intents(root).len();

    Ok(Json::obj(vec![
        ("status", Json::str("REPORT")),
        ("branch", Json::str(branch)),
        ("head", Json::str(head)),
        ("tree", Json::str(tree)),
        ("dirty_count", Json::Num(dirty as f64)),
        ("untracked_count", Json::Num(untracked as f64)),
        ("obligations_declared", Json::Num(obligations as f64)),
        ("disabled", Json::Bool(disabled)),
        ("latest_receipt", latest),
        ("pending_intents", Json::Num(pending as f64)),
        (
            "hint",
            Json::str("`flow run` verifies the INDEX tree (git add first); the worktree is never modified"),
        ),
    ]))
}

// -------------------------------------------------------------------- run
pub fn cmd_run(
    root: &Path,
    only: Option<Vec<String>>,
    timeout_override: Option<u64>,
) -> Result<Json, String> {
    let config = state::load_config(root)?;
    if config.disabled {
        return Ok(refused(
            "Flow is disabled in this repository (`flow enable` to re-enable)",
        ));
    }
    if config.obligations.is_empty() {
        return Ok(Json::obj(vec![
            ("status", Json::str("UNKNOWN")),
            (
                "reason",
                Json::str("no obligation declared: verification scope is undefined, nothing is publishable"),
            ),
        ]));
    }
    let git = Git::new(root);
    let tree = git.write_tree()?;
    let started_at = state::iso8601_now();

    // Materialise the exact tree in a disposable directory, with its .git
    // link removed so an obligation cannot reach the shared Git directory.
    let parent = git.head_oid()?;
    let commit = git.commit_tree(&tree, &parent, &format!("flow verify {tree}"))?;
    let scratch = std::env::temp_dir().join(format!("flow-verify-{}-{}", std::process::id(), tree));
    if scratch.exists() {
        let _ = fs::remove_dir_all(&scratch);
    }
    let add = git.run(&[
        "worktree",
        "add",
        "--detach",
        "--quiet",
        scratch.to_string_lossy().as_ref(),
        &commit,
    ])?;
    if add.status != 0 {
        return Err(format!(
            "cannot materialise tree {tree}: {}",
            add.stderr.trim()
        ));
    }
    let gitfile = scratch.join(".git");
    if gitfile.is_file() {
        let _ = fs::remove_file(&gitfile);
    }

    let selected: Vec<Obligation> = match &only {
        None => config.obligations.clone(),
        Some(ids) => config
            .obligations
            .iter()
            .filter(|o| ids.contains(&o.id))
            .cloned()
            .collect(),
    };
    if selected.is_empty() {
        let _ = fs::remove_dir_all(&scratch);
        let _ = git.run(&["worktree", "prune"]);
        return Ok(refused("none of the requested obligation ids is declared"));
    }
    // An obligation id becomes a log file name: refuse anything that is not a
    // plain repository-relative component (path injection defence).
    for ob in &selected {
        if let Err(reason) = crate::paths::validate_relative(&format!(".flow/logs/{}", ob.id)) {
            let _ = fs::remove_dir_all(&scratch);
            let _ = git.run(&["worktree", "prune"]);
            return Ok(refused(format!(
                "unsafe obligation id {:?}: {reason}",
                ob.id
            )));
        }
    }
    // The verification directory must never live inside the repository.
    if crate::paths::lexically_inside(root, &scratch) {
        let _ = fs::remove_dir_all(&scratch);
        let _ = git.run(&["worktree", "prune"]);
        return Ok(refused(
            "refusing to verify inside the repository working tree",
        ));
    }

    let context_id = build_context_id(&git, &tree, &selected);
    let log_dir = state::logs_dir(root).join(&tree);
    let mut outcomes = Vec::new();
    let mut logs = Vec::new();
    let mut status = "PASSED";
    for ob in &selected {
        let mut spec = ob.clone();
        if let Some(t) = timeout_override {
            spec.timeout_s = t;
        }
        let outcome = gitcli::run_obligation(&spec, &scratch);
        let stdout_clean = gitcli::redact(&outcome.stdout);
        let stderr_clean = gitcli::redact(&outcome.stderr);
        let stdout_rel = format!(".flow/logs/{tree}/{}.stdout.log", outcome.id);
        let stderr_rel = format!(".flow/logs/{tree}/{}.stderr.log", outcome.id);
        fs::create_dir_all(&log_dir).map_err(|e| format!("cannot create log dir: {e}"))?;
        let _ = crate::paths::canonical_within(root, &log_dir)?;
        state::write_atomic(
            &log_dir.join(format!("{}.stdout.log", outcome.id)),
            &stdout_clean,
        )?;
        state::write_atomic(
            &log_dir.join(format!("{}.stderr.log", outcome.id)),
            &stderr_clean,
        )?;
        logs.push(stdout_rel);
        logs.push(stderr_rel);
        if ob.required {
            match outcome.status {
                "FAILED" => status = "FAILED",
                "TIMEOUT" if status == "PASSED" => status = "TIMEOUT",
                "UNSUPPORTED" if status == "PASSED" => status = "UNSUPPORTED",
                _ => {}
            }
        }
        outcomes.push(Json::obj(vec![
            ("id", Json::str(ob.id.clone())),
            ("status", Json::str(outcome.status)),
            ("required", Json::Bool(ob.required)),
            (
                "exit",
                outcome
                    .exit
                    .map(|c| Json::Num(c as f64))
                    .unwrap_or(Json::Null),
            ),
            ("duration_ms", Json::Num(outcome.duration_ms as f64)),
            ("timed_out", Json::Bool(outcome.timed_out)),
            ("stdout_preview", Json::str(preview(&stdout_clean, 400))),
            ("stderr_preview", Json::str(preview(&stderr_clean, 400))),
        ]));
    }

    let worktree_diff = git
        .capture(&["diff", "--name-only", &tree])
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    let _ = fs::remove_dir_all(&scratch);
    let _ = git.run(&["worktree", "prune"]);

    let receipt = Json::obj(vec![
        ("schema", Json::str(state::SCHEMA_RECEIPT)),
        ("tree_oid", Json::str(tree.clone())),
        ("commit_oid", Json::str(commit)),
        ("parent_oid", Json::str(parent)),
        ("context_id", Json::str(context_id)),
        ("policy_version", Json::str(config.policy_version.clone())),
        (
            "execution_boundary",
            Json::str(config.execution_boundary.clone()),
        ),
        ("status", Json::str(status)),
        ("started_at", Json::str(started_at)),
        ("finished_at", Json::str(state::iso8601_now())),
        ("obligations", Json::Arr(outcomes)),
        (
            "logs",
            Json::Arr(logs.iter().map(|l| Json::str(l.clone())).collect()),
        ),
        (
            "worktree_diff",
            Json::Arr(worktree_diff.iter().map(|p| Json::str(p.clone())).collect()),
        ),
        (
            "scope_note",
            Json::str(
                "verified for the declared obligations only; undeclared inputs are not covered",
            ),
        ),
    ]);
    let path = state::save_receipt(root, &receipt)?;
    let mut out = receipt.clone();
    if let Json::Obj(pairs) = &mut out {
        pairs.push((
            "receipt_path".into(),
            Json::str(path.to_string_lossy().to_string()),
        ));
    }
    Ok(out)
}

fn preview(text: &str, limit: usize) -> String {
    let mut out: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        out.push('…');
    }
    out
}

fn build_context_id(git: &Git, tree: &str, obligations: &[Obligation]) -> String {
    let obs = obligations
        .iter()
        .map(|o| {
            Json::obj(vec![
                ("id", Json::str(o.id.clone())),
                ("program", Json::str(o.program.clone())),
                (
                    "args",
                    Json::Arr(o.args.iter().map(|a| Json::str(a.clone())).collect()),
                ),
            ])
        })
        .collect();
    let context = Json::obj(vec![
        ("tree_oid", Json::str(tree)),
        ("policy_version", Json::str(state::POLICY_VERSION)),
        ("git_version", Json::str(git.version())),
        ("os", Json::str(std::env::consts::OS)),
        ("arch", Json::str(std::env::consts::ARCH)),
        ("obligations", Json::Arr(obs)),
    ]);
    sha256::sha256_hex(context.to_string().as_bytes())
}

// ---------------------------------------------------------------- publish
pub fn cmd_publish(
    root: &Path,
    reference: Option<String>,
    expected_old: Option<String>,
    message: Option<String>,
) -> Result<Json, String> {
    let config = state::load_config(root)?;
    if config.disabled {
        return Ok(refused("Flow is disabled in this repository"));
    }
    let git = Git::new(root);
    // Publish exactly the tree that `run` verified: the index tree. A receipt is
    // bound to that OID, so a different index (or HEAD) can never be published by
    // accident.
    let tree = git.write_tree()?;
    let receipt = match Receipt::load(root, &tree)? {
        Some(r) => r,
        None => {
            return Ok(refused(format!(
                "no receipt for tree {tree}: run `flow run` and verify this exact tree first"
            )))
        }
    };
    if receipt.tree_oid != tree {
        return Ok(refused(format!(
            "receipt integrity error: file for {tree} names tree {:?}",
            receipt.tree_oid
        )));
    }
    if receipt.status != "PASSED" {
        return Ok(refused(format!(
            "receipt status is {}: a non-PASSED receipt is never publishable",
            receipt.status
        )));
    }
    let reference = match reference {
        Some(r) => r,
        None => current_ref(&git)?,
    };
    let old = match expected_old {
        Some(o) => o,
        None => match git.show_ref(&reference)? {
            Some(o) => o,
            None => return Ok(refused(format!("cannot resolve {reference}"))),
        },
    };
    let current = git.show_ref(&reference)?;
    if current.as_deref() != Some(old.as_str()) {
        return Ok(refused(format!(
            "stale expected old OID: ref is {:?}, expected {old}",
            current
        )));
    }
    let message = message.unwrap_or_else(|| format!("flow: publish verified tree {tree}"));
    let commit = git.commit_tree(&tree, &old, &message)?;
    let id = format!(
        "{}-{}",
        state::iso8601_now(),
        &commit[..12.min(commit.len())]
    );
    state::append_journal(
        root,
        &Json::obj(vec![
            ("event", Json::str("intent")),
            ("id", Json::str(id.clone())),
            ("ref", Json::str(reference.clone())),
            ("old", Json::str(old.clone())),
            ("new", Json::str(commit.clone())),
            ("tree", Json::str(tree.clone())),
            ("at", Json::str(state::iso8601_now())),
        ]),
    )?;
    if std::env::var("FLOW_TEST_CRASH").as_deref() == Ok("after-intent") {
        std::process::exit(89);
    }
    let recheck = git.show_ref(&reference)?;
    if recheck.as_deref() != Some(old.as_str()) {
        state::append_journal(
            root,
            &Json::obj(vec![
                ("event", Json::str("outcome")),
                ("id", Json::str(id)),
                ("status", Json::str("REFUSED_STALE")),
                ("observed", Json::str(recheck.unwrap_or_default())),
                ("at", Json::str(state::iso8601_now())),
            ]),
        )?;
        return Ok(refused(
            "ref moved between intent and CAS: nothing was published",
        ));
    }
    let res = git.update_ref(&reference, &commit, &old)?;
    if res.status != 0 {
        state::append_journal(
            root,
            &Json::obj(vec![
                ("event", Json::str("outcome")),
                ("id", Json::str(id)),
                ("status", Json::str("REFUSED")),
                ("stderr", Json::str(gitcli::redact(res.stderr.trim()))),
                ("at", Json::str(state::iso8601_now())),
            ]),
        )?;
        return Ok(refused(format!(
            "git update-ref refused: {}",
            res.stderr.trim()
        )));
    }
    if std::env::var("FLOW_TEST_CRASH").as_deref() == Ok("after-ref") {
        std::process::exit(89);
    }
    state::append_journal(
        root,
        &Json::obj(vec![
            ("event", Json::str("outcome")),
            ("id", Json::str(id)),
            ("status", Json::str("PUBLISHED")),
            ("commit", Json::str(commit.clone())),
            ("at", Json::str(state::iso8601_now())),
        ]),
    )?;
    Ok(Json::obj(vec![
        ("status", Json::str("PUBLISHED")),
        ("ref", Json::str(reference)),
        ("old", Json::str(old)),
        ("commit", Json::str(commit)),
        ("tree", Json::str(tree)),
        ("context_id", Json::str(receipt.context_id.clone())),
        ("policy_version", Json::str(receipt.policy_version.clone())),
        (
            "execution_boundary",
            Json::str(receipt.execution_boundary.clone()),
        ),
        ("receipt_logs", Json::Num(receipt.logs.len() as f64)),
        ("receipt_started_at", Json::str(receipt.started_at.clone())),
        (
            "receipt_finished_at",
            Json::str(receipt.finished_at.clone()),
        ),
        ("receipt_commit", Json::str(receipt.commit_oid.clone())),
        (
            "obligations",
            Json::Arr(
                receipt
                    .obligations
                    .iter()
                    .filter_map(|o| {
                        o.get("id")
                            .and_then(Json::as_str)
                            .map(|id| Json::str(id.to_string()))
                    })
                    .collect(),
            ),
        ),
        (
            "worktree_diff",
            Json::Arr(
                receipt
                    .worktree_diff
                    .iter()
                    .map(|p| Json::str(p.clone()))
                    .collect(),
            ),
        ),
        (
            "note",
            Json::str("only refs were moved; the working tree was not modified and may now differ"),
        ),
    ]))
}

// -------------------------------------------------------------- reconcile
fn pending_intents(root: &Path) -> Vec<Json> {
    let Ok(records) = state::read_journal(root) else {
        return Vec::new();
    };
    let mut done: Vec<String> = Vec::new();
    for record in &records {
        if record.get("event").and_then(Json::as_str) == Some("outcome") {
            if let Some(id) = record.get("id").and_then(Json::as_str) {
                done.push(id.to_string());
            }
        }
    }
    records
        .into_iter()
        .filter(|r| r.get("event").and_then(Json::as_str) == Some("intent"))
        .filter(|r| {
            r.get("id")
                .and_then(Json::as_str)
                .map(|id| !done.contains(&id.to_string()))
                .unwrap_or(false)
        })
        .collect()
}

pub fn cmd_reconcile(root: &Path) -> Result<Json, String> {
    let git = Git::new(root);
    let pending = pending_intents(root);
    let mut resolved = Vec::new();
    for intent in &pending {
        let id = intent
            .get("id")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let reference = intent
            .get("ref")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let old = intent
            .get("old")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let new = intent
            .get("new")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let observed = git.show_ref(&reference)?.unwrap_or_default();
        let status = if observed == new {
            "PUBLISHED"
        } else if observed == old {
            "NOT_PUBLISHED"
        } else {
            "NEEDS_RECONCILIATION"
        };
        state::append_journal(
            root,
            &Json::obj(vec![
                ("event", Json::str("outcome")),
                ("id", Json::str(id.clone())),
                ("status", Json::str(format!("RECONCILED_{status}"))),
                ("observed", Json::str(observed.clone())),
                ("at", Json::str(state::iso8601_now())),
            ]),
        )?;
        resolved.push(Json::obj(vec![
            ("id", Json::str(id)),
            ("ref", Json::str(reference)),
            ("observed", Json::str(observed)),
            ("resolution", Json::str(status)),
        ]));
    }
    Ok(Json::obj(vec![
        ("status", Json::str("RECONCILED")),
        ("resolved", Json::Arr(resolved)),
        (
            "note",
            Json::str("reconciliation never moves a ref: it only records what is observable"),
        ),
    ]))
}

// ----------------------------------------------------------------- explain
pub fn cmd_explain(root: &Path, tree: Option<String>) -> Result<Json, String> {
    let git = Git::new(root);
    let tree = match tree {
        Some(t) => t,
        None => git.head_tree()?,
    };
    let receipt = Receipt::load(root, &tree)?;
    let records = state::read_journal(root)?;
    let related: Vec<Json> = records
        .into_iter()
        .filter(|r| r.get("tree").and_then(Json::as_str) == Some(tree.as_str()))
        .collect();
    Ok(Json::obj(vec![
        ("status", Json::str("EXPLAIN")),
        ("tree", Json::str(tree)),
        ("receipt", receipt.map(|r| r.json).unwrap_or(Json::Null)),
        ("journal", Json::Arr(related)),
    ]))
}

// --------------------------------------------------------------- disable
pub fn cmd_set_disabled(root: &Path, disabled: bool) -> Result<Json, String> {
    let mut config = state::load_config(root)?;
    config.disabled = disabled;
    state::save_config(root, &config)?;
    Ok(Json::obj(vec![
        (
            "status",
            Json::str(if disabled { "DISABLED" } else { "ENABLED" }),
        ),
        ("disabled", Json::Bool(disabled)),
    ]))
}
