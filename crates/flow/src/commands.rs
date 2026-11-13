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
            "note",
            Json::str("this working copy now approves exactly these obligations; editing .flow/config.json again requires `flow approve`"),
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
                (
                    "path",
                    Json::str(state::config_path(root).to_string_lossy().to_string()),
                ),
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

// ------------------------------------------------------- policy identity
fn obligation_json(o: &Obligation) -> Json {
    Json::obj(vec![
        ("id", Json::str(o.id.clone())),
        ("program", Json::str(o.program.clone())),
        (
            "args",
            Json::Arr(o.args.iter().map(|a| Json::str(a.clone())).collect()),
        ),
        ("required", Json::Bool(o.required)),
        ("timeout_s", Json::Num(o.timeout_s as f64)),
    ])
}

/// The effective verification policy: every input that changes what a `PASSED`
/// means. Anything that can silently weaken the gate belongs here.
fn effective_policy(config: &Config, obligations: &[Obligation]) -> Json {
    Json::obj(vec![
        ("policy_version", Json::str(config.policy_version.clone())),
        (
            "execution_boundary",
            Json::str(config.execution_boundary.clone()),
        ),
        (
            "obligations",
            Json::Arr(obligations.iter().map(obligation_json).collect()),
        ),
        (
            "obligation_env",
            Json::Arr(
                gitcli::env_allowlist()
                    .iter()
                    .map(|n| Json::str(*n))
                    .collect(),
            ),
        ),
    ])
}

fn policy_digest(policy: &Json) -> String {
    sha256::sha256_hex(policy.to_string().as_bytes())
}

fn build_context_id(git: &Git, tree: &str, policy: &Json) -> String {
    let context = Json::obj(vec![
        ("codec", Json::str(state::CONTEXT_CODEC)),
        ("tree_oid", Json::str(tree)),
        ("policy", policy.clone()),
        ("git_version", Json::str(git.version())),
        ("os", Json::str(std::env::consts::OS)),
        ("arch", Json::str(std::env::consts::ARCH)),
    ]);
    sha256::sha256_hex(context.to_string().as_bytes())
}

/// Refuse a policy that cannot express a meaningful proof, or whose ids would
/// collide as log file names.
fn validate_obligations(obligations: &[Obligation]) -> Result<(), String> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut folded: BTreeMap<String, String> = BTreeMap::new();
    for ob in obligations {
        crate::paths::validate_relative(&format!(".flow/logs/{}", ob.id))
            .map_err(|reason| format!("unsafe obligation id {:?}: {reason}", ob.id))?;
        if !seen.insert(ob.id.as_str()) {
            return Err(format!(
                "duplicate obligation id {:?}: ids key the logs and the receipt, so they must be unique",
                ob.id
            ));
        }
        // Log files are named after the id. On a case-insensitive filesystem two
        // ids differing only by case would write to one file and lose evidence,
        // so the collision is refused on every platform.
        if let Some(other) = folded.get(&ob.id.to_lowercase()) {
            return Err(format!(
                "obligation ids {:?} and {:?} differ only by case: their log files would collide on a case-insensitive filesystem",
                other, ob.id
            ));
        }
        folded.insert(ob.id.to_lowercase(), ob.id.clone());
    }
    if !obligations.iter().any(|o| o.required) {
        return Err(
            "the policy declares no required obligation: nothing would be proven, so no receipt is publishable"
                .into(),
        );
    }
    Ok(())
}

// -------------------------------------------------------------------- run

/// Refuse to certify a checkout that is not a faithful copy of the tree.
///
/// `git worktree add` exits 0, with an empty stderr, on trees the host cannot
/// represent: two paths differing only by case on a case-insensitive filesystem,
/// a symlink when `core.symlinks=false`, a gitlink whose content is not
/// materialised. Without this check a required obligation runs on content that is
/// not the verified tree while the receipt claims the tree was checked, which is
/// a false `PASSED` with no tampering involved.
///
/// This is a statement about the checkout **at the moment it is checked**: a hook
/// (or an obligation) can change it afterwards. `verify_checkout_unchanged` runs
/// the same byte and mode proof again after the last obligation and refuses a run
/// whose declared files changed meanwhile.
fn verify_checkout_fidelity(git: &Git, scratch: &Path, tree: &str) -> Result<(), String> {
    let expected = ls_tree_entries(git, tree)?;

    // A gitlink is refused for what it is, **before** the presence check: a
    // checkout never materialises submodule content, so a gitlink entry would
    // otherwise be reported as "the checkout is missing 1 of N entries", which
    // names the symptom and hides the cause. The refusal is still a refusal.
    if let Some((_, _, path, _)) = expected.iter().find(|(mode, _, _, _)| mode == "160000") {
        return Err(format!(
            "tree {tree} entry {path:?} is a gitlink: submodule content is not materialised, so the tree cannot be verified"
        ));
    }

    // Two paths differing only by case cannot both exist on a case-insensitive
    // filesystem: the checkout silently keeps one of them.
    let mut folded: BTreeMap<String, usize> = BTreeMap::new();
    for (_, _, path, _) in &expected {
        *folded.entry(path.to_lowercase()).or_insert(0) += 1;
    }
    if let Some((name, count)) = folded.iter().find(|(_, count)| **count > 1) {
        return Err(format!(
            "tree {tree} holds {count} paths that differ only by case (for example {name:?}): this host cannot represent them faithfully, so the obligations would not see the verified tree"
        ));
    }

    // Everything the tree names must actually exist on disk, and nothing else may
    // be present. `git worktree add` runs the repository's `post-checkout` hook
    // (including one reached through `core.hooksPath` pointing at a directory the
    // tree itself ships), so the checkout can gain files and directories after it
    // was written. An extra path is not cosmetic: a toolchain file, a
    // `.cargo/config.toml`, an empty cache directory or a test found by a glob
    // changes what an obligation sees without touching a byte the tree declares.
    let mut actual_files: Vec<String> = Vec::new();
    let mut actual_dirs: Vec<String> = Vec::new();
    collect_entries(scratch, scratch, &mut actual_files, &mut actual_dirs)?;
    let present: BTreeSet<&str> = actual_files.iter().map(String::as_str).collect();
    let mut missing: Vec<&str> = expected
        .iter()
        .map(|(_, _, path, _)| path.as_str())
        .filter(|path| !present.contains(path))
        .collect();
    if !missing.is_empty() {
        missing.sort_unstable();
        return Err(format!(
            "the checkout is missing {} of {} tree entries (for example {:?}): the obligations would not see the verified tree",
            missing.len(),
            expected.len(),
            &missing[..missing.len().min(3)]
        ));
    }
    let declared: BTreeSet<&str> = expected
        .iter()
        .map(|(_, _, path, _)| path.as_str())
        .collect();
    let mut unexpected: Vec<&str> = present
        .iter()
        .copied()
        .filter(|path| !declared.contains(path))
        .collect();
    if !unexpected.is_empty() {
        unexpected.sort_unstable();
        return Err(format!(
            "the checkout holds {} file(s) the tree does not declare (for example {:?}): a hook or the host created them, so the obligations would not see exactly the verified tree",
            unexpected.len(),
            &unexpected[..unexpected.len().min(3)]
        ));
    }
    let declared_dirs = ls_tree_dirs(git, tree)?;
    let mut unexpected_dirs: Vec<&str> = actual_dirs
        .iter()
        .map(String::as_str)
        .filter(|dir| !declared_dirs.contains(*dir))
        .collect();
    if !unexpected_dirs.is_empty() {
        unexpected_dirs.sort_unstable();
        return Err(format!(
            "the checkout holds {} directory(ies) the tree does not declare (for example {:?}): a hook or the host created them, and an empty directory can change what an obligation discovers",
            unexpected_dirs.len(),
            &unexpected_dirs[..unexpected_dirs.len().min(3)]
        ));
    }

    // And each entry must have the type the tree declares.
    for (mode, _, path, _) in &expected {
        let full = scratch.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        let meta = fs::symlink_metadata(&full)
            .map_err(|e| format!("cannot stat {path:?} after checkout: {e}"))?;
        match mode.as_str() {
            "120000" if !meta.file_type().is_symlink() => {
                return Err(format!(
                    "tree {tree} entry {path:?} is a symlink but was checked out as a regular file (core.symlinks=false): refusing to certify it"
                ));
            }
            "100644" | "100755" if !meta.is_file() => {
                return Err(format!(
                    "tree {tree} entry {path:?} (mode {mode}) was not checked out as a regular file"
                ));
            }
            _ => {}
        }
    }

    verify_checkout_modes(scratch, tree, &expected)?;
    verify_checkout_content(git, scratch, tree, &expected, "during checkout")
}

/// The tree's entries: `(mode, blob oid, path, size)`, from one bounded
/// `ls-tree -r -l -z`. A path that is not valid UTF-8 is refused with an explicit
/// reason instead of being lossily folded into `U+FFFD` (two different paths would
/// then look like a case collision, which is a misleading diagnosis).
fn ls_tree_entries(git: &Git, tree: &str) -> Result<Vec<(String, String, String, u64)>, String> {
    // The listing comes from the source repository: the checkout deliberately
    // has no `.git`, so it cannot answer for itself.
    let listing = git.run(&["ls-tree", "-r", "-l", "-z", tree])?;
    if listing.status != 0 {
        return Err(format!(
            "cannot list tree {tree}: {}",
            listing.stderr.trim()
        ));
    }
    parse_tree_entries(tree, &listing.stdout_bytes)
}

/// The parsing half of [`ls_tree_entries`], separated so every refusal can be
/// pinned against a crafted Git answer (a hostile or broken Git is an input, not
/// an oracle): the walk never panics, and an entry it cannot read is refused.
fn parse_tree_entries(
    tree: &str,
    listing: &[u8],
) -> Result<Vec<(String, String, String, u64)>, String> {
    /// A tree beyond this size is not a realistic verification target; refusing
    /// keeps the walk bounded.
    const MAX_ENTRIES: usize = 200_000;

    let mut expected: Vec<(String, String, String, u64)> = Vec::new();
    for entry in listing.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else {
            return Err(format!("tree {tree}: unparsable ls-tree entry"));
        };
        let meta = String::from_utf8_lossy(&entry[..tab]).to_string();
        let path = std::str::from_utf8(&entry[tab + 1..]).map_err(|_| {
            format!(
                "tree {tree} holds a path that is not valid UTF-8: this platform cannot represent it faithfully, so the tree cannot be verified"
            )
        })?;
        if path.is_empty() {
            continue;
        }
        let mut fields = meta.split_whitespace();
        let mode = fields.next().unwrap_or("").to_string();
        let _kind = fields.next().unwrap_or("");
        let oid = fields.next().unwrap_or("").to_string();
        // `ls-tree -l` prints the object size, right-aligned, before the tab.
        let size = fields
            .next()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        expected.push((mode, oid, path.replace('\\', "/"), size));
    }
    if expected.len() > MAX_ENTRIES {
        return Err(format!(
            "tree {tree} has {} entries, above the {MAX_ENTRIES}-entry fidelity budget",
            expected.len()
        ));
    }
    Ok(expected)
}

/// The directories the tree declares (`ls-tree -r -d`), so an **empty** directory
/// created by a hook is visible: Git does not represent empty directories, but an
/// obligation that discovers directories can still see one.
fn ls_tree_dirs(git: &Git, tree: &str) -> Result<BTreeSet<String>, String> {
    let listing = git.run(&["ls-tree", "-r", "-d", "-z", tree])?;
    if listing.status != 0 {
        return Err(format!(
            "cannot list the directories of tree {tree}: {}",
            listing.stderr.trim()
        ));
    }
    parse_tree_dirs(tree, &listing.stdout_bytes)
}

/// The parsing half of [`ls_tree_dirs`], separated for the same reason as
/// [`parse_tree_entries`]: crafted bytes can pin every refusal directly.
fn parse_tree_dirs(tree: &str, listing: &[u8]) -> Result<BTreeSet<String>, String> {
    let mut dirs = BTreeSet::new();
    for entry in listing.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else {
            return Err(format!("tree {tree}: unparsable ls-tree -d entry"));
        };
        let path = std::str::from_utf8(&entry[tab + 1..]).map_err(|_| {
            format!(
                "tree {tree} holds a directory path that is not valid UTF-8: this platform cannot represent it faithfully"
            )
        })?;
        dirs.insert(path.replace('\\', "/"));
    }
    Ok(dirs)
}

/// On a host that carries the executable bit, the checkout must carry the tree's
/// mode: a hook that chmods a file (or a filesystem that cannot represent the
/// mode) changes what an obligation *executes*, without touching a byte the
/// content comparison can see. Windows does not represent the bit and is not
/// checked; that limit is documented.
fn verify_checkout_modes(
    scratch: &Path,
    tree: &str,
    expected: &[(String, String, String, u64)],
) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (mode, _, path, _) in expected {
            if mode != "100644" && mode != "100755" {
                continue;
            }
            let full = scratch.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
            let meta = fs::symlink_metadata(&full)
                .map_err(|e| format!("cannot stat {path:?} after checkout: {e}"))?;
            let executable = meta.permissions().mode() & 0o111 != 0;
            if executable != (mode == "100755") {
                return Err(format!(
                    "tree {tree} entry {path:?} declares mode {mode} but is {}: a hook or a filesystem that cannot carry the executable bit changed it, so the obligations would not run on the verified tree",
                    if executable { "executable" } else { "non-executable" }
                ));
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (scratch, tree, expected);
    }
    Ok(())
}

/// Prove the declared content did not change while the obligations ran.
///
/// The fidelity check happens *before* the first obligation. A `post-checkout`
/// hook can spawn a writer that changes a file later, and an obligation can
/// rewrite its own input; both would make the receipt claim that the tree passed
/// while the obligations saw something else. Re-running the same byte and mode
/// proof after the last obligation closes that window for a declared file.
///
/// Extra files are **allowed** here: a legitimate obligation writes build outputs
/// (`target/`, `node_modules/`). What is not detectable without a snapshot is a
/// write that is reverted before this re-check, or one that happens after it; the
/// limit is documented and `doctor` states it.
fn verify_checkout_unchanged(git: &Git, scratch: &Path, tree: &str) -> Result<(), String> {
    let expected = ls_tree_entries(git, tree)?;
    verify_checkout_modes(scratch, tree, &expected)?;
    verify_checkout_content(git, scratch, tree, &expected, "while the obligations ran")
}

/// Prove that the checkout holds the bytes the tree stores.
///
/// `git worktree add` applies the repository's content filters
/// (`core.autocrlf`, `core.eol`, `.gitattributes` — committed **or local**
/// (`.git/info/attributes`, `core.attributesFile`) — and any driver those
/// attributes name). A filter can rewrite content **without changing its
/// length**, so a size comparison is not a proof; and a "no conversion looks
/// possible" guess is not one either, because the attribute sources include
/// host-local files Flow cannot enumerate exhaustively. Every regular file is
/// therefore compared, byte for byte, with the blob the tree names, read through
/// one bounded `git cat-file --batch`.
///
/// Above the entry or byte budget the comparison is **refused**, never skipped:
/// a tree Flow cannot prove is a tree Flow does not certify. What this cannot
/// detect is a filter whose output is byte-identical to the blob: then there is
/// nothing to detect.
///
/// `stage` names *when* the comparison runs (`during checkout` or `while the
/// obligations ran`) so a refusal says what changed and when.
fn verify_checkout_content(
    git: &Git,
    scratch: &Path,
    tree: &str,
    expected: &[(String, String, String, u64)],
    stage: &str,
) -> Result<(), String> {
    /// Beyond this entry count the comparison is refused, not skipped.
    const CONTENT_ENTRIES: usize = 20_000;

    let regular: Vec<(&str, &str)> = expected
        .iter()
        .filter(|(mode, oid, _, _)| (mode == "100644" || mode == "100755") && !oid.is_empty())
        .map(|(_, oid, path, _)| (oid.as_str(), path.as_str()))
        .collect();
    if regular.is_empty() {
        return Ok(());
    }
    if regular.len() > CONTENT_ENTRIES {
        return Err(format!(
            "tree {tree} has {} regular files: above the {CONTENT_ENTRIES}-entry fidelity budget, so it cannot be certified",
            regular.len()
        ));
    }
    let mut request = String::new();
    for (oid, _) in &regular {
        request.push_str(oid);
        request.push('\n');
    }
    let out = git.run_bounded(&["cat-file", "--batch"], Some(request.as_bytes()), None)?;
    if out.status != 0 {
        return Err(format!(
            "cannot read the tree content for {tree}: {}",
            out.stderr.trim()
        ));
    }
    let blobs = parse_cat_file_batch(tree, &regular, &out.stdout_bytes)?;
    for ((oid, path), blob) in regular.iter().zip(blobs) {
        let full = scratch.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        let disk = fs::read(&full).map_err(|e| format!("cannot read {path:?} {stage}: {e}"))?;
        if disk != blob {
            return Err(format!(
                "tree {tree} entry {path:?} changed {stage} ({} on disk vs {} in the tree, blob {}): a content filter (core.autocrlf, core.eol, a committed or local .gitattributes/.git/info/attributes, core.attributesFile or a driver), a hook or the obligation itself rewrote it, so the obligations would not see — or would not have seen — the verified tree",
                disk.len(),
                blob.len(),
                short(oid)
            ));
        }
    }
    Ok(())
}

/// Parse one bounded `git cat-file --batch` answer: exactly one blob per
/// requested oid, in request order.
///
/// Git's machine output is treated as untrusted input here: a missing newline, a
/// non-blob object, an unparsable size, a truncated body, an entry count or byte
/// total above the fidelity budget, or an arithmetic overflow is **refused**, not
/// guessed. Liveness is the point: every one of those is reachable from a
/// corrupted object store or a Git that answers nonsense, and none of them may
/// become a panic (a panic is not a refusal and could bypass cleanup).
fn parse_cat_file_batch<'e, 'b>(
    tree: &str,
    regular: &[(&'e str, &'e str)],
    bytes: &'b [u8],
) -> Result<Vec<&'b [u8]>, String> {
    /// Retained bytes for the whole comparison, before refusing.
    const CONTENT_BUDGET: usize = 64 * 1024 * 1024;

    let mut blobs: Vec<&'b [u8]> = Vec::with_capacity(regular.len());
    let mut position = 0usize;
    let mut total = 0usize;
    for (oid, path) in regular {
        let rest = bytes
            .get(position..)
            .ok_or_else(|| format!("truncated cat-file batch before {path:?}"))?;
        let Some(offset) = rest.iter().position(|b| *b == b'\n') else {
            return Err(format!("truncated cat-file batch before {path:?}"));
        };
        let header_end = position + offset;
        let header = String::from_utf8_lossy(&bytes[position..header_end]).to_string();
        let mut fields = header.split(' ');
        let _echo = fields.next();
        let kind = fields.next().unwrap_or("");
        if kind != "blob" {
            return Err(format!(
                "tree {tree} entry {path:?} resolved to {kind:?}, not a blob (blob {oid})"
            ));
        }
        let size: usize = fields
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("unparsable cat-file header {header:?}"))?;
        let start = header_end + 1;
        let end = start
            .checked_add(size)
            .ok_or_else(|| format!("cat-file header {header:?} overflows a size"))?;
        let blob = bytes
            .get(start..end)
            .ok_or_else(|| format!("truncated cat-file batch in {path:?}"))?;
        total += size;
        if total > CONTENT_BUDGET {
            return Err(format!(
                "tree {tree} exceeds the {CONTENT_BUDGET}-byte content-fidelity budget: refusing to certify a tree Flow cannot compare"
            ));
        }
        blobs.push(blob);
        position = end + 1;
    }
    Ok(blobs)
}
fn collect_entries(
    root: &Path,
    dir: &Path,
    files: &mut Vec<String>,
    dirs: &mut Vec<String>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        // Only Flow's own scratch ownership marker is not part of the verified
        // tree. The `.git` link is removed by `materialise` before this check, so a
        // `.git` entry here is a file the tree did not declare — a hook that
        // recreates it after the unlink must be refused, not ignored. Any other
        // root entry, including one whose name merely starts with `.flow-scratch`,
        // is an extra file.
        if dir == root && name == ".flow-scratch-owned" {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() && !meta.file_type().is_symlink() {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| format!("{} escaped the checkout", path.display()))?;
            dirs.push(rel.to_string_lossy().replace('\\', "/"));
            collect_entries(root, &path, files, dirs)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| format!("{} escaped the checkout", path.display()))?;
            files.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}
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
        None => git.write_tree()?,
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
    let context_id = build_context_id(&git, &tree, &policy);
    let log_dir = state::logs_dir(root).join(&tree);
    let mut outcomes = Vec::new();
    let mut logs = Vec::new();
    let mut failed = false;
    let mut timed_out = false;
    let mut unsupported = false;
    for ob in &selected {
        let outcome = gitcli::run_obligation(ob, scratch.path());
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
        None => git.write_tree()?,
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
    let expected_context = build_context_id(&git, &tree, &policy);
    if receipt.context_id != expected_context {
        return Ok(refused_code(
            "CONTEXT_MISMATCH",
            format!(
                "the receipt context is {} but this environment now computes {}: the toolchain or platform changed since the run",
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
    // 5. the verification commit must still describe the same tree, when the
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
                Json::str(state::journal_path(root).to_string_lossy().to_string()),
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
fn lock_refusal(root: &Path, err: crate::lock::LockError) -> Json {
    match err {
        crate::lock::LockError::Held(why) => refused_code("LOCKED", why),
        crate::lock::LockError::Unknown(why) => {
            let mut value = refused_code("LOCK_UNKNOWN", why);
            if let Json::Obj(pairs) = &mut value {
                pairs.push(("manual_required".into(), Json::Bool(true)));
                pairs.push((
                    "lock_path".into(),
                    Json::str(crate::lock::lock_path(root).to_string_lossy().to_string()),
                ));
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
fn journal_failure(root: &Path, err: state::JournalError) -> Json {
    let (status, code) = match &err {
        state::JournalError::Io(_) => (state::STATE_UNKNOWN_OUTCOME, "JOURNAL_UNREADABLE"),
        state::JournalError::Corrupt(_) => (state::STATE_CORRUPT_JOURNAL, "JOURNAL_CORRUPT"),
    };
    Json::obj(vec![
        ("status", Json::str(status)),
        ("code", Json::str(code)),
        ("error", Json::str(err.to_string())),
        (
            "journal_path",
            Json::str(state::journal_path(root).to_string_lossy().to_string()),
        ),
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

// ----------------------------------------------------------------- explain
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
    let (receipt_json, receipt_trust, receipt_display, evidence_complete, missing_logs) =
        match &receipt {
            None => (
                Json::Null,
                "NO_RECEIPT",
                Json::Null,
                false,
                Vec::<String>::new(),
            ),
            Some(r) => {
                let integrity_ok = state::verify_receipt_integrity(&r.json).is_ok();
                let missing: Vec<String> = r
                    .logs
                    .iter()
                    .filter(|rel| !root.join(rel.as_str()).is_file())
                    .cloned()
                    .collect();
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
                    integrity_ok && missing.is_empty(),
                    missing,
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
        ("receipt_status_display", receipt_display),
        ("evidence_complete", Json::Bool(evidence_complete)),
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
        // The budget is 64 MiB, not `64 + 1024 * 1024` and not `64 * 1024 + 1024`:
        // an ordinary batch of two 600 KiB blobs must be accepted, so either
        // collapsed constant turns this into a refusal.
        let wanted = regular(&[("aaaa", "a.bin"), ("bbbb", "b.bin")]);
        let blob = vec![b'x'; 600 * 1024];
        let mut bytes = Vec::new();
        for (oid, _) in &wanted {
            bytes.extend_from_slice(oid.as_bytes());
            bytes.extend_from_slice(format!(" blob {}\n", blob.len()).as_bytes());
            bytes.extend_from_slice(&blob);
            bytes.push(b'\n');
        }
        let blobs = parse_cat_file_batch("t", &wanted, &bytes).expect("100 KiB is ordinary");
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
        let reason = parse_cat_file_batch("t", &wanted, &bytes).expect_err("over budget");
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
        let entries: Vec<(String, String, String, u64)> = (0..20_001)
            .map(|i| ("100644".to_string(), "aaaa".to_string(), format!("f{i}"), 0))
            .collect();
        let reason = verify_checkout_content(&git, &scratch, "t", &entries, "during checkout")
            .expect_err("above the entry budget");
        assert!(
            reason.contains("above the 20000-entry fidelity budget"),
            "{reason}"
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
    fn cat_file_batch_accepts_a_total_of_exactly_the_content_budget() {
        // 64 MiB is the documented ceiling for the retained comparison. A total
        // exactly at the ceiling must be compared, not refused: `>` and `>=` (or
        // `==`) differ only here, and no ordinary fixture reaches it.
        const CHUNK: usize = 32 * 1024 * 1024;
        let wanted = regular(&[("aaaa", "a.bin"), ("bbbb", "b.bin")]);
        let mut bytes = Vec::with_capacity(2 * (CHUNK + 32));
        for (oid, _) in &wanted {
            bytes.extend_from_slice(oid.as_bytes());
            bytes.extend_from_slice(format!(" blob {CHUNK}\n").as_bytes());
            bytes.extend(std::iter::repeat_n(b'x', CHUNK));
            bytes.push(b'\n');
        }
        let blobs = parse_cat_file_batch("t", &wanted, &bytes)
            .expect("exactly 64 MiB is the ceiling, not a refusal");
        assert_eq!(blobs.len(), 2);
        assert_eq!(blobs[0].len(), CHUNK);
    }

    #[test]
    fn a_checkout_with_exactly_the_entry_budget_is_compared_not_refused() {
        // 20 000 regular files is the documented ceiling of the byte comparison.
        // The references all name one real blob, so the accepted answer also
        // proves the comparison still reads and matches the checkout.
        const ENTRIES: usize = 20_000;
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
        let expected: Vec<(String, String, String, u64)> = (0..ENTRIES)
            .map(|_| ("100644".to_string(), oid.clone(), "a.txt".to_string(), 6))
            .collect();
        verify_checkout_content(
            &git,
            &root.join("scratch"),
            "t",
            &expected,
            "during checkout",
        )
        .expect("exactly 20 000 regular files is the ceiling, not a refusal");
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
