//! Flow CLI commands: init, doctor, status, run, publish, reconcile, explain,
//! disable/enable. Everything returns structured JSON; refusals are explicit.
//!
//! The publication gate is the whole point of this crate, so the rules are
//! stated once, here, and enforced in one place:
//!
//! * a receipt proves exactly one *effective policy* on exactly one Git tree;
//! * a selective (`--obligation`) run is diagnostic and never publishable;
//! * `publish` recomputes the policy identity instead of trusting the receipt;
//! * recovery observes, never guesses, and refuses to report a corrupt journal
//!   as a success.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::gitcli::{self, Git, Obligation};
use crate::json::{self, Json};
use crate::sha256;
use crate::state::{self, Config, Receipt};

mod checkout;
mod policy;

pub(crate) use checkout::*;
pub(crate) use policy::*;

fn refused(why: impl Into<String>) -> Json {
    Json::obj(vec![
        ("status", Json::str("REFUSED")),
        ("reason", Json::str(why.into())),
    ])
}

/// A refusal that carries a stable machine-readable code. The `status` stays
/// `REFUSED` so callers (and the exit-code contract) never have to special-case
/// the reason, while tests and operators can assert on the precise cause.
fn refused_code(code: &str, why: impl Into<String>) -> Json {
    Json::obj(vec![
        ("status", Json::str("REFUSED")),
        ("code", Json::str(code)),
        ("reason", Json::str(why.into())),
    ])
}

/// Removes the run's active marker on every path out of `cmd_run`, including
/// refusals and errors. `gc` refuses while a live marker names the tree.
struct RunMarker {
    path: PathBuf,
}

impl Drop for RunMarker {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn short(digest: &str) -> String {
    digest.chars().take(12).collect()
}

fn current_ref(git: &Git) -> Result<String, String> {
    let out = git.run(&["symbolic-ref", "-q", "HEAD"])?;
    if out.status != 0 {
        return Err("detached HEAD: pass --ref explicitly".into());
    }
    Ok(out.stdout.trim().to_string())
}

// ------------------------------------------------------------------- init
pub fn cmd_init(root: &Path, force: bool, adopt_policy: bool) -> Result<Json, String> {
    let git = Git::new(root);
    let _ = git.version();
    let dir = state::flow_dir(root);
    // A `.flow/config.json` can arrive with a clone. It is a claim about what
    // should run, not a consent to run it: adoption is a separate, explicit act.
    let existing_config = state::config_path(root).exists();
    if dir.exists() && !force && !(adopt_policy && existing_config) {
        return Ok(refused(
            ".flow already exists (pass --force to rewrite config, journal is preserved)",
        ));
    }
    let config = if existing_config {
        match state::load_config(root) {
            Ok(config) => config,
            Err(e) => {
                return Ok(refused_code(
                    "CONFIG_UNREADABLE",
                    format!("a .flow/config.json is present but cannot be used: {e}"),
                ))
            }
        }
    } else {
        Config::default()
    };
    if let Err(reason) = validate_obligations(&config.obligations) {
        if !config.obligations.is_empty() {
            return Ok(refused_code("INVALID_POLICY", reason));
        }
    }
    let policy_digest = policy_digest(&effective_policy(&config, &config.obligations));
    let adopted_already = state::load_adoption(root)?
        .map(|a| a.policy_digest == policy_digest)
        .unwrap_or(false);

    if existing_config && !adopted_already && !adopt_policy {
        // Show exactly what would be executed. Silence is not consent.
        let commands: Vec<Json> = config
            .obligations
            .iter()
            .map(|o| {
                Json::str(format!(
                    "{} {}{}",
                    o.program,
                    o.args.join(" "),
                    if o.required {
                        " (required)"
                    } else {
                        " (optional)"
                    }
                ))
            })
            .collect();
        let mut value = refused_code(
            "POLICY_ADOPTION_REQUIRED",
            "this working copy already contains a .flow/config.json that was never approved here. Review it, then run `flow init --adopt-policy` or `flow approve`",
        );
        if let Json::Obj(pairs) = &mut value {
            pairs.push(("policy_digest".into(), Json::str(short(&policy_digest))));
            pairs.push(("obligations".into(), Json::Arr(commands)));
        }
        return Ok(value);
    }

    fs::create_dir_all(state::receipts_dir(root))
        .map_err(|e| format!("cannot create receipts dir: {e}"))?;
    fs::create_dir_all(state::logs_dir(root))
        .map_err(|e| format!("cannot create logs dir: {e}"))?;
    // Never truncate a journal, a receipt or a proposal: `init` is not a reset.
    if !state::journal_path(root).exists() {
        state::write_atomic(&state::journal_path(root), "")?;
    }
    // Flow's own state must never become part of the tree it verifies: if it
    // did, every policy change would move the tree OID and a receipt could
    // never be compared with "the same tree under a different policy".
    let gitignore = dir.join(".gitignore");
    if !gitignore.exists() {
        state::write_atomic(&gitignore, "*\n")?;
    }
    if !existing_config {
        state::save_config(root, &config)?;
    }
    let source = if adopt_policy {
        "init --adopt-policy"
    } else {
        "init"
    };
    state::save_adoption(
        root,
        &state::Adoption {
            policy_digest: policy_digest.clone(),
            policy_version: config.policy_version.clone(),
            approved_at: state::iso8601_now(),
            source: source.into(),
        },
    )?;
    Ok(Json::obj(vec![
        ("status", Json::str("INITIALIZED")),
        ("flow_dir", Json::str(".flow")),
        ("policy_version", Json::str(state::POLICY_VERSION)),
        ("adopted", Json::Bool(true)),
        ("adoption_source", Json::str(source)),
        ("policy_digest", Json::str(short(&policy_digest))),
        (
            "journal_preserved",
            Json::Bool(state::journal_path(root).exists()),
        ),
        (
            "note",
            Json::str(if config.obligations.is_empty() {
                "no obligation declared yet: `flow run` will refuse with UNKNOWN until .flow/config.json lists obligations; .flow/.gitignore keeps Flow state out of the verified tree"
            } else {
                "the adopted policy declares obligations; `.flow/.gitignore` keeps Flow state out of the verified tree"
            }),
        ),
    ]))
}

/// Record local approval of the current `.flow/config.json` policy.
pub fn cmd_approve(root: &Path) -> Result<Json, String> {
    let config = state::load_config(root)?;
    if let Err(reason) = validate_obligations(&config.obligations) {
        return Ok(refused_code("INVALID_POLICY", reason));
    }
    let digest = policy_digest(&effective_policy(&config, &config.obligations));
    state::save_adoption(
        root,
        &state::Adoption {
            policy_digest: digest.clone(),
            policy_version: config.policy_version.clone(),
            approved_at: state::iso8601_now(),
            source: "approve".into(),
        },
    )?;
    Ok(Json::obj(vec![
        ("status", Json::str("APPROVED")),
        ("policy_digest", Json::str(digest)),
        ("policy_version", Json::str(config.policy_version.clone())),
        (
            "obligations",
            Json::Arr(
                config
                    .obligations
                    .iter()
                    .map(|o| {
                        Json::str(format!(
                            "{} {}{}",
                            o.program,
                            o.args.join(" "),
                            if o.required { " (required)" } else { " (optional)" }
                        ))
                    })
                    .collect(),
            ),
        ),
        (
            "context_probes",
            Json::Arr(
                config
                    .context_probes
                    .iter()
                    .map(|probe| Json::str(probe.join(" ")))
                    .collect(),
            ),
        ),
        (
            "note",
            Json::str("this working copy now approves exactly these obligations and context probes; editing .flow/config.json again requires `flow approve`"),
        ),
    ]))
}

/// The adoption gate: a policy that this working copy never approved is not run.
fn adoption_gate(root: &Path, config: &Config) -> Result<Option<Json>, String> {
    let digest = policy_digest(&effective_policy(config, &config.obligations));
    match state::load_adoption(root)? {
        None => Ok(Some(refused_code(
            "POLICY_NOT_ADOPTED",
            "this working copy has no adoption record for .flow/config.json: nothing in it will be executed. Run `flow init --adopt-policy` (or `flow approve`) after reading the policy",
        ))),
        Some(adoption) if adoption.policy_digest != digest => Ok(Some(refused_code(
            "POLICY_NOT_APPROVED",
            format!(
                ".flow/config.json changed since it was approved (approved {}, current {}): review it, then run `flow approve`",
                short(&adoption.policy_digest),
                short(&digest)
            ),
        ))),
        Some(_) => Ok(None),
    }
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

    // A policy is executed only after this working copy adopted it. `doctor`
    // states which of the three situations the repository is in.
    if initialized {
        match (state::load_config(root), state::load_adoption(root)) {
            (Ok(config), Ok(Some(adoption))) => {
                let digest = policy_digest(&effective_policy(&config, &config.obligations));
                if digest == adoption.policy_digest {
                    push(
                        "policy_adopted",
                        "OK",
                        format!(
                            "{} obligation(s) approved by {} at {}",
                            config.obligations.len(),
                            adoption.source,
                            adoption.approved_at
                        ),
                    );
                } else {
                    push(
                        "policy_adopted",
                        "WARN",
                        "the policy changed since it was approved: `flow approve` after reviewing .flow/config.json".to_string(),
                    );
                }
            }
            (Ok(_), Ok(None)) => push(
                "policy_adopted",
                "WARN",
                "no adoption record: nothing in .flow/config.json will run until `flow init --adopt-policy` or `flow approve`".to_string(),
            ),
            (Err(e), _) => push("policy_adopted", "FAILED", e),
            (_, Err(e)) => push("policy_adopted", "FAILED", e),
        }
    }

    let lock = crate::lock::inspect(root);
    push(
        "publication_lock",
        match lock.attribution {
            "absent" | "dead" => "OK",
            "live" => "WARN",
            _ => "FAILED",
        },
        if !lock.present {
            "no lock file".to_string()
        } else {
            format!(
                "present, owner attribution {}, purpose {:?}{}{}",
                lock.attribution,
                lock.purpose,
                lock.pid.map(|p| format!(", pid {p}")).unwrap_or_default(),
                lock.age_s
                    .map(|a| format!(", {a}s old"))
                    .unwrap_or_default()
            )
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
            "no hook installed. Flow dispatches no hook of its own; the Git plumbing it calls can still run post-checkout and reference-transaction if you add them".to_string()
        } else {
            format!(
                "hooks present ({}): Flow dispatches none of its own, but the Git plumbing it calls runs post-checkout on `worktree add` and reference-transaction on `update-ref`, and a blocking reference-transaction hook aborts publish. Obligations are declared in .flow/config.json",
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

    // Attribute sources that can rewrite a checkout. They are enumerated here for
    // disclosure only: `flow run` no longer guesses whether a conversion "can"
    // apply, it compares every regular file against its blob and refuses a
    // checkout that differs, so no attribute source (committed or host-local) is
    // a bypass. The list tells an operator *why* a refusal may have happened.
    let mut attribute_sources: Vec<String> = Vec::new();
    if root.join(".gitattributes").exists() {
        attribute_sources.push(".gitattributes (in the working tree)".to_string());
    }
    if root.join(".git").join("info").join("attributes").exists() {
        attribute_sources.push(".git/info/attributes (local, not in the tree)".to_string());
    }
    let attributes_file = git
        .run(&["config", "--get", "core.attributesFile"])
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    if !attributes_file.is_empty() {
        attribute_sources.push(format!("core.attributesFile={attributes_file}"));
    }
    push(
        "gitattributes",
        if attribute_sources.is_empty() {
            "OK"
        } else {
            "WARN"
        },
        if attribute_sources.is_empty() {
            "no content-attribute source present".to_string()
        } else {
            format!(
                "content-attribute source(s) present ({}): eol/filter rules can rewrite a checkout; `flow run` compares every regular file with the blob the tree names and refuses a checkout that differs, so this is a disclosure, not a bypass",
                attribute_sources.join(", ")
            )
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

    let journal = match state::read_journal(root) {
        Ok(records) => Json::obj(vec![
            ("status", Json::str("OK")),
            ("records", Json::Num(records.len() as f64)),
        ]),
        Err(e) => Json::obj(vec![
            ("status", Json::str("FAILED")),
            ("detail", Json::str(e.to_string())),
        ]),
    };
    push(
        "journal",
        journal
            .get("status")
            .and_then(Json::as_str)
            .unwrap_or("FAILED"),
        journal
            .get("detail")
            .and_then(Json::as_str)
            .map(str::to_string)
            .or_else(|| {
                journal
                    .get("records")
                    .and_then(Json::as_num)
                    .map(|n| format!("{n} records readable"))
            })
            .unwrap_or_else(|| "unreadable".to_string()),
    );

    Ok(Json::obj(vec![
        ("status", Json::str("REPORT")),
        ("overall", Json::str(if checks.iter().any(|c| c.get("status").and_then(Json::as_str) == Some("FAILED")) { "FAILED" } else { "OK" })),
        ("exit_contract", Json::str("0 when overall=OK, 1 when overall=FAILED (a diagnostic that found a broken guarantee is not a success)")),
        ("checks", Json::Arr(checks)),
        (
            "guarantees",
            Json::Arr(vec![
                Json::str("receipts bind an exact Git tree OID and one effective policy digest"),
                Json::str("publication uses ref CAS with a durable intent/outcome journal"),
                Json::str("the verification checkout equals the verified tree exactly when it is checked: every regular file byte for byte, executable bit on Unix, no missing file, no extra file or directory (Flow's own scratch ownership marker excepted) — and the declared files are re-proved unchanged after the obligations ran (a change refuses the run)"),
                Json::str("a required obligation that cannot run makes the result UNSUPPORTED, never PASSED"),
                Json::str("a selective run is recorded as PARTIAL and is never publishable"),
            ]),
        ),
        (
            "non_guarantees",
            Json::Arr(vec![
                Json::str("no OS-level isolation of obligations"),
                Json::str("hooks are not suppressed: `git worktree add` runs post-checkout and `git update-ref` runs reference-transaction. A change already present when the checkout is checked is refused, and a change to a declared file while the obligations run is caught by a re-check after the last obligation; a hook that writes outside the checkout, a write reverted before the re-check, or a write after it is not detected. A blocking reference-transaction aborts publish"),
                Json::str("no completeness proof of the build context"),
                Json::str("no cryptographic protection of a receipt: anyone who can write .flow/receipts can forge one"),
                Json::str("a checkout that cannot reproduce the tree is refused, not repaired"),
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
                    // A receipt whose content does not match its integrity digest
                    // is not evidence. It is shown for inspection, never as PASSED.
                    let trust = match state::verify_receipt_integrity(&value) {
                        Ok(()) => "INTEGRITY_OK",
                        Err(_) => "RECEIPT_INTEGRITY_INVALID",
                    };
                    let displayed_status = if trust == "INTEGRITY_OK" {
                        value.get("status").cloned().unwrap_or(Json::Null)
                    } else {
                        Json::str("RECEIPT_INTEGRITY_INVALID")
                    };
                    let logs: Vec<String> = value
                        .get("logs")
                        .and_then(Json::as_arr)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    let missing: Vec<String> = logs
                        .iter()
                        .filter(|rel| !root.join(rel.as_str()).is_file())
                        .cloned()
                        .collect();
                    latest = Json::obj(vec![
                        (
                            "tree_oid",
                            value.get("tree_oid").cloned().unwrap_or(Json::Null),
                        ),
                        ("status", displayed_status),
                        ("trust", Json::str(trust)),
                        (
                            "evidence_complete",
                            Json::Bool(trust == "INTEGRITY_OK" && missing.is_empty()),
                        ),
                        (
                            "missing_logs",
                            Json::Arr(missing.into_iter().map(Json::str).collect()),
                        ),
                        (
                            "scope",
                            value
                                .get("verification_scope")
                                .and_then(|s| s.get("mode"))
                                .cloned()
                                .unwrap_or(Json::Null),
                        ),
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
    // An unreadable journal must not be reported as "nothing pending".
    let pending = pending_intents(root)?.len();

    // Which policy this working copy believes it is running, and who approved it.
    let policy = match (&config, state::load_adoption(root)) {
        (Some(config), Ok(adoption)) => {
            let digest = policy_digest(&effective_policy(config, &config.obligations));
            Json::obj(vec![
                ("path", Json::str(".flow/config.json".to_string())),
                ("policy_version", Json::str(config.policy_version.clone())),
                ("digest", Json::str(digest.clone())),
                ("obligations", Json::Num(config.obligations.len() as f64)),
                (
                    "adopted",
                    Json::Bool(
                        adoption
                            .as_ref()
                            .map(|a| a.policy_digest == digest)
                            .unwrap_or(false),
                    ),
                ),
                (
                    "adopted_at",
                    adoption
                        .as_ref()
                        .map(|a| Json::str(a.approved_at.clone()))
                        .unwrap_or(Json::Null),
                ),
                (
                    "adoption_source",
                    adoption
                        .as_ref()
                        .map(|a| Json::str(a.source.clone()))
                        .unwrap_or(Json::Null),
                ),
            ])
        }
        (Some(_), Err(e)) => Json::obj(vec![("error", Json::str(e))]),
        (None, _) => Json::Null,
    };
    Ok(Json::obj(vec![
        ("status", Json::str("REPORT")),
        ("schema", Json::str(crate::state::SCHEMA_STATUS)),
        ("policy", policy),
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
    candidate: Option<String>,
) -> Result<Json, String> {
    let config = state::load_config(root)?;
    if config.disabled {
        return Ok(refused_code(
            "DISABLED",
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
    // The effective policy includes any --timeout override: the receipt must
    // prove the deadline that was actually applied, not the configured one.
    let mut effective: Vec<Obligation> = config.obligations.clone();
    if let Some(t) = timeout_override {
        for ob in &mut effective {
            ob.timeout_s = t;
        }
    }
    if let Err(reason) = validate_obligations(&effective) {
        return Ok(refused_code("INVALID_POLICY", reason));
    }
    if let Some(refusal) = adoption_gate(root, &config)? {
        return Ok(refusal);
    }
    // The tree to verify is either the immutable candidate tree or the index.
    let candidate_record = match candidate.as_deref() {
        None => None,
        Some(id) => match crate::proposal::Candidate::load(root, id)? {
            Some(record) => Some(record),
            None => {
                return Ok(refused_code(
                    "UNKNOWN_CANDIDATE",
                    format!("no candidate {id:?} in .flow/candidates/ (see `flow proposals list` after `flow assemble`)"),
                ))
            }
        },
    };

    let selection: Option<Vec<Obligation>> = match &only {
        None => None,
        Some(ids) => {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            for id in ids {
                if !seen.insert(id.as_str()) {
                    return Ok(refused_code(
                        "DUPLICATE_OBLIGATION_ID",
                        format!(
                            "obligation {id:?} is requested more than once: a selection is a set"
                        ),
                    ));
                }
                if !effective.iter().any(|o| &o.id == id) {
                    return Ok(refused_code(
                        "UNKNOWN_OBLIGATION_ID",
                        format!("obligation {id:?} is not declared in .flow/config.json"),
                    ));
                }
            }
            Some(
                effective
                    .iter()
                    .filter(|o| ids.contains(&o.id))
                    .cloned()
                    .collect(),
            )
        }
    };
    // A selection is diagnostic only: it cannot prove the whole policy, so it
    // is reported as PARTIAL and is never written to the complete receipt slot.
    let scope_mode = if selection.is_some() {
        "selected"
    } else {
        "complete"
    };
    let selected: Vec<Obligation> = selection.unwrap_or_else(|| effective.clone());

    let git = Git::new(root);
    let tree = match &candidate_record {
        Some(record) => record.tree_oid.clone(),
        None => index_tree_without_locking(&git)?,
    };
    // A candidate tree must still be in the object store: a pruned candidate is
    // not verifiable, and verifying something else while naming it would be a
    // false receipt.
    if candidate_record.is_some() {
        let present =
            git.run_bounded(&["cat-file", "-e", &format!("{tree}^{{tree}}")], None, None)?;
        if present.status != 0 {
            return Ok(refused_code(
                "CANDIDATE_TREE_MISSING",
                format!("candidate tree {tree} is no longer in the object store: re-run `flow assemble`"),
            ));
        }
    }
    let started_at = state::iso8601_now();
    // Identity of this execution. Every log goes under it, so two concurrent
    // runs of the same tree can never overwrite one another's evidence.
    let run_id = format!("{}-{:x}", std::process::id(), state::unique_suffix());
    let runs_dir = state::runs_dir(root);
    fs::create_dir_all(&runs_dir).map_err(|e| format!("cannot create runs dir: {e}"))?;
    let marker_path = runs_dir.join(format!("{run_id}.active"));
    let marker = Json::obj(vec![
        ("schema", Json::str("flow-run-active/1")),
        ("run_id", Json::str(run_id.clone())),
        ("pid", Json::Num(std::process::id() as f64)),
        ("tree", Json::str(tree.clone())),
        ("started_at", Json::str(started_at.clone())),
    ]);
    state::write_atomic(&marker_path, &marker.to_string())?;
    let _run_marker = RunMarker { path: marker_path };

    // Materialise the exact tree in a directory Flow owns and knows it created:
    // the `.git` link is removed so an obligation cannot reach the shared gitdir.
    // `Scratch` cleans up in `Drop`, including on the error paths below.
    // A candidate is verified on its own base, so the verification commit's
    // parent is the base the candidate was assembled from — not today's HEAD.
    let parent = match &candidate_record {
        Some(record) => record.base_commit.clone(),
        None => git.head_oid()?,
    };
    let commit = git.commit_tree(&tree, &parent, &format!("flow verify {tree}"))?;
    let scratch = match crate::scratch::materialise(&git, &commit, root) {
        Ok(scratch) => scratch,
        Err(reason) => return Ok(refused_code("SCRATCH_REFUSED", reason)),
    };
    // A checkout that does not reproduce the tree exactly is not the tree.
    if let Err(reason) = verify_checkout_fidelity(&git, scratch.path(), &tree) {
        return Ok(refused_code("UNSUPPORTED_CHECKOUT", reason));
    }

    let policy = effective_policy(&config, &effective);
    let policy_digest = policy_digest(&policy);
    let context_id = build_context_id(&git, &tree, &policy, &config, root);
    let log_dir = state::logs_dir(root).join(&tree).join(&run_id);
    let mut outcomes = Vec::new();
    let mut logs = Vec::new();
    let mut log_digests = Vec::new();
    let mut failed = false;
    let mut timed_out = false;
    let mut unsupported = false;
    for ob in &selected {
        let outcome = gitcli::run_obligation(ob, scratch.path());
        let stdout_clean = gitcli::redact(&outcome.stdout);
        let stderr_clean = gitcli::redact(&outcome.stderr);
        let stdout_rel = format!(".flow/logs/{tree}/{run_id}/{}.stdout.log", outcome.id);
        let stderr_rel = format!(".flow/logs/{tree}/{run_id}/{}.stderr.log", outcome.id);
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
        log_digests.push(Json::obj(vec![
            ("id", Json::str(outcome.id.clone())),
            (
                "stdout_sha256",
                Json::str(sha256::sha256_hex(stdout_clean.as_bytes())),
            ),
            (
                "stderr_sha256",
                Json::str(sha256::sha256_hex(stderr_clean.as_bytes())),
            ),
        ]));
        logs.push(stdout_rel);
        logs.push(stderr_rel);
        if ob.required {
            // A status outside this list is an unknown outcome: never a pass.
            match outcome.status {
                "PASSED" => {}
                "FAILED" => failed = true,
                "TIMEOUT" => timed_out = true,
                _ => unsupported = true,
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
            // A truncated log is incomplete evidence: say so in the receipt
            // rather than presenting a clipped stream as the whole output.
            ("stdout_truncated", Json::Bool(outcome.stdout_truncated)),
            ("stderr_truncated", Json::Bool(outcome.stderr_truncated)),
            ("stdout_preview", Json::str(preview(&stdout_clean, 400))),
            ("stderr_preview", Json::str(preview(&stderr_clean, 400))),
        ]));
    }

    // The pre-check proved the checkout before the first obligation; this proves a
    // declared file was not changed while they ran (a `post-checkout` hook can
    // spawn a delayed writer, and an obligation can rewrite its own input). Extra
    // files are allowed here: obligations legitimately write build outputs.
    if let Err(reason) = verify_checkout_unchanged(&git, scratch.path(), &tree) {
        return Ok(refused_code("CHECKOUT_MUTATED_DURING_RUN", reason));
    }

    let worktree_diff = git
        .capture(&["diff", "--name-only", &tree])
        // An empty list on a failed diff would be a receipt claiming "no worktree
        // differences" the check never observed, so the run refuses to certify.
        .map_err(|e| {
            format!("cannot record the worktree diff against {tree} (a receipt with an empty diff would be a false claim): {e}")
        })?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    // The scratch directory is removed by `Scratch::drop`, which runs on every
    // path out of this function, including the `?` above.
    drop(scratch);

    let declared_ids: Vec<String> = effective.iter().map(|o| o.id.clone()).collect();
    let required_ids: Vec<String> = effective
        .iter()
        .filter(|o| o.required)
        .map(|o| o.id.clone())
        .collect();
    let selected_ids: Vec<String> = selected.iter().map(|o| o.id.clone()).collect();
    let missing_required: Vec<String> = required_ids
        .iter()
        .filter(|id| !selected_ids.contains(id))
        .cloned()
        .collect();

    let status = if scope_mode == "selected" {
        "PARTIAL"
    } else if !missing_required.is_empty() {
        "INCOMPLETE"
    } else if failed {
        "FAILED"
    } else if timed_out {
        "TIMEOUT"
    } else if unsupported {
        "UNSUPPORTED"
    } else {
        "PASSED"
    };

    let scope = Json::obj(vec![
        ("mode", Json::str(scope_mode)),
        (
            "declared",
            Json::Arr(declared_ids.iter().map(|s| Json::str(s.clone())).collect()),
        ),
        (
            "required",
            Json::Arr(required_ids.iter().map(|s| Json::str(s.clone())).collect()),
        ),
        (
            "selected",
            Json::Arr(selected_ids.iter().map(|s| Json::str(s.clone())).collect()),
        ),
        (
            "executed",
            Json::Arr(selected_ids.iter().map(|s| Json::str(s.clone())).collect()),
        ),
        (
            "missing_required",
            Json::Arr(
                missing_required
                    .iter()
                    .map(|s| Json::str(s.clone()))
                    .collect(),
            ),
        ),
    ]);

    let receipt = state::seal_receipt(&Json::obj(vec![
        ("schema", Json::str(state::SCHEMA_RECEIPT)),
        ("tree_oid", Json::str(tree.clone())),
        ("commit_oid", Json::str(commit)),
        ("parent_oid", Json::str(parent)),
        ("context_id", Json::str(context_id)),
        ("policy_digest", Json::str(policy_digest)),
        ("policy", policy),
        ("policy_version", Json::str(config.policy_version.clone())),
        (
            "execution_boundary",
            Json::str(config.execution_boundary.clone()),
        ),
        ("status", Json::str(status)),
        (
            "candidate_id",
            match &candidate_record {
                Some(record) => Json::str(record.candidate_id.clone()),
                None => Json::Null,
            },
        ),
        ("verification_scope", scope),
        ("started_at", Json::str(started_at)),
        ("finished_at", Json::str(state::iso8601_now())),
        ("run_id", Json::str(run_id.clone())),
        ("log_digests", Json::Arr(log_digests)),
        ("obligations", Json::Arr(outcomes)),
        (
            "logs",
            Json::Arr(logs.iter().map(|l| Json::str(l.clone())).collect()),
        ),
        (
            "worktree_diff",
            Json::Arr(worktree_diff.iter().map(|p| Json::str(p.clone())).collect()),
        ),
        // Evidence of the post-run proof: the declared files were still the tree's
        // when the last obligation finished (a run whose checkout changed is
        // refused before a receipt is written, so this field is always OK here).
        ("checkout_recheck", Json::str("OK")),
        (
            "scope_note",
            Json::str(if scope_mode == "selected" {
                "diagnostic run: only the selected obligations were executed, so this receipt proves nothing about the rest of the policy and is never publishable"
            } else {
                "verified for the declared obligations only; undeclared inputs are not covered"
            }),
        ),
    ]));
    let path = state::save_receipt(root, &receipt)?;
    let relative = path
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned());
    let mut out = receipt.clone();
    if let Json::Obj(pairs) = &mut out {
        pairs.push(("receipt_path".into(), Json::str(relative)));
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

// ---------------------------------------------------------------- publish
pub fn cmd_publish(
    root: &Path,
    reference: Option<String>,
    expected_old: Option<String>,
    message: Option<String>,
    candidate: Option<String>,
    dry_run: bool,
) -> Result<Json, String> {
    let config = state::load_config(root)?;
    if config.disabled {
        return Ok(refused_code(
            "DISABLED",
            "Flow is disabled in this repository",
        ));
    }
    if let Err(reason) = validate_obligations(&config.obligations) {
        return Ok(refused_code("INVALID_POLICY", reason));
    }
    if let Some(refusal) = adoption_gate(root, &config)? {
        return Ok(refusal);
    }
    let git = Git::new(root);
    let candidate_record = match candidate.as_deref() {
        None => None,
        Some(id) => match crate::proposal::Candidate::load(root, id)? {
            Some(record) => Some(record),
            None => {
                return Ok(refused_code(
                    "UNKNOWN_CANDIDATE",
                    format!("no candidate {id:?} in .flow/candidates/"),
                ))
            }
        },
    };

    // Publish to a branch only. `HEAD`, a tag or a remote ref is either
    // ambiguous or the wrong object to move for "this tree is verified".
    let reference = match reference {
        Some(r) => {
            if !r.starts_with("refs/heads/") {
                return Ok(refused_code(
                    "REF_NOT_ALLOWED",
                    format!("--ref {r:?} must be a full branch ref under refs/heads/"),
                ));
            }
            r
        }
        None => current_ref(&git)?,
    };
    if reference.len() > 1024 || reference.contains('\0') || reference.contains(' ') {
        return Ok(refused_code(
            "REF_NOT_ALLOWED",
            format!("--ref {reference:?} is not a usable ref name"),
        ));
    }
    if git.run(&["check-ref-format", &reference])?.status != 0 {
        return Ok(refused_code(
            "REF_NOT_ALLOWED",
            format!("--ref {reference:?} is not a valid ref name (git check-ref-format)"),
        ));
    }
    // A symbolic ref would move its target while this command reports the name
    // the operator passed: they would believe the commit landed on `alias` when
    // it landed on something else. Refuse rather than report a misleading name.
    let symref = git.run(&["symbolic-ref", "-q", &reference])?;
    if symref.status == 0 {
        return Ok(refused_code(
            "REF_IS_SYMREF",
            format!(
                "--ref {reference:?} is a symbolic ref to {:?}: publish to the real branch ref",
                symref.stdout.trim()
            ),
        ));
    }

    let _lock = match crate::lock::acquire_lock(root, "publish") {
        Ok(guard) => guard,
        Err(e) => return Ok(lock_refusal(root, e)),
    };

    // Publish exactly the tree that `run` verified: the candidate's tree when
    // `--candidate` is given, the index tree otherwise.
    let tree = match &candidate_record {
        Some(record) => record.tree_oid.clone(),
        None => index_tree_without_locking(&git)?,
    };
    if candidate_record.is_some() {
        let present =
            git.run_bounded(&["cat-file", "-e", &format!("{tree}^{{tree}}")], None, None)?;
        if present.status != 0 {
            return Ok(refused_code(
                "CANDIDATE_TREE_MISSING",
                format!("candidate tree {tree} is no longer in the object store: nothing coherent can be published"),
            ));
        }
    }
    let receipt = match Receipt::load(root, &tree)? {
        Some(r) => r,
        None => {
            return Ok(refused_code(
                "NO_RECEIPT",
                format!("no complete receipt for tree {tree}: run `flow run` on this exact index tree first (a selective run never produces one)"),
            ))
        }
    };

    // 1. the receipt must be written by this version, unaltered.
    if receipt.schema != state::SCHEMA_RECEIPT {
        return Ok(refused_code(
            "RECEIPT_SCHEMA_STALE",
            format!(
                "receipt schema {:?} is not {}: re-run `flow run`",
                receipt.schema,
                state::SCHEMA_RECEIPT
            ),
        ));
    }
    if let Err(e) = state::verify_receipt_integrity(&receipt.json) {
        return Ok(refused_code("RECEIPT_INTEGRITY", e));
    }
    // 2. it must name this tree, and cover the whole declared policy.
    if receipt.tree_oid != tree {
        return Ok(refused_code(
            "TREE_MISMATCH",
            format!(
                "receipt integrity error: file for {tree} names tree {:?}",
                receipt.tree_oid
            ),
        ));
    }
    if receipt.scope_mode != "complete" {
        return Ok(refused_code(
            "SCOPE_INCOMPLETE",
            format!(
                "receipt scope is {:?}: a selective run is diagnostic and never publishable",
                receipt.scope_mode
            ),
        ));
    }
    if !receipt.missing_required.is_empty() {
        return Ok(refused_code(
            "MISSING_REQUIRED",
            format!(
                "receipt does not cover required obligations {:?}",
                receipt.missing_required
            ),
        ));
    }
    // 3. the policy must still be the policy the receipt proves. This is
    //    recomputed from the current configuration, never read from the receipt.
    let policy = effective_policy(&config, &config.obligations);
    let expected_digest = policy_digest(&policy);
    if receipt.policy_digest != expected_digest {
        return Ok(refused_code(
            "STALE_POLICY",
            format!(
                "the receipt proves policy {} but the current policy is {}: any edit to .flow/config.json, and any `--timeout` override at run time, changes the proven policy - re-run `flow run` without overrides",
                short(&receipt.policy_digest),
                short(&expected_digest)
            ),
        ));
    }
    if receipt.policy_version != config.policy_version {
        return Ok(refused_code(
            "POLICY_VERSION_MISMATCH",
            format!(
                "receipt policy_version {:?} != current {:?}",
                receipt.policy_version, config.policy_version
            ),
        ));
    }
    if receipt.execution_boundary != config.execution_boundary {
        return Ok(refused_code(
            "EXECUTION_BOUNDARY_MISMATCH",
            format!(
                "receipt execution_boundary {:?} != current {:?}",
                receipt.execution_boundary, config.execution_boundary
            ),
        ));
    }
    // The context covers the toolchain and platform the run actually used; a Git
    // upgrade between run and publish changes it, and the receipt is then no
    // longer a statement about the tool that would publish.
    let expected_context = build_context_id(&git, &tree, &policy, &config, root);
    if receipt.context_id != expected_context {
        return Ok(refused_code(
            "CONTEXT_MISMATCH",
            format!(
                "the receipt context is {} but this environment now computes {}: an allowlisted environment value, an obligation program's bytes, a declared probe's answer, or the platform changed since the run",
                short(&receipt.context_id),
                short(&expected_context)
            ),
        ));
    }
    // 4. every currently required obligation must be present *and* passed. The
    //    global status alone is not enough: a missing entry is not a pass.
    for ob in config.obligations.iter().filter(|o| o.required) {
        match receipt
            .obligations
            .iter()
            .find(|x| x.get("id").and_then(Json::as_str) == Some(ob.id.as_str()))
        {
            None => {
                return Ok(refused_code(
                    "MISSING_REQUIRED",
                    format!("required obligation {:?} is absent from the receipt", ob.id),
                ))
            }
            Some(entry) if entry.get("status").and_then(Json::as_str) != Some("PASSED") => {
                return Ok(refused_code(
                    "REQUIRED_NOT_PASSED",
                    format!(
                        "required obligation {:?} has status {:?}",
                        ob.id,
                        entry
                            .get("status")
                            .and_then(Json::as_str)
                            .unwrap_or("UNKNOWN")
                    ),
                ))
            }
            Some(_) => {}
        }
    }
    if receipt.status != "PASSED" {
        return Ok(refused_code(
            "STATUS_NOT_PASSED",
            format!(
                "receipt status is {}: a non-PASSED receipt is never publishable",
                receipt.status
            ),
        ));
    }
    // 5. attribution: the receipt proves an exact tree, not necessarily the
    //    candidate the operator names. When `--candidate` is given, the record
    //    must name that candidate; otherwise the publication would report C2
    //    while its evidence belongs to C1. Without `--candidate`, the receipt's
    //    candidate is exposed in the output and journal instead of guessed.
    let receipt_candidate_id = receipt
        .json
        .get("candidate_id")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(record) = &candidate_record {
        if receipt_candidate_id != record.candidate_id {
            return Ok(refused_code(
                "RECEIPT_CANDIDATE_MISMATCH",
                format!(
                    "the receipt for tree {tree} was written by candidate {:?}, but this publication names candidate {}: run `flow run --candidate {}` first, or publish without --candidate and read receipt_candidate_id",
                    if receipt_candidate_id.is_empty() {
                        "<index tree, no candidate>"
                    } else {
                        &receipt_candidate_id
                    },
                    record.candidate_id,
                    record.candidate_id
                ),
            ));
        }
    }
    // 6. the verification commit must still describe the same tree, when the
    //    object still exists (it may have been pruned; absence is not a failure).
    if !receipt.commit_oid.is_empty() {
        if let Ok(t) = git.capture(&["rev-parse", &format!("{}^{{tree}}", receipt.commit_oid)]) {
            if t != tree {
                return Ok(refused_code(
                    "RECEIPT_COMMIT_MISMATCH",
                    format!(
                        "receipt commit {} carries tree {t}, not the verified {tree}",
                        receipt.commit_oid
                    ),
                ));
            }
        }
    }

    // The evidence the receipt names must still be there and unchanged. The seal
    // covers the names and hashes, not the bytes on disk: a deleted or swapped log
    // is not evidence, and publication must be at least as strict as `explain`.
    let (missing_logs, tampered_logs, logs_ok) = verify_receipt_logs(root, &receipt);
    if !missing_logs.is_empty() {
        return Ok(refused_code(
            "MISSING_LOGS",
            format!(
                "the receipt for tree {tree} names {} log(s) that no longer exist (for example {:?}): the evidence is incomplete, re-run `flow run`",
                missing_logs.len(),
                &missing_logs[..missing_logs.len().min(3)]
            ),
        ));
    }
    if !logs_ok {
        return Ok(refused_code(
            "LOG_INTEGRITY_INVALID",
            format!(
                "the receipt for tree {tree} names log(s) whose bytes no longer hash to the recorded digest (for example {:?}): refusing to publish swapped evidence",
                &tampered_logs[..tampered_logs.len().min(3)]
            ),
        ));
    }

    let old = match expected_old {
        Some(o) => o,
        None => match git.show_ref(&reference)? {
            Some(o) => o,
            None => {
                return Ok(refused_code(
                    "UNKNOWN_REF",
                    format!("cannot resolve {reference}"),
                ))
            }
        },
    };
    let current = git.show_ref(&reference)?;
    if current.as_deref() != Some(old.as_str()) {
        return Ok(refused_code(
            "STALE_REF",
            format!("stale expected old OID: ref is {current:?}, expected {old}"),
        ));
    }
    // Publishing a tree the ref already carries would only add an empty commit.
    if let Ok(current_tree) = git.capture(&["rev-parse", &format!("{old}^{{tree}}")]) {
        if current_tree == tree {
            return Ok(refused_code(
                "ALREADY_PUBLISHED",
                format!("{reference} already points at a commit whose tree is {tree}: nothing to publish"),
            ));
        }
    }

    // A candidate is a statement about "base + these proposals". Publishing it on
    // top of a different commit would silently claim a merge Flow never made.
    if let Some(record) = &candidate_record {
        if record.base_commit != old {
            return Ok(refused_code(
                "BASE_MISMATCH",
                format!(
                    "candidate {} was assembled on base {} but this publication would land on {}: re-assemble on the new base and re-run, or pass the matching --expected-old",
                    record.candidate_id, record.base_commit, old
                ),
            ));
        }
    }
    if dry_run {
        // Everything above is the same preflight a real publication performs.
        // Nothing below this point has happened yet: no commit object, no ref
        // movement, no journal line.
        return Ok(Json::obj(vec![
            ("status", Json::str("PLAN")),
            ("schema", Json::str(state::SCHEMA_PLAN)),
            ("dry_run", Json::Bool(true)),
            ("ref", Json::str(reference)),
            ("old", Json::str(old)),
            ("tree", Json::str(tree.clone())),
            (
                "candidate_id",
                match &candidate_record {
                    Some(record) => Json::str(record.candidate_id.clone()),
                    None => Json::Null,
                },
            ),
            ("receipt_candidate_id", Json::str(receipt_candidate_id.clone())),
            ("context_id", Json::str(receipt.context_id.clone())),
            ("policy_digest", Json::str(receipt.policy_digest.clone())),
            ("policy_version", Json::str(receipt.policy_version.clone())),
            (
                "execution_boundary",
                Json::str(receipt.execution_boundary.clone()),
            ),
            (
                "obligations",
                Json::Arr(
                    receipt
                        .obligations
                        .iter()
                        .filter_map(|o| o.get("id").and_then(Json::as_str).map(Json::str))
                        .collect(),
                ),
            ),
            (
                "changes",
                match &candidate_record {
                    Some(record) => {
                        Json::Arr(record.changes.iter().map(|c| c.to_json()).collect())
                    }
                    None => Json::Arr(Vec::new()),
                },
            ),
            (
                "journal",
                Json::str(".flow/journal.jsonl".to_string()),
            ),
            (
                "note",
                Json::str("plan only: no commit was created, no ref was moved, no journal record was appended"),
            ),
        ]));
    }
    let message = message.unwrap_or_else(|| format!("flow: publish verified tree {tree}"));
    let commit = git.commit_tree(&tree, &old, &message)?;
    // The commit we are about to publish must carry exactly the qualified tree.
    let prepared_tree = git.capture(&["rev-parse", &format!("{commit}^{{tree}}")])?;
    if prepared_tree != tree {
        return Ok(refused_code(
            "COMMIT_TREE_MISMATCH",
            format!(
                "prepared commit {commit} carries tree {prepared_tree}, not the verified {tree}"
            ),
        ));
    }
    let id = format!(
        "{}-{}-{}",
        state::iso8601_now(),
        &commit[..12.min(commit.len())],
        state::unique_suffix()
    );
    if crash_point("before-intent") {
        std::process::exit(89);
    }
    state::append_journal(
        root,
        &Json::obj(vec![
            ("event", Json::str("intent")),
            ("id", Json::str(id.clone())),
            ("ref", Json::str(reference.clone())),
            ("old", Json::str(old.clone())),
            ("new", Json::str(commit.clone())),
            ("tree", Json::str(tree.clone())),
            (
                "candidate_id",
                if receipt_candidate_id.is_empty() {
                    Json::Null
                } else {
                    Json::str(receipt_candidate_id.clone())
                },
            ),
            ("at", Json::str(state::iso8601_now())),
        ]),
    )?;
    if crash_point("after-intent") {
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
        return Ok(refused_code(
            "REF_MOVED",
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
        return Ok(refused_code(
            "UPDATE_REF_REFUSED",
            format!("git update-ref refused: {}", res.stderr.trim()),
        ));
    }
    if crash_point("after-ref") {
        std::process::exit(89);
    }
    // Post-CAS observation: the ref must now carry exactly the prepared commit.
    // Anything else is an outcome Flow did not plan, so it is reported as needing
    // reconciliation rather than as a success.
    let observed = git.show_ref(&reference)?;
    if observed.as_deref() != Some(commit.as_str()) {
        state::append_journal(
            root,
            &Json::obj(vec![
                ("event", Json::str("outcome")),
                ("id", Json::str(id)),
                ("status", Json::str(state::STATE_NEEDS_RECONCILIATION)),
                ("observed", Json::str(observed.unwrap_or_default())),
                ("at", Json::str(state::iso8601_now())),
            ]),
        )?;
        return Ok(refused_code(
            "NEEDS_RECONCILIATION",
            format!("update-ref reported success but {reference} does not carry {commit}: inspect with `flow reconcile`"),
        ));
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
        (
            "candidate_id",
            match &candidate_record {
                Some(record) => Json::str(record.candidate_id.clone()),
                None => Json::Null,
            },
        ),
        ("ref", Json::str(reference)),
        ("old", Json::str(old)),
        ("commit", Json::str(commit)),
        ("tree", Json::str(tree)),
        ("context_id", Json::str(receipt.context_id.clone())),
        ("policy_digest", Json::str(receipt.policy_digest.clone())),
        ("policy_version", Json::str(receipt.policy_version.clone())),
        (
            "execution_boundary",
            Json::str(receipt.execution_boundary.clone()),
        ),
        ("receipt_logs", Json::Num(receipt.logs.len() as f64)),
        ("receipt_run_id", Json::str(receipt.run_id.clone())),
        (
            "receipt_candidate_id",
            Json::str(receipt_candidate_id.clone()),
        ),
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
/// Count journal records by `id` for one event kind.
/// A lock refusal carries its own code: a held lock and an unattributable one
/// have different remedies, and the second must never look like "try again".
fn lock_refusal(_root: &Path, err: crate::lock::LockError) -> Json {
    match err {
        crate::lock::LockError::Held(why) => refused_code("LOCKED", why),
        crate::lock::LockError::Unknown(why) => {
            let mut value = refused_code("LOCK_UNKNOWN", why);
            if let Json::Obj(pairs) = &mut value {
                pairs.push(("manual_required".into(), Json::Bool(true)));
                pairs.push(("lock_path".into(), Json::str(".flow/lock".to_string())));
            }
            value
        }
        crate::lock::LockError::Io(why) => refused_code("LOCK_IO", why),
    }
}

/// Crash-injection point, **debug builds only**.
///
/// The test suite needs to interrupt a publication between its durable steps.
/// A released binary must not honour an environment variable that can skip the
/// outcome record, so the variable is ignored outside `debug_assertions` — and a
/// CI step proves it on the release binary.
fn crash_point(name: &str) -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    std::env::var("FLOW_TEST_CRASH").as_deref() == Ok(name)
}

fn count_by_id(records: &[Json], event: &str) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for record in records {
        if record.get("event").and_then(Json::as_str) == Some(event) {
            let id = record
                .get("id")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            *counts.entry(id).or_insert(0) += 1;
        }
    }
    counts
}

/// Intents that no outcome can be attributed to.
///
/// Counting, not set membership: an intent is closed only when the journal holds
/// at least as many outcomes as intents for that id. One outcome must never
/// close two intents.
fn pending_intents(root: &Path) -> Result<Vec<Json>, String> {
    let records = state::read_journal(root)?;
    let outcomes = count_by_id(&records, "outcome");
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut pending = Vec::new();
    for record in records {
        if record.get("event").and_then(Json::as_str) != Some("intent") {
            continue;
        }
        let id = record
            .get("id")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let position = {
            let slot = seen.entry(id.clone()).or_insert(0);
            let position = *slot;
            *slot += 1;
            position
        };
        if position >= outcomes.get(&id).copied().unwrap_or(0) {
            pending.push(record);
        }
    }
    Ok(pending)
}

/// The explicit state of "we could not even read the journal".
fn journal_failure(_root: &Path, err: state::JournalError) -> Json {
    let (status, code) = match &err {
        state::JournalError::Io(_) => (state::STATE_UNKNOWN_OUTCOME, "JOURNAL_UNREADABLE"),
        state::JournalError::Corrupt(_) => (state::STATE_CORRUPT_JOURNAL, "JOURNAL_CORRUPT"),
    };
    Json::obj(vec![
        ("status", Json::str(status)),
        ("code", Json::str(code)),
        ("error", Json::str(err.to_string())),
        ("journal_path", Json::str(".flow/journal.jsonl".to_string())),
        ("journal_preserved", Json::Bool(true)),
        ("manual_required", Json::Bool(true)),
        ("resolved", Json::Arr(Vec::new())),
        (
            "note",
            Json::str(
                "recovery refuses to guess: the journal was not rewritten and no ref was moved",
            ),
        ),
    ])
}

pub fn cmd_reconcile(root: &Path) -> Result<Json, String> {
    let git = Git::new(root);
    let _lock = match crate::lock::acquire_lock(root, "reconcile") {
        Ok(guard) => guard,
        Err(e) => return Ok(lock_refusal(root, e)),
    };
    let records = match state::read_journal(root) {
        Ok(records) => records,
        Err(e) => return Ok(journal_failure(root, e)),
    };

    let occurrences = count_by_id(&records, "intent");
    let outcome_counts = count_by_id(&records, "outcome");

    let mut resolved: Vec<Json> = Vec::new();
    let mut needs_human = false;
    for intent in &records {
        if intent.get("event").and_then(Json::as_str) != Some("intent") {
            continue;
        }
        let id = intent
            .get("id")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let intents_for_id = occurrences.get(&id).copied().unwrap_or(0);
        let outcomes_for_id = outcome_counts.get(&id).copied().unwrap_or(0);
        // A shared id cannot be attributed to one intent. Checking `done` first
        // would let one duplicate's outcome silently close the other, which is
        // exactly how an unresolved publication can be reported as reconciled.
        if intents_for_id > 1 {
            if intents_for_id > outcomes_for_id {
                needs_human = true;
                resolved.push(Json::obj(vec![
                    ("id", Json::str(id)),
                    ("resolution", Json::str(state::STATE_NEEDS_RECONCILIATION)),
                    ("reason", Json::str("DUPLICATE_INTENT_ID")),
                ]));
            }
            continue;
        }
        if outcomes_for_id > 0 {
            continue;
        }
        let field = |k: &str| {
            intent
                .get(k)
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string()
        };
        let (reference, old, new) = (field("ref"), field("old"), field("new"));
        if reference.is_empty() || old.is_empty() || new.is_empty() {
            needs_human = true;
            resolved.push(Json::obj(vec![
                ("id", Json::str(id)),
                ("resolution", Json::str(state::STATE_NEEDS_RECONCILIATION)),
                ("reason", Json::str("INCOMPLETE_INTENT")),
            ]));
            continue;
        }
        let observed = match git.show_ref(&reference) {
            Ok(Some(oid)) => oid,
            Ok(None) => {
                needs_human = true;
                resolved.push(Json::obj(vec![
                    ("id", Json::str(id)),
                    ("ref", Json::str(reference)),
                    ("resolution", Json::str(state::STATE_NEEDS_RECONCILIATION)),
                    ("reason", Json::str("REF_ABSENT")),
                ]));
                continue;
            }
            Err(e) => {
                needs_human = true;
                resolved.push(Json::obj(vec![
                    ("id", Json::str(id)),
                    ("ref", Json::str(reference)),
                    ("resolution", Json::str(state::STATE_NEEDS_RECONCILIATION)),
                    ("reason", Json::str(format!("REF_UNREADABLE: {e}"))),
                ]));
                continue;
            }
        };
        let resolution = if observed == new {
            "PUBLISHED"
        } else if observed == old {
            "NOT_PUBLISHED"
        } else {
            state::STATE_NEEDS_RECONCILIATION
        };
        // Only a decisive observation is recorded. An ambiguous one stays open
        // so it is re-examined instead of being closed by a guess.
        if resolution != state::STATE_NEEDS_RECONCILIATION {
            state::append_journal(
                root,
                &Json::obj(vec![
                    ("event", Json::str("outcome")),
                    ("id", Json::str(id.clone())),
                    ("status", Json::str(format!("RECONCILED_{resolution}"))),
                    ("observed", Json::str(observed.clone())),
                    ("at", Json::str(state::iso8601_now())),
                ]),
            )?;
        } else {
            needs_human = true;
        }
        resolved.push(Json::obj(vec![
            ("id", Json::str(id)),
            ("ref", Json::str(reference)),
            ("observed", Json::str(observed)),
            ("resolution", Json::str(resolution)),
        ]));
    }

    let status = if needs_human {
        state::STATE_NEEDS_RECONCILIATION
    } else {
        "RECONCILED"
    };
    Ok(Json::obj(vec![
        ("status", Json::str(status)),
        ("resolved", Json::Arr(resolved)),
        ("manual_required", Json::Bool(needs_human)),
        (
            "note",
            Json::str("reconciliation never moves a ref: it only records what is observable"),
        ),
    ]))
}

// ---------------------------------------------------------------------- gc

/// Modification time of an entry: receipts and logs are written once, at the end
/// of the run that produced them. Kept as a `SystemTime` so "newest first" keeps
/// sub-second precision (a flat seconds key would tie every run of the same
/// second and make "the newest N" arbitrary).
fn entry_mtime(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// A plain file or a real directory. A link is never a removal target and a
/// removal never traverses one, for the same reason every `.flow/` write refuses
/// links: the repository may have arrived with one.
fn is_plain_entry(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_file() || m.file_type().is_dir())
        .unwrap_or(false)
}

fn directory_bytes(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            total += directory_bytes(&entry.path());
        } else {
            total += meta.len();
        }
    }
    total
}

/// Apply an explicit retention policy to Flow's own evidence.
///
/// `.flow/` grows with every verification and an audit found no policy at all: a
/// long-lived workspace accumulates receipts, logs and candidates without bound.
/// `gc` is opt-in by construction — it requires at least one of `--keep` and
/// `--max-age-days`, so no default ever deletes evidence — and it never touches
/// the journal, the config, the adoption record or the proposals.
///
/// It refuses while a publication intent is unresolved: the journal and the
/// receipts it names are the recovery record, not garbage.
pub fn cmd_gc(
    root: &Path,
    keep: Option<usize>,
    max_age_days: Option<u64>,
    dry_run: bool,
) -> Result<Json, String> {
    if keep.is_none() && max_age_days.is_none() {
        return Ok(crate::proposal::refusal(
            "USAGE",
            "flow gc needs an explicit retention policy: --keep <N> and/or --max-age-days <D> (add --dry-run to plan it)",
        ));
    }
    state::load_config(root)?;
    // `gc` takes the same lock as `publish`/`reconcile`, so it can never collect
    // the journal, a receipt or a log while a publication transaction is in
    // flight. A verification (`run`) is long-lived and does not hold the lock:
    // it advertises itself with a marker instead.
    let _lock = match crate::lock::acquire_lock(root, "gc") {
        Ok(guard) => guard,
        Err(e) => return Ok(lock_refusal(root, e)),
    };
    let runs = state::runs_dir(root);
    if let Ok(entries) = fs::read_dir(&runs) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".active") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(value) = json::parse(&text) else {
                continue;
            };
            let pid = value.get("pid").and_then(Json::as_num).unwrap_or(0.0) as u32;
            let run_id = value
                .get("run_id")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            match crate::lock::pid_alive(pid) {
                Some(true) => {
                    return Ok(crate::proposal::refusal(
                        "RUN_ACTIVE",
                        format!(
                            "flow gc refuses while run {run_id} (pid {pid}) is writing evidence: it would delete logs or a candidate that the run still needs. Re-run gc after the run finishes"
                        ),
                    ));
                }
                Some(false) => {
                    // The writer is gone: the marker is stale and safe to clear.
                    let _ = fs::remove_file(&path);
                }
                None => {
                    return Ok(crate::proposal::refusal(
                        "RUN_ACTIVE",
                        format!(
                            "flow gc cannot prove that run {run_id} (pid {pid}) is finished, so it refuses to collect evidence. Inspect {} and remove the marker by hand only if no flow process is running",
                            path.display()
                        ),
                    ));
                }
            }
        }
    }
    let pending = pending_intents(root)?;
    if !pending.is_empty() {
        return Ok(crate::proposal::refusal(
            "PENDING_INTENTS",
            format!(
                "flow gc refuses while {} publication intent(s) are unresolved: run `flow reconcile` first. The journal, the receipts and the logs it names are the recovery record",
                pending.len()
            ),
        ));
    }

    let max_age = max_age_days.map(|days| days.saturating_mul(86_400));
    let now = std::time::SystemTime::now();
    // With no age policy, `--keep` decides alone; with no keep policy, age does.
    let expired = |path: &Path| -> bool {
        let Some(limit) = max_age else {
            return true;
        };
        match entry_mtime(path) {
            // A future mtime is not old evidence: keep it.
            Some(mtime) => now
                .duration_since(mtime)
                .map(|age| age.as_secs() >= limit)
                .unwrap_or(false),
            None => true,
        }
    };
    // Partial receipts and candidate records are not publishable evidence: only
    // an explicit age policy may collect them. `--keep` alone must never delete a
    // candidate the operator assembled and has not run yet (found by the
    // adversarial pre-review: `gc --keep N` removed every candidate).
    let expired_by_age = |path: &Path| -> bool {
        let Some(limit) = max_age else {
            return false;
        };
        match entry_mtime(path) {
            Some(mtime) => now
                .duration_since(mtime)
                .map(|age| age.as_secs() >= limit)
                .unwrap_or(false),
            None => true,
        }
    };

    let mut deleted: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut bytes_freed = 0u64;
    let mut kept_receipts: BTreeSet<String> = BTreeSet::new();
    let mut kept_partial_trees: BTreeSet<String> = BTreeSet::new();
    let mut kept_partials = 0usize;
    let mut kept_candidates = 0usize;

    // Receipts. The `keep` newest complete receipts are the evidence the operator
    // may still publish, so they survive regardless of age; the rest follow the
    // age policy. A partial receipt is never publishable, so it is age-collected
    // only and never displaces a complete one.
    let receipts = state::receipts_dir(root);
    let mut complete: Vec<PathBuf> = Vec::new();
    let mut partial: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&receipts) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            if !is_plain_entry(&path) {
                skipped.push(format!("receipts/{name} (not a plain file)"));
                continue;
            }
            let Some(stem) = name.strip_suffix(".json") else {
                skipped.push(format!("receipts/{name} (not a receipt)"));
                continue;
            };
            if stem.contains(".partial.") {
                partial.push(path);
            } else if crate::paths::is_object_id(stem) {
                complete.push(path);
            } else {
                skipped.push(format!("receipts/{name} (not an object id)"));
            }
        }
    }
    complete.sort_by_key(|path| std::cmp::Reverse(entry_mtime(path)));
    let keep_count = keep.unwrap_or(0);
    for (index, path) in complete.iter().enumerate() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let stem = name.strip_suffix(".json").unwrap_or(&name).to_string();
        if index < keep_count || !expired(path) {
            kept_receipts.insert(stem);
            continue;
        }
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if !dry_run {
            fs::remove_file(path).map_err(|e| format!("cannot remove receipts/{name}: {e}"))?;
        }
        bytes_freed += size;
        deleted.push(format!("receipts/{name}"));
    }
    for path in partial {
        if !expired_by_age(&path) {
            kept_partials += 1;
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(tree) = name.split(".partial.").next() {
                    kept_partial_trees.insert(tree.to_string());
                }
            }
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if !dry_run {
            fs::remove_file(&path).map_err(|e| format!("cannot remove receipts/{name}: {e}"))?;
        }
        bytes_freed += size;
        deleted.push(format!("receipts/{name}"));
    }

    // Logs: one directory per tree. The logs of a kept receipt are kept with it,
    // because they are the evidence that receipt names.
    let logs = state::logs_dir(root);
    if let Ok(entries) = fs::read_dir(&logs) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            if kept_receipts.contains(&name)
                || kept_partial_trees.contains(&name)
                || !expired(&path)
            {
                continue;
            }
            if !is_plain_entry(&path) {
                skipped.push(format!("logs/{name} (not a plain entry)"));
                continue;
            }
            let size = directory_bytes(&path);
            if !dry_run {
                // `is_plain_entry` admits a plain file as well as a directory: a
                // stray file under `logs/` is still named by the policy, and it
                // must not turn `gc` into a failure (Windows refuses
                // `remove_dir_all` on a file with ERROR_DIRECTORY).
                let removed = if path.is_dir() {
                    fs::remove_dir_all(&path)
                } else {
                    fs::remove_file(&path)
                };
                removed.map_err(|e| format!("cannot remove logs/{name}: {e}"))?;
            }
            bytes_freed += size;
            deleted.push(format!("logs/{name}/"));
        }
    }

    // Candidates: an unanswered candidate record is not evidence of anything, so
    // it follows the same policy. The proposals it names are left alone.
    let candidates = state::candidates_dir(root);
    if let Ok(entries) = fs::read_dir(&candidates) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let Some(stem) = name.strip_suffix(".json") else {
                continue;
            };
            if !crate::paths::is_record_id(stem) || !is_plain_entry(&path) {
                skipped.push(format!("candidates/{name} (not a record)"));
                continue;
            }
            if !expired_by_age(&path) {
                kept_candidates += 1;
                continue;
            }
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if !dry_run {
                fs::remove_file(&path)
                    .map_err(|e| format!("cannot remove candidates/{name}: {e}"))?;
            }
            bytes_freed += size;
            deleted.push(format!("candidates/{name}"));
        }
    }

    Ok(Json::obj(vec![
        ("status", Json::str("GC")),
        ("schema", Json::str("flow-gc/1")),
        ("dry_run", Json::Bool(dry_run)),
        (
            "policy",
            Json::obj(vec![
                (
                    "keep",
                    match keep {
                        Some(n) => Json::Num(n as f64),
                        None => Json::Null,
                    },
                ),
                (
                    "max_age_days",
                    match max_age_days {
                        Some(n) => Json::Num(n as f64),
                        None => Json::Null,
                    },
                ),
            ]),
        ),
        (
            "kept",
            Json::obj(vec![
                ("receipts", Json::Num(kept_receipts.len() as f64)),
                ("partial_receipts", Json::Num(kept_partials as f64)),
                ("candidates", Json::Num(kept_candidates as f64)),
            ]),
        ),
        ("deleted_count", Json::Num(deleted.len() as f64)),
        ("bytes_freed", Json::Num(bytes_freed as f64)),
        (
            "deleted",
            Json::Arr(deleted.into_iter().map(Json::str).collect()),
        ),
        (
            "skipped",
            Json::Arr(skipped.into_iter().map(Json::str).collect()),
        ),
        (
            "never_deleted",
            Json::str(
                "journal.jsonl, config.json, adoption.json, proposals/, anything outside .flow/",
            ),
        ),
    ]))
}

// ----------------------------------------------------------------- explain
/// The receipt's own evidence check, shared by `explain` and `publish`.
///
/// A sealed receipt names the logs it hashed; the seal covers the *names and
/// hashes*, not the files on disk. Publication therefore checks the bytes as
/// well: a receipt whose log was deleted or swapped proves nothing, and the
/// publication gate must not be weaker than the diagnostic (found by the
/// adversarial pre-review).
fn verify_receipt_logs(root: &Path, receipt: &Receipt) -> (Vec<String>, Vec<String>, bool) {
    let missing: Vec<String> = receipt
        .logs
        .iter()
        .filter(|rel| !root.join(rel.as_str()).is_file())
        .cloned()
        .collect();
    let mut ok = true;
    let mut tampered: Vec<String> = Vec::new();
    for entry in &receipt.log_digests {
        let id = entry.get("id").and_then(Json::as_str).unwrap_or("");
        for (suffix, key) in [("stdout", "stdout_sha256"), ("stderr", "stderr_sha256")] {
            let Some(recorded) = entry.get(key).and_then(Json::as_str) else {
                ok = false;
                continue;
            };
            let wanted = format!("/{id}.{suffix}.log");
            let Some(rel) = receipt.logs.iter().find(|p| p.ends_with(&wanted)) else {
                ok = false;
                continue;
            };
            match std::fs::read(root.join(rel)).ok() {
                Some(bytes) if sha256::sha256_hex(&bytes) == recorded => {}
                _ => {
                    ok = false;
                    tampered.push(rel.clone());
                }
            }
        }
    }
    (missing, tampered, ok)
}

pub fn cmd_explain(
    root: &Path,
    tree: Option<String>,
    candidate: Option<String>,
) -> Result<Json, String> {
    let git = Git::new(root);
    let candidate_record = match candidate.as_deref() {
        None => None,
        Some(id) => match crate::proposal::Candidate::load(root, id)? {
            Some(record) => Some(record),
            None => {
                return Ok(refused_code(
                    "UNKNOWN_CANDIDATE",
                    format!("no candidate {id:?} in .flow/candidates/"),
                ))
            }
        },
    };
    let tree = match (&tree, &candidate_record) {
        (Some(t), _) => t.clone(),
        (None, Some(record)) => record.tree_oid.clone(),
        (None, None) => git.head_tree()?,
    };
    let receipt = Receipt::load(root, &tree)?;
    let records = state::read_journal(root)?;
    let related: Vec<Json> = records
        .iter()
        .filter(|r| r.get("tree").and_then(Json::as_str) == Some(tree.as_str()))
        .cloned()
        .collect();
    // Proposals are attributed to a candidate, so explain can name who proposed
    // what without the caller having to correlate ids by hand.
    let proposals = match &candidate_record {
        None => Json::Arr(Vec::new()),
        Some(record) => Json::Arr(
            record
                .proposals
                .iter()
                .filter_map(|id| crate::proposal::Proposal::load(root, id).ok().flatten())
                .map(|proposal| {
                    Json::obj(vec![
                        ("proposal_id", Json::str(proposal.proposal_id)),
                        ("agent_id", Json::str(proposal.agent_id)),
                        ("task_id", Json::str(proposal.task_id)),
                        ("base_commit", Json::str(proposal.base_commit)),
                        ("tree_oid", Json::str(proposal.tree_oid)),
                    ])
                })
                .collect(),
        ),
    };
    // A receipt is evidence only if its content still matches its digest and the
    // logs it names are present. Inspection must say so explicitly: displaying a
    // tampered receipt as `PASSED` is the false green Flow refuses at publication.
    let (
        receipt_json,
        receipt_trust,
        receipt_display,
        evidence_complete,
        missing_logs,
        log_integrity,
        tampered_logs,
    ) = match &receipt {
        None => (
            Json::Null,
            "NO_RECEIPT",
            Json::Null,
            false,
            Vec::<String>::new(),
            "NO_RECEIPT",
            Vec::<String>::new(),
        ),
        Some(r) => {
            let integrity_ok = state::verify_receipt_integrity(&r.json).is_ok();
            // A receipt that names log digests is only trustworthy if the files
            // still hash to them: swapped evidence is not evidence.
            let (missing, tampered, log_ok) = verify_receipt_logs(root, r);
            let log_integrity = if r.log_digests.is_empty() {
                "NO_LOG_DIGESTS"
            } else if log_ok {
                "OK"
            } else {
                "LOG_INTEGRITY_INVALID"
            };
            let display = if integrity_ok {
                Json::str(r.status.clone())
            } else {
                Json::str("RECEIPT_INTEGRITY_INVALID")
            };
            (
                r.json.clone(),
                if integrity_ok {
                    "INTEGRITY_OK"
                } else {
                    "RECEIPT_INTEGRITY_INVALID"
                },
                display,
                integrity_ok && missing.is_empty() && log_ok,
                missing,
                log_integrity,
                tampered,
            )
        }
    };
    Ok(Json::obj(vec![
        ("status", Json::str("EXPLAIN")),
        ("tree", Json::str(tree)),
        (
            "candidate_id",
            match &candidate_record {
                Some(record) => Json::str(record.candidate_id.clone()),
                None => Json::Null,
            },
        ),
        (
            "candidate",
            match &candidate_record {
                Some(record) => record.json.clone(),
                None => Json::Null,
            },
        ),
        ("proposals", proposals),
        ("receipt", receipt_json),
        ("receipt_trust", Json::str(receipt_trust)),
        (
            "run_id",
            match &receipt {
                Some(r) => Json::str(r.run_id.clone()),
                None => Json::Null,
            },
        ),
        ("receipt_status_display", receipt_display),
        ("evidence_complete", Json::Bool(evidence_complete)),
        ("log_integrity", Json::str(log_integrity)),
        (
            "tampered_logs",
            Json::Arr(tampered_logs.into_iter().map(Json::str).collect()),
        ),
        (
            "missing_logs",
            Json::Arr(missing_logs.into_iter().map(Json::str).collect()),
        ),
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

// --------------------------------------------------------------- proposals
pub fn cmd_propose(
    root: &Path,
    agent: Option<String>,
    task: Option<String>,
    base: Option<String>,
    patch: Option<String>,
    key: Option<String>,
) -> Result<Json, String> {
    let git = Git::new(root);
    let (Some(agent), Some(task), Some(base), Some(patch)) = (agent, task, base, patch) else {
        return Ok(crate::proposal::refusal(
            "USAGE",
            "flow propose --agent <A> --task <T> --base <commit> --patch <file> [--key <key>]",
        ));
    };
    crate::proposal::admit(
        &git,
        root,
        &agent,
        &task,
        &base,
        Path::new(&patch),
        key.as_deref(),
    )
}

pub fn cmd_proposals(root: &Path, show: Option<String>) -> Result<Json, String> {
    match show {
        Some(id) => match crate::proposal::Proposal::load(root, &id)? {
            Some(proposal) => Ok(proposal.json),
            None => Ok(crate::proposal::refusal(
                "UNKNOWN_PROPOSAL",
                format!("no admitted proposal {id:?} in .flow/proposals/"),
            )),
        },
        None => {
            let proposals = crate::proposal::Proposal::list(root)?;
            Ok(Json::obj(vec![
                ("status", Json::str("RECORD")),
                ("schema", Json::str(crate::state::SCHEMA_PROPOSAL_LIST)),
                (
                    "proposals",
                    Json::Arr(proposals.iter().map(|p| p.json.clone()).collect()),
                ),
                ("count", Json::Num(proposals.len() as f64)),
            ]))
        }
    }
}

pub fn cmd_assemble(root: &Path, proposals: &[String]) -> Result<Json, String> {
    crate::proposal::assemble(root, proposals)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The three parsers below consume answers from a *separate* Git process.
    // Their inputs are therefore untrusted: a corrupted object store, a Git that
    // answers nonsense, or a machine that truncates a pipe must produce a named
    // refusal, never a panic (which would skip `Drop` cleanup and could be
    // mistaken for a crash the operator has to debug). These tests pin that
    // boundary directly, including the arithmetic edges real Git would have to
    // reach to break it.

    #[test]
    fn program_resolution_and_identity_cover_explicit_missing_and_bare_names() {
        let root = std::env::temp_dir().join(format!(
            "flow-program-{}-{}",
            std::process::id(),
            state::unique_suffix()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = root.join("tool");
        fs::write(&file, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&file).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&file, perms).unwrap();
        }
        let explicit = file.to_string_lossy().to_string();
        assert_eq!(
            resolve_program_path(&explicit),
            Some(file.canonicalize().unwrap()),
            "an explicit existing path is used as given"
        );
        assert_eq!(
            resolve_program_path(&format!("{}/absent", root.display())),
            None,
            "an explicit missing path is not searched on PATH"
        );
        assert!(
            resolve_program_path("git").is_some(),
            "a bare name is searched on PATH"
        );
        let missing = program_identity("flow-definitely-absent-program");
        assert_eq!(
            missing.get("resolved").map(|v| v == &Json::Null),
            Some(true),
            "an absent program has no resolved path: {missing}"
        );
        // A declared probe with no program is marked INVALID and never executed.
        let config = Config {
            context_probes: vec![Vec::new()],
            ..Config::default()
        };
        let probes = probe_identity(&config, &root);
        let first = probes
            .as_arr()
            .and_then(|items| items.first())
            .and_then(|probe| probe.get("status"))
            .and_then(Json::as_str);
        assert_eq!(first, Some("INVALID"), "{probes}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn context_helpers_are_deterministic_and_do_not_leak_values() {
        let Json::Arr(entries) = env_identity() else {
            panic!("env identity is a list")
        };
        assert_eq!(entries.len(), gitcli::env_allowlist().len());
        for entry in entries {
            let digest = entry
                .get("value_digest")
                .and_then(Json::as_str)
                .unwrap_or("");
            assert!(
                digest == "ABSENT" || digest.starts_with("sha256:"),
                "unexpected digest {digest:?}"
            );
        }
        let missing = program_identity("flow-definitely-missing-program-xyz");
        assert!(missing
            .get("resolved")
            .map(|v| matches!(v, Json::Null))
            .unwrap_or(false));
        let git = program_identity("git");
        assert!(git.get("resolved").and_then(Json::as_str).is_some());

        let config = Config {
            context_probes: vec![Vec::new(), vec!["git".into(), "--version".into()]],
            ..Config::default()
        };
        let probes = probe_identity(&config, Path::new("."));
        let arr = probes.as_arr().expect("probes are a list");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].get("status").and_then(Json::as_str), Some("INVALID"));
        assert!(arr[1].get("status").and_then(Json::as_str).is_some());
    }

    fn entry(mode: &str, oid: &str, size: u64, path: &str) -> Vec<u8> {
        format!("{mode} blob {oid:>40} {size:>7}\t{path}\0").into_bytes()
    }

    #[test]
    fn tree_entries_are_parsed_with_size_and_normalised_separators() {
        let mut listing = entry("100644", "aaaa", 3, "a.txt");
        listing.extend(entry("100755", "bbbb", 5, "bin\\run"));
        listing.extend(entry("120000", "cccc", 1, "link"));
        let entries = parse_tree_entries("t", &listing).expect("a well-formed listing parses");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, "100644");
        assert_eq!(entries[0].1, "aaaa");
        assert_eq!(entries[0].2, "a.txt");
        assert_eq!(entries[0].3, 3);
        assert_eq!(
            entries[1].2, "bin/run",
            "a Windows Git lists backslashes; the tree's spelling is forward slashes"
        );
        assert_eq!(entries[2].0, "120000");
        assert!(
            parse_tree_entries("t", b"").unwrap().is_empty(),
            "no entries is not an error"
        );
    }

    #[test]
    fn tree_entries_refuse_every_unreadable_shape() {
        let no_tab = parse_tree_entries("t", b"100644 blob aaaa 0").expect_err("no tab");
        assert!(no_tab.contains("unparsable ls-tree entry"), "{no_tab}");

        let mut invalid = b"100644 blob aaaa 0\t".to_vec();
        invalid.push(0xff);
        invalid.push(b'\0');
        let reason = parse_tree_entries("t", &invalid).expect_err("non-UTF-8 path");
        assert!(reason.contains("not valid UTF-8"), "{reason}");

        // An entry whose path is empty is skipped, not folded into `""`.
        let empty_path = parse_tree_entries("t", b"100644 blob aaaa 0\t\0").unwrap();
        assert!(empty_path.is_empty());

        // A missing or unparsable size is 0, not a refusal: the size is a budget
        // hint, and an entry without one is still a real entry to compare.
        let no_size = parse_tree_entries("t", b"100644 blob aaaa\ta\0").unwrap();
        assert_eq!(no_size[0].3, 0);
        let bad_size = parse_tree_entries("t", b"100644 blob aaaa nope\ta\0").unwrap();
        assert_eq!(bad_size[0].3, 0);
    }

    #[test]
    fn program_on_path_finds_a_real_program_and_refuses_a_missing_name() {
        // `doctor` reports a toolchain as missing only if it is really missing: a
        // resolver that answers "absent" for everything would make every report
        // say UNSUPPORTED while a resolver that answers "present" would certify a
        // toolchain the host cannot run.
        assert!(program_on_path("git"), "git is on PATH in this suite");
        assert!(
            !program_on_path("definitely-not-a-program-flow-9f3"),
            "a name that cannot exist must not be reported as present"
        );
    }

    #[test]
    fn tree_entries_exactly_at_the_fidelity_budget_are_accepted_not_refused() {
        // The budget is a ceiling, not a threshold: `>` and `>=` differ exactly
        // here, and refusing a tree at the documented bound would make the limit
        // smaller than the documentation says.
        let mut listing = Vec::new();
        for index in 0..200_000u32 {
            listing.extend(entry("100644", "aaaa", 0, &format!("f{index}")));
        }
        let entries = parse_tree_entries("t", &listing).expect("exactly the budget is valid");
        assert_eq!(entries.len(), 200_000);
    }

    #[test]
    fn tree_entries_above_the_fidelity_budget_are_refused_not_truncated() {
        let mut listing = Vec::new();
        for index in 0..200_001u32 {
            listing.extend(entry("100644", "aaaa", 0, &format!("f{index}")));
        }
        let reason = parse_tree_entries("t", &listing).expect_err("above the entry budget");
        assert!(
            reason.contains("above the 200000-entry fidelity budget"),
            "{reason}"
        );
    }

    #[test]
    fn tree_dirs_are_parsed_and_refused_the_same_way() {
        let dirs = parse_tree_dirs(
            "t",
            b"040000 tree aaaa 0\tsrc\x00040000 tree bbbb 0\ta\\b\0",
        )
        .expect("well-formed");
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains("src"));
        assert!(dirs.contains("a/b"));

        let no_tab = parse_tree_dirs("t", b"040000 tree aaaa 0").expect_err("no tab");
        assert!(no_tab.contains("unparsable ls-tree -d entry"), "{no_tab}");

        let mut invalid = b"040000 tree aaaa 0\t".to_vec();
        invalid.push(0xff);
        invalid.push(b'\0');
        let reason = parse_tree_dirs("t", &invalid).expect_err("non-UTF-8 directory");
        assert!(reason.contains("not valid UTF-8"), "{reason}");
    }

    fn regular<'a>(pairs: &'a [(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        pairs.to_vec()
    }

    #[test]
    fn cat_file_batch_reads_the_blobs_in_request_order() {
        let wanted = regular(&[("aaaa", "a.txt"), ("bbbb", "b.txt")]);
        let bytes = b"aaaa blob 3\nabc\naaaa blob 0\n\n";
        let blobs = parse_cat_file_batch("t", &wanted, bytes).expect("well-formed batch");
        assert_eq!(blobs.len(), 2);
        assert_eq!(blobs[0], b"abc");
        assert!(blobs[1].is_empty());
    }

    #[test]
    fn cat_file_batch_accepts_an_ordinary_total_below_the_content_budget() {
        // The default chunk budget is 32 MiB, not 32 + 1024 * 1024 and not
        // 32 * 1024 + 1024: an ordinary batch of two 600 KiB blobs must be
        // accepted, so either collapsed constant turns this into a refusal.
        let wanted = regular(&[("aaaa", "a.bin"), ("bbbb", "b.bin")]);
        let blob = vec![b'x'; 600 * 1024];
        let mut bytes = Vec::new();
        for (oid, _) in &wanted {
            bytes.extend_from_slice(oid.as_bytes());
            bytes.extend_from_slice(format!(" blob {}\n", blob.len()).as_bytes());
            bytes.extend_from_slice(&blob);
            bytes.push(b'\n');
        }
        let blobs = parse_cat_file_batch("t", &wanted, &bytes).expect("1.2 MiB is ordinary");
        assert_eq!(blobs.len(), 2);
        assert_eq!(blobs[0].len(), blob.len());
    }

    #[test]
    fn cat_file_batch_refuses_truncation_non_blobs_and_bad_sizes() {
        let wanted = regular(&[("aaaa", "a.txt")]);

        let reason = parse_cat_file_batch("t", &wanted, b"aaaa blob 3").expect_err("no newline");
        assert!(
            reason.contains("truncated cat-file batch before"),
            "{reason}"
        );

        let reason =
            parse_cat_file_batch("t", &wanted, b"aaaa tree 3\nabc\n").expect_err("not a blob");
        assert!(
            reason.contains("resolved to \"tree\", not a blob"),
            "{reason}"
        );

        let reason =
            parse_cat_file_batch("t", &wanted, b"aaaa blob xx\nabc\n").expect_err("bad size");
        assert!(reason.contains("unparsable cat-file header"), "{reason}");

        let reason =
            parse_cat_file_batch("t", &wanted, b"aaaa blob 4\nabc").expect_err("short body");
        assert!(reason.contains("truncated cat-file batch in"), "{reason}");
    }

    #[test]
    fn cat_file_batch_never_panics_when_the_answer_ends_exactly_between_entries() {
        // The first blob ends the byte stream with no trailing newline. The
        // previous implementation advanced `position` past the end and sliced
        // `bytes[position..]` for the next entry, which panics. A hostile Git can
        // produce this, so it must be a refusal.
        let wanted = regular(&[("aaaa", "a.txt"), ("bbbb", "b.txt")]);
        let reason = parse_cat_file_batch("t", &wanted, b"aaaa blob 3\nabc")
            .expect_err("a batch that stops between entries is truncated");
        assert!(
            reason.contains("truncated cat-file batch before \"b.txt\""),
            "{reason}"
        );
    }

    #[test]
    fn cat_file_batch_refuses_an_overflowing_size_and_the_byte_budget() {
        let wanted = regular(&[("aaaa", "a.txt")]);
        let reason = parse_cat_file_batch("t", &wanted, b"aaaa blob 18446744073709551615\n")
            .expect_err("a size that cannot be a byte count");
        assert!(
            reason.contains("overflows a size") || reason.contains("unparsable cat-file header"),
            "{reason}"
        );

        // 64 MiB + 1: one byte over the declared budget, with the bytes present so
        // the refusal is the budget and not a truncation.
        let over = 64 * 1024 * 1024 + 1;
        let mut bytes = format!("aaaa blob {over}\n").into_bytes();
        bytes.extend(std::iter::repeat_n(b'x', over));
        bytes.push(b'\n');
        let reason = parse_cat_file_batch_with_budget("t", &wanted, &bytes, 64 * 1024 * 1024)
            .expect_err("over budget");
        assert!(reason.contains("content-fidelity budget"), "{reason}");
    }

    // ------------------------------------------------------ checkout proofs

    /// A `Git` that cannot answer for any tree: the calls below reach Git and
    /// receive a non-zero exit, which is the branch a corrupted object store
    /// produces ("cannot list tree", not a panic and not an empty listing).
    fn git_without_a_repository() -> Git {
        Git::new(std::env::temp_dir())
    }

    #[test]
    fn listing_a_tree_that_git_cannot_read_is_a_refusal() {
        let git = git_without_a_repository();
        let reason = ls_tree_entries(&git, "0000000").expect_err("no such tree");
        assert!(reason.contains("cannot list tree"), "{reason}");
        let reason = ls_tree_dirs(&git, "0000000").expect_err("no such tree");
        assert!(reason.contains("cannot list the directories"), "{reason}");
    }

    #[test]
    fn content_proofs_skip_when_no_regular_file_is_declared() {
        let git = git_without_a_repository();
        let scratch = std::env::temp_dir();
        assert!(
            verify_checkout_content(&git, &scratch, "t", &[], "during checkout").is_ok(),
            "an empty tree has no content to compare"
        );
        // Only symlinks, gitlinks and entries without a blob oid: nothing to read.
        let entries = vec![
            (
                "120000".to_string(),
                "aaaa".to_string(),
                "link".to_string(),
                0,
            ),
            (
                "160000".to_string(),
                "bbbb".to_string(),
                "sub".to_string(),
                0,
            ),
            ("100644".to_string(), String::new(), "no-oid".to_string(), 0),
        ];
        assert!(
            verify_checkout_content(&git, &scratch, "t", &entries, "during checkout").is_ok(),
            "a tree without regular blobs is not a content-comparison target"
        );
    }

    #[test]
    fn content_proofs_refuse_a_tree_above_the_entry_budget() {
        let git = git_without_a_repository();
        let scratch = std::env::temp_dir();
        // The documented envelope is one constant, asserted here; the boundary is
        // exercised with a three-entry budget so the test does not build a
        // 200 000-file checkout to prove an integer comparison.
        assert_eq!(
            CONTENT_BUDGET.entries, 200_000,
            "the documented envelope is 200 000 regular files"
        );
        assert_eq!(CONTENT_BUDGET.chunk_bytes, 32 * 1024 * 1024);
        assert_eq!(CONTENT_BUDGET.single_blob_bytes, 48 * 1024 * 1024);
        let budget = ContentBudget {
            entries: 3,
            ..CONTENT_BUDGET
        };
        let entries: Vec<(String, String, String, u64)> = (0..4)
            .map(|i| ("100644".to_string(), "aaaa".to_string(), format!("f{i}"), 4))
            .collect();
        let reason =
            verify_checkout_content_with(&git, &scratch, "t", &entries, "during checkout", budget)
                .expect_err("above the entry budget");
        assert!(
            reason.contains("above the 3-entry fidelity budget"),
            "{reason}"
        );
        // Exactly at the budget is compared, not refused: only the run below can
        // tell `>` from `>=`, and it reaches Git (which refuses here because the
        // directory is not a repository), so the entry check did not fire.
        let at_budget = &entries[..3];
        let reason =
            verify_checkout_content_with(&git, &scratch, "t", at_budget, "during checkout", budget)
                .expect_err("the budget itself is not the refusal");
        assert!(
            !reason.contains("entry fidelity budget"),
            "the entry budget fired at exactly the budget: {reason}"
        );
    }

    #[cfg(windows)]
    fn make_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        let out = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other("mklink /J failed"))
        }
    }

    #[cfg(unix)]
    fn make_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[test]
    fn collecting_the_checkout_refuses_an_unreadable_directory_and_never_follows_a_link() {
        let root = std::env::temp_dir().join(format!(
            "flow-collect-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("plain.txt");
        fs::write(&file, "x").unwrap();
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let reason = collect_entries(&root, &file, &mut files, &mut dirs)
            .expect_err("a file is not a directory to walk");
        assert!(reason.contains("cannot read"), "{reason}");

        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let link = root.join("link");
        if make_dir_link(&target, &link).is_ok() {
            files.clear();
            dirs.clear();
            collect_entries(&root, &root, &mut files, &mut dirs).expect("the tree is readable");
            assert!(
                !dirs.iter().any(|d| d == "link"),
                "a link is an entry, not a directory to descend into: {dirs:?}"
            );
            assert!(
                files.iter().any(|f| f == "link"),
                "a link to a directory must be collected as a file entry: {files:?}"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_preview_keeps_exactly_the_limit_and_marks_only_what_it_cut() {
        // The receipt's log preview is the operator's evidence of *what the
        // obligation printed*. It must be the first `limit` characters, with an
        // ellipsis only when something was cut: a preview that is empty, that
        // marks a text of exactly the limit, or that forgets the marks, all hide
        // output the operator needs to judge a failure.
        let exact = "é".repeat(400);
        assert_eq!(
            preview(&exact, 400),
            exact,
            "the limit itself is kept whole"
        );
        assert_eq!(preview("short", 400), "short", "a short text is not marked");

        let longer = "é".repeat(401);
        let cut = preview(&longer, 400);
        assert_eq!(
            cut.chars().count(),
            401,
            "400 kept characters and one ellipsis"
        );
        assert_eq!(cut.chars().next_back(), Some('…'));
        assert_eq!(cut.chars().take(400).collect::<String>(), exact);
    }

    #[test]
    fn cat_file_batch_accepts_a_total_of_exactly_the_chunk_budget() {
        // The chunk ceiling must be compared, not refused: `>` and `>=` (or `==`)
        // differ only at the boundary. The budget is a parameter so eight bytes
        // prove it instead of 32 MiB.
        const BUDGET: u64 = 16;
        let wanted = regular(&[("aaaa", "a.bin"), ("bbbb", "b.bin")]);
        let mut bytes = Vec::new();
        for (oid, _) in &wanted {
            bytes.extend_from_slice(oid.as_bytes());
            bytes.extend_from_slice(b" blob 8\n");
            bytes.extend_from_slice(b"12345678");
            bytes.push(b'\n');
        }
        let blobs = parse_cat_file_batch_with_budget("t", &wanted, &bytes, BUDGET)
            .expect("exactly the ceiling is not a refusal");
        assert_eq!(blobs.len(), 2);
        assert_eq!(blobs[0], b"12345678");

        let reason = parse_cat_file_batch_with_budget("t", &wanted, &bytes, BUDGET - 1)
            .expect_err("one byte over the ceiling is refused");
        assert!(reason.contains("content-fidelity budget"), "{reason}");
    }

    #[test]
    fn a_checkout_at_the_entry_budget_is_compared_and_one_over_is_refused() {
        // The references all name one real blob, so an accepted answer also proves
        // the comparison still reads and matches the checkout. The budget is three
        // entries: the boundary is the same integer comparison as 200 000, and the
        // defaults are asserted in
        // `content_proofs_refuse_a_tree_above_the_entry_budget`.
        let root = std::env::temp_dir().join(format!(
            "flow-content-budget-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("scratch")).unwrap();
        let init = std::process::Command::new("git")
            .arg("init")
            .arg("-q")
            .arg(&root)
            .status()
            .expect("git runs");
        assert!(init.success());
        fs::write(root.join("a.txt"), "alpha\n").unwrap();
        let oid = String::from_utf8(
            std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["hash-object", "-w", "a.txt"])
                .output()
                .expect("git runs")
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        fs::write(root.join("scratch").join("a.txt"), "alpha\n").unwrap();
        let git = Git::new(&root);
        let budget = ContentBudget {
            entries: 3,
            ..CONTENT_BUDGET
        };
        let expected: Vec<(String, String, String, u64)> = (0..3)
            .map(|_| ("100644".to_string(), oid.clone(), "a.txt".to_string(), 6))
            .collect();
        verify_checkout_content_with(
            &git,
            &root.join("scratch"),
            "t",
            &expected,
            "during checkout",
            budget,
        )
        .expect("exactly the entry budget is compared, not refused");

        let mut one_over = expected.clone();
        one_over.push(("100644".to_string(), oid.clone(), "a.txt".to_string(), 6));
        let reason = verify_checkout_content_with(
            &git,
            &root.join("scratch"),
            "t",
            &one_over,
            "during checkout",
            budget,
        )
        .expect_err("one entry over the budget");
        assert!(
            reason.contains("above the 3-entry fidelity budget"),
            "{reason}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tree_is_compared_in_several_chunks_without_skipping_one() {
        // A chunk budget smaller than two declared files forces one `cat-file`
        // request per file: if the chunk loop dropped or duplicated an entry, the
        // comparison would pass on the wrong file set. Two real blobs, two
        // chunks, both compared.
        let root = std::env::temp_dir().join(format!(
            "flow-content-chunks-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("scratch")).unwrap();
        let init = std::process::Command::new("git")
            .arg("init")
            .arg("-q")
            .arg(&root)
            .status()
            .expect("git runs");
        assert!(init.success());
        let mut expected: Vec<(String, String, String, u64)> = Vec::new();
        for (name, content) in [("a.txt", "alpha\n"), ("b.txt", "bravo\n")] {
            fs::write(root.join(name), content).unwrap();
            let oid = String::from_utf8(
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(&root)
                    .args(["hash-object", "-w", name])
                    .output()
                    .expect("git runs")
                    .stdout,
            )
            .unwrap()
            .trim()
            .to_string();
            fs::write(root.join("scratch").join(name), content).unwrap();
            expected.push((
                "100644".to_string(),
                oid,
                name.to_string(),
                content.len() as u64,
            ));
        }
        let git = Git::new(&root);
        verify_checkout_content_with(
            &git,
            &root.join("scratch"),
            "t",
            &expected,
            "during checkout",
            ContentBudget {
                chunk_bytes: 1,
                ..CONTENT_BUDGET
            },
        )
        .expect("two chunks, both compared");

        // One wrong byte in the second chunk is still caught: the second request
        // is not skipped.
        fs::write(root.join("scratch").join("b.txt"), "BRAVO\n").unwrap();
        let reason = verify_checkout_content_with(
            &git,
            &root.join("scratch"),
            "t",
            &expected,
            "during checkout",
            ContentBudget {
                chunk_bytes: 1,
                ..CONTENT_BUDGET
            },
        )
        .expect_err("the second chunk is compared too");
        assert!(reason.contains("\"b.txt\" changed"), "{reason}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_single_file_above_the_per_file_budget_is_refused_by_name() {
        let git = git_without_a_repository();
        let scratch = std::env::temp_dir();
        let entries = vec![(
            "100644".to_string(),
            "aaaa".to_string(),
            "huge.bin".to_string(),
            49 * 1024 * 1024,
        )];
        let reason = verify_checkout_content(&git, &scratch, "t", &entries, "during checkout")
            .expect_err("above the per-file budget");
        assert!(reason.contains("\"huge.bin\""), "{reason}");
        assert!(reason.contains("single-file comparison budget"), "{reason}");
    }

    // ---------------------------------------------------------------- gc
    const GC_CONFIG: &str = r#"{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[],"disabled":false}"#;

    fn gc_repo(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "flow-gc-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(state::receipts_dir(&root)).unwrap();
        fs::create_dir_all(state::logs_dir(&root)).unwrap();
        fs::create_dir_all(state::candidates_dir(&root)).unwrap();
        fs::write(state::config_path(&root), GC_CONFIG).unwrap();
        fs::write(state::journal_path(&root), "").unwrap();
        root
    }

    fn gc_tree_id(seed: usize) -> String {
        let mut id = "0".repeat(38);
        id.push_str(&format!("{seed:02x}"));
        assert!(crate::paths::is_object_id(&id));
        id
    }

    fn gc_receipt(root: &Path, seed: usize) -> PathBuf {
        let path = state::receipt_path(root, &gc_tree_id(seed));
        fs::write(&path, "{}").unwrap();
        path
    }

    fn gc_log(root: &Path, seed: usize) -> PathBuf {
        let dir = state::logs_dir(root).join(gc_tree_id(seed));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("fmt.stdout.log"), "log\n").unwrap();
        dir
    }

    fn gc_age_days(path: &Path, days: u64) {
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86_400);
        let file = fs::File::options().write(true).open(path).expect("open");
        file.set_modified(when).expect("set mtime");
    }

    fn gc_deleted(value: &Json) -> Vec<String> {
        value
            .get("deleted")
            .and_then(|v| match v {
                Json::Arr(items) => Some(
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[test]
    fn gc_refuses_without_an_explicit_policy() {
        let root = gc_repo("no-policy");
        let value = cmd_gc(&root, None, None, true).expect("a refusal is a document");
        assert_eq!(value.get("code").and_then(Json::as_str), Some("USAGE"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_keeps_the_newest_receipts_and_their_logs() {
        let root = gc_repo("keep");
        // Three receipts, written oldest first so the mtime order is the seed
        // order; the newest is seed 2.
        for seed in [0usize, 1, 2] {
            gc_receipt(&root, seed);
            std::thread::sleep(std::time::Duration::from_millis(20));
            gc_log(&root, seed);
        }
        // An orphan log: no receipt names it, so nothing protects it.
        let orphan = state::logs_dir(&root).join(gc_tree_id(9));
        fs::create_dir_all(&orphan).unwrap();

        let value = cmd_gc(&root, Some(1), None, false).expect("a plan");
        assert_eq!(value.get("status").and_then(Json::as_str), Some("GC"));
        let deleted = gc_deleted(&value);
        assert!(
            deleted.contains(&format!("receipts/{}.json", gc_tree_id(0))),
            "{deleted:?}"
        );
        assert!(
            deleted.contains(&format!("receipts/{}.json", gc_tree_id(1))),
            "{deleted:?}"
        );
        assert!(
            !deleted.contains(&format!("receipts/{}.json", gc_tree_id(2))),
            "the newest is kept: {deleted:?}"
        );
        assert!(
            deleted.contains(&format!("logs/{}/", gc_tree_id(0))),
            "a deleted receipt takes its log: {deleted:?}"
        );
        assert!(
            deleted.contains(&format!("logs/{}/", gc_tree_id(9))),
            "the orphan log goes: {deleted:?}"
        );
        assert!(
            state::receipt_path(&root, &gc_tree_id(2)).exists(),
            "the kept receipt survives"
        );
        assert!(
            state::logs_dir(&root).join(gc_tree_id(2)).exists(),
            "the kept receipt keeps its log"
        );
        assert!(!state::receipt_path(&root, &gc_tree_id(0)).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_dry_run_reports_the_same_plan_and_removes_nothing() {
        let root = gc_repo("dry");
        for seed in [0usize, 1] {
            gc_receipt(&root, seed);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let plan = cmd_gc(&root, Some(0), None, true).expect("a plan");
        assert_eq!(
            plan.get("dry_run").and_then(|v| match v {
                Json::Bool(b) => Some(*b),
                _ => None,
            }),
            Some(true)
        );
        assert!(!gc_deleted(&plan).is_empty());
        assert!(
            state::receipt_path(&root, &gc_tree_id(0)).exists(),
            "a dry run deletes nothing"
        );
        assert!(
            state::receipt_path(&root, &gc_tree_id(1)).exists(),
            "a dry run deletes nothing"
        );
        // The same policy without --dry-run deletes them, so the plan was not a lie.
        let real = cmd_gc(&root, Some(0), None, false).expect("a plan");
        assert_eq!(gc_deleted(&real).len(), gc_deleted(&plan).len());
        assert!(!state::receipt_path(&root, &gc_tree_id(0)).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_removes_old_evidence_and_keeps_fresh_evidence() {
        let root = gc_repo("age");
        let old = gc_receipt(&root, 0);
        gc_age_days(&old, 40);
        let fresh = gc_receipt(&root, 1);
        gc_age_days(&fresh, 1);
        let candidate = state::candidates_dir(&root).join(format!("{}.json", "a".repeat(32)));
        fs::write(&candidate, "{}").unwrap();
        gc_age_days(&candidate, 40);

        let value = cmd_gc(&root, None, Some(30), false).expect("a plan");
        let deleted = gc_deleted(&value);
        assert!(
            deleted.contains(&format!("receipts/{}.json", gc_tree_id(0))),
            "{deleted:?}"
        );
        assert!(
            !deleted.contains(&format!("receipts/{}.json", gc_tree_id(1))),
            "1 day old is fresh: {deleted:?}"
        );
        assert!(
            deleted.iter().any(|item| item.starts_with("candidates/")),
            "{deleted:?}"
        );
        assert!(fresh.exists());
        assert!(!old.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_never_removes_a_file_it_cannot_name_or_a_link() {
        let root = gc_repo("foreign");
        let notes = state::receipts_dir(&root).join("notes.txt");
        fs::write(&notes, "not evidence").unwrap();
        let value = cmd_gc(&root, Some(0), Some(1), false).expect("a plan");
        assert!(
            notes.exists(),
            "a file that is not an object id is not Flow's to remove"
        );
        let skipped = value.get("skipped").and_then(|v| match v {
            Json::Arr(items) => Some(items.len()),
            _ => None,
        });
        assert!(
            skipped.unwrap_or(0) >= 1,
            "the refusal is reported, not silent"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_refuses_while_a_publication_intent_is_unresolved() {
        let root = gc_repo("pending");
        gc_receipt(&root, 0);
        let intent = r#"{"schema":"flow-journal/1","id":"intent-1","event":"intent","ref":"refs/heads/main","old":"0","new":"1"}"#;
        fs::write(state::journal_path(&root), format!("{intent}\n")).unwrap();
        let value = cmd_gc(&root, Some(0), None, false).expect("a refusal is a document");
        assert_eq!(
            value.get("code").and_then(Json::as_str),
            Some("PENDING_INTENTS")
        );
        assert!(
            state::receipt_path(&root, &gc_tree_id(0)).exists(),
            "nothing is deleted during recovery"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_collects_partial_receipts_plain_log_files_and_unnameable_candidates() {
        let root = gc_repo("partial");
        // A partial receipt from a selected run: never publishable, so it follows
        // the age policy and is never allowed to displace a complete receipt.
        let partial = state::receipts_dir(&root).join(format!(
            "{}.partial.{}.json",
            gc_tree_id(3),
            "abcdef123456"
        ));
        fs::write(&partial, "{}").unwrap();
        // A `.json` name that is not an object id is not Flow's to remove.
        let foreign_json = state::receipts_dir(&root).join("not-an-oid.json");
        fs::write(&foreign_json, "{}").unwrap();
        // A log entry that is a plain file, not a directory: measurable, listable
        // and removable without ever following a link.
        let stray = state::logs_dir(&root).join("stray");
        fs::write(&stray, "one byte").unwrap();
        // Candidates: a non-JSON name is left alone, a `.json` name that is not a
        // record id is reported as skipped, and a real record id is collected.
        fs::write(state::candidates_dir(&root).join("notes.txt"), "x").unwrap();
        fs::write(state::candidates_dir(&root).join("bad-name.json"), "{}").unwrap();
        let candidate = state::candidates_dir(&root).join(format!("{}.json", "a".repeat(32)));
        fs::write(&candidate, "{}").unwrap();

        // `--keep` alone governs receipts and logs; it must not collect a fresh
        // candidate or a fresh partial receipt the operator still needs.
        let keep_only = cmd_gc(&root, Some(0), None, false).expect("a plan");
        let deleted = gc_deleted(&keep_only);
        assert!(
            partial.exists(),
            "--keep alone must not delete a fresh partial receipt: {deleted:?}"
        );
        assert!(
            candidate.exists(),
            "--keep alone must not delete a fresh candidate: {deleted:?}"
        );
        assert!(deleted.contains(&"logs/stray/".to_string()), "{deleted:?}");
        assert!(!stray.exists(), "a plain log entry is collected");

        // An explicit age policy collects them.
        gc_age_days(&partial, 40);
        gc_age_days(&candidate, 40);
        let value = cmd_gc(&root, Some(0), Some(30), false).expect("a plan");
        let deleted = gc_deleted(&value);
        assert!(
            deleted.iter().any(|item| item.contains(".partial.")),
            "{deleted:?}"
        );
        assert!(
            deleted.iter().any(|item| item.starts_with("candidates/")),
            "{deleted:?}"
        );
        assert!(!partial.exists());
        assert!(!candidate.exists());
        assert!(foreign_json.exists(), "a non-object-id receipt stays");
        assert!(state::candidates_dir(&root).join("notes.txt").exists());
        assert!(state::candidates_dir(&root).join("bad-name.json").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_keep_only_keeps_fresh_candidates_and_partial_receipts() {
        let root = gc_repo("keep-only");
        let partial = state::receipts_dir(&root).join(format!(
            "{}.partial.{}.json",
            gc_tree_id(8),
            "abcdef123456"
        ));
        fs::write(&partial, "{}").unwrap();
        let candidate = state::candidates_dir(&root).join(format!("{}.json", "d".repeat(32)));
        fs::write(&candidate, "{}").unwrap();
        let value = cmd_gc(&root, Some(1), None, false).expect("a plan");
        assert_eq!(
            value.get("status").and_then(Json::as_str),
            Some("GC"),
            "{value}"
        );
        assert!(partial.exists(), "no age policy: a partial receipt is kept");
        assert!(candidate.exists(), "no age policy: a candidate is kept");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_keeps_fresh_partial_receipts_candidates_and_logs_and_ages_the_rest() {
        let root = gc_repo("partial-age");
        // A fresh complete receipt keeps its log without touching the age policy.
        gc_receipt(&root, 0);
        gc_log(&root, 0);
        // An old complete receipt is collected.
        let old_complete = gc_receipt(&root, 1);
        gc_age_days(&old_complete, 40);
        // A fresh partial receipt protects the tree its name prefixes.
        let fresh_partial = state::receipts_dir(&root).join(format!(
            "{}.partial.{}.json",
            gc_tree_id(4),
            "abcdef123456"
        ));
        fs::write(&fresh_partial, "{}").unwrap();
        gc_log(&root, 4);
        // An old partial receipt is collected, but a log directory with a fresh
        // mtime is not old evidence: it follows the age policy, not the receipt.
        let old_partial = state::receipts_dir(&root).join(format!(
            "{}.partial.{}.json",
            gc_tree_id(5),
            "abcdef123456"
        ));
        fs::write(&old_partial, "{}").unwrap();
        gc_age_days(&old_partial, 40);
        gc_log(&root, 5);
        let fresh_candidate = state::candidates_dir(&root).join(format!("{}.json", "b".repeat(32)));
        fs::write(&fresh_candidate, "{}").unwrap();
        let old_candidate = state::candidates_dir(&root).join(format!("{}.json", "c".repeat(32)));
        fs::write(&old_candidate, "{}").unwrap();
        gc_age_days(&old_candidate, 40);

        let value = cmd_gc(&root, None, Some(30), false).expect("a plan");
        let deleted = gc_deleted(&value);
        assert!(fresh_partial.exists(), "a fresh partial receipt is kept");
        assert!(!old_partial.exists(), "an old partial receipt is aged out");
        assert!(
            state::logs_dir(&root).join(gc_tree_id(0)).exists(),
            "the log of a kept receipt survives"
        );
        assert!(
            state::logs_dir(&root).join(gc_tree_id(4)).exists(),
            "the log of a fresh partial receipt survives"
        );
        assert!(
            state::logs_dir(&root).join(gc_tree_id(5)).exists(),
            "a log directory is judged by its own age, not by a deleted receipt"
        );
        assert!(fresh_candidate.exists(), "a fresh candidate is kept");
        assert!(!old_candidate.exists(), "an old candidate is collected");
        assert!(
            deleted.iter().all(|item| !item.contains(&"b".repeat(32))),
            "kept evidence is never reported as deleted: {deleted:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gc_interprets_run_markers_and_clears_only_the_provably_stale_ones() {
        let root = gc_repo("markers");
        let runs = state::runs_dir(&root);
        fs::create_dir_all(&runs).unwrap();
        // A file that is not a marker is not read at all.
        fs::write(runs.join("notes.txt"), "not a marker").unwrap();
        // A marker that is a directory cannot be read. It names no run, so gc
        // ignores it: an entry it cannot interpret is never a reason to delete
        // evidence, and never a reason to invent a live run either.
        fs::create_dir_all(runs.join("unreadable.active")).unwrap();
        // A marker whose JSON is broken names no owner: ignored for the same reason.
        fs::write(runs.join("broken.active"), "{not json").unwrap();
        let value = cmd_gc(&root, Some(0), None, false).expect("a plan");
        assert_eq!(value.get("status").and_then(Json::as_str), Some("GC"));
        assert!(
            runs.join("unreadable.active").exists(),
            "an unreadable marker is left for a human, not deleted"
        );
        assert!(runs.join("broken.active").exists());
        fs::remove_dir_all(runs.join("unreadable.active")).unwrap();
        fs::remove_file(runs.join("broken.active")).unwrap();

        // A marker naming this very process is a live run: gc refuses by name.
        let live = runs.join("live.active");
        fs::write(
            &live,
            format!(
                r#"{{"run_id":"run-live","pid":{},"started_at":"2026-01-01T00:00:00Z"}}"#,
                std::process::id()
            ),
        )
        .unwrap();
        let value = cmd_gc(&root, Some(0), None, false).expect("a refusal is a document");
        assert_eq!(value.get("code").and_then(Json::as_str), Some("RUN_ACTIVE"));
        assert!(live.exists(), "a live marker is never cleared");
        fs::remove_file(&live).unwrap();

        // A marker whose pid is provably dead is stale: gc clears it and proceeds.
        let stale = runs.join("stale.active");
        fs::write(
            &stale,
            r#"{"run_id":"run-1","pid":4194304,"started_at":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();
        let value = cmd_gc(&root, Some(0), None, false).expect("a plan");
        assert_eq!(value.get("status").and_then(Json::as_str), Some("GC"));
        assert!(!stale.exists(), "a provably stale marker is cleared");
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn gc_refuses_to_follow_a_link_in_receipts_or_logs() {
        use std::os::unix::fs::symlink;
        let root = gc_repo("links");
        let outside = std::env::temp_dir().join(format!(
            "flow-gc-outside-{}-{}",
            std::process::id(),
            state::unique_suffix()
        ));
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("evidence.json"), "outside").unwrap();
        let receipt_link = state::receipts_dir(&root).join(format!("{}.json", gc_tree_id(1)));
        symlink(outside.join("evidence.json"), &receipt_link).unwrap();
        let log_link = state::logs_dir(&root).join(gc_tree_id(2));
        symlink(&outside, &log_link).unwrap();

        let value = cmd_gc(&root, Some(0), None, false).expect("a plan");
        let skipped = value
            .get("skipped")
            .and_then(Json::as_arr)
            .map(|a| a.len())
            .unwrap_or(0);
        assert!(skipped >= 2, "both links are reported as skipped: {value}");
        assert!(
            outside.join("evidence.json").exists(),
            "a link is never followed out of the workspace"
        );
        assert!(
            fs::symlink_metadata(&receipt_link).is_ok(),
            "a link is never a removal target"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn gc_refuses_on_a_lock_it_can_attribute_to_a_live_writer() {
        let root = gc_repo("locked");
        let host = crate::host::local_host_id().expect("the test host has an identity");
        fs::write(
            crate::lock::lock_path(&root),
            format!(
                "{} {} 0 testnonce gc host:{host}\n",
                crate::lock::LOCK_SCHEMA,
                std::process::id()
            ),
        )
        .unwrap();
        let value = cmd_gc(&root, Some(0), None, false).expect("a refusal is a document");
        assert_eq!(value.get("code").and_then(Json::as_str), Some("LOCKED"));
        let _ = fs::remove_dir_all(&root);
    }

    // A deterministic, dependency-free generator. The crate is deliberately
    // zero-dependency and CI runs `--locked --offline`, so no `proptest` or
    // `arbitrary` may be added just to answer "is it fuzzed?". This is a
    // randomised sweep on every build, not coverage-guided fuzzing, and it is
    // never presented as such.
    struct Rand(u64);

    impl Rand {
        fn below(&mut self, bound: usize) -> usize {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            (x.wrapping_mul(0x2545_F491_4F6C_DD1D) % bound as u64) as usize
        }

        fn bytes(&mut self, max: usize) -> Vec<u8> {
            const ALPHABET: &[u8] = b"\0\t\n /\\:.-_a0\xff";
            let len = self.below(max + 1);
            let mut out = Vec::with_capacity(len);
            for _ in 0..len {
                if self.below(4) == 0 {
                    out.push(self.below(256) as u8);
                } else {
                    out.push(ALPHABET[self.below(ALPHABET.len())]);
                }
            }
            out
        }
    }

    fn random_path(random: &mut Rand) -> Vec<u8> {
        const ALPHABET: &[u8] = b"ab/\\\t\xff.";
        let len = random.below(7);
        (0..len)
            .map(|_| ALPHABET[random.below(ALPHABET.len())])
            .collect()
    }

    #[test]
    fn arbitrary_ls_tree_answers_are_parsed_or_refused_and_never_panic() {
        let mut random = Rand(0x5EED_1234_ABCD_0001);
        let mut parsed = 0usize;
        let mut refused = 0usize;
        for _ in 0..2_000 {
            let listing = if random.below(2) == 0 {
                let mut out = Vec::new();
                for _ in 0..(1 + random.below(3)) {
                    out.extend_from_slice(b"100644 blob aaaa 0\t");
                    out.extend_from_slice(&random_path(&mut random));
                    out.push(0);
                }
                out
            } else {
                random.bytes(96)
            };
            match parse_tree_entries("t", &listing) {
                Ok(entries) => {
                    parsed += 1;
                    for (mode, oid, path, _) in entries {
                        assert!(!path.is_empty(), "an empty path is skipped, never recorded");
                        assert!(!path.contains('\\'), "separators are normalised: {path:?}");
                        let _ = (mode, oid);
                    }
                }
                Err(reason) => {
                    refused += 1;
                    assert!(reason.starts_with("tree t"), "{reason}");
                }
            }
            match parse_tree_dirs("t", &listing) {
                Ok(dirs) => {
                    for dir in dirs {
                        assert!(!dir.contains('\\'), "separators are normalised: {dir:?}");
                    }
                }
                Err(reason) => assert!(reason.starts_with("tree t"), "{reason}"),
            }
        }
        assert!(
            parsed > 100 && refused > 100,
            "both answers must be exercised, got {parsed} parsed and {refused} refused"
        );
    }

    #[test]
    fn arbitrary_cat_file_answers_are_parsed_or_refused_and_never_panic() {
        let mut random = Rand(0x5EED_2026_0BAD_F00D);
        let mut parsed = 0usize;
        let mut refused = 0usize;
        for _ in 0..2_000 {
            let count = 1 + random.below(4);
            let oids: Vec<String> = (0..count)
                .map(|index| format!("{:040x}", random.below(1 << 16) + index))
                .collect();
            let pairs: Vec<(&str, &str)> = oids.iter().map(|oid| (oid.as_str(), "f")).collect();
            let mut well_formed = Vec::new();
            for (oid, _) in &pairs {
                let body = random.bytes(8);
                well_formed.extend_from_slice(oid.as_bytes());
                well_formed.extend_from_slice(b" blob ");
                well_formed.extend_from_slice(body.len().to_string().as_bytes());
                well_formed.push(b'\n');
                well_formed.extend_from_slice(&body);
                well_formed.push(b'\n');
            }
            let bytes = match random.below(3) {
                0 => random.bytes(96),
                1 => well_formed[..random.below(well_formed.len() + 1)].to_vec(),
                _ => well_formed,
            };
            match parse_cat_file_batch("t", &pairs, &bytes) {
                Ok(blobs) => {
                    parsed += 1;
                    assert_eq!(blobs.len(), pairs.len(), "one blob per requested oid");
                }
                Err(reason) => {
                    refused += 1;
                    assert!(!reason.is_empty());
                }
            }
        }
        assert!(
            parsed > 100 && refused > 100,
            "both answers must be exercised, got {parsed} parsed and {refused} refused"
        );

        // The header that used to overflow the position arithmetic is a refusal,
        // not a panic (see the historical `checked_add` fix).
        let wanted: Vec<(&str, &str)> = vec![("aaaa", "a.txt")];
        let huge = format!("aaaa blob {}\n", usize::MAX);
        let reason =
            parse_cat_file_batch("t", &wanted, huge.as_bytes()).expect_err("overflowing size");
        assert!(reason.contains("overflows a size"), "{reason}");
    }
}
