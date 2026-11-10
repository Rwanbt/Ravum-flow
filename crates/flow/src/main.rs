//! Ravum Flow — verified-tree receipts and exact-tree assembly for Git.
//!
//! Design constraints encoded here:
//! * no dependency, no daemon, no network, no LLM;
//! * a receipt names the exact Git tree OID that was checked;
//! * a publication is ref-CAS + a durable intent/outcome journal;
//! * anything not executed is `UNSUPPORTED`, `UNKNOWN` or `NOT_RUN` — never `PASSED`.
//!
//! The CLI prints machine-readable JSON for every command and exits 0 only for a
//! status that means "the requested thing happened" (`PASSED`, `PUBLISHED`,
//! `PLAN`, `INITIALIZED`, `APPROVED`, `ADMITTED`, `ASSEMBLED`, `RECORD`,
//! `RECONCILED`, `EXPLAIN`, `STATUS`, `DISABLED`, `ENABLED`). Every refusal,
//! failure and unknown outcome exits 1. A usage error exits 2.

mod commands;
mod gitcli;
mod json;
mod lock;
mod paths;
mod proposal;
mod scratch;
mod sha256;
mod state;

use std::path::PathBuf;
use std::process::ExitCode;

use json::Json;

const USAGE: &str = "\
flow — verified-tree receipts and exact-tree assembly for Git repositories
       (local, offline, no daemon, no LLM)

USAGE:
  flow <command> [options]

SETUP:
  init [--force] [--adopt-policy]  create .flow/; adopt a repository policy only explicitly
  approve                           record local approval of the current .flow/config.json
  doctor                            read-only environment and guarantee report
  status                            repository, dirty state, receipt, pending intents
  disable | enable                  refuse / allow mutating commands

PROPOSALS (multi-agent):
  propose --agent A --task T1 --base <commit> --patch <file> [--key K]
                                    admit an attributed, immutable proposal
  proposals list                    list admitted proposals
  proposals show <id>               one proposal
  assemble --proposal <id>...       build an immutable candidate tree from proposals

VERIFICATION AND PUBLICATION:
  run [--candidate <id>] [--obligation ID]... [--timeout S]
                                    verify the candidate tree, or the INDEX tree
  publish [--candidate <id>] [--ref REF] [--expected-old OID] [--message M]
          [--dry-run]               CAS-publish a verified tree
  reconcile                         resolve pending intents by observation
  explain [--tree OID | --candidate <id>]
                                    receipt, proposals and journal for a tree

  version                           tool, policy and schema versions
  help                              this text

Every command prints machine-readable JSON on stdout. `--json` is accepted for
compatibility; JSON is already the default.
Exit codes: 0 = PASSED/PUBLISHED/PLAN/INITIALIZED/APPROVED/ADMITTED/ASSEMBLED/
RECORD/RECONCILED/EXPLAIN/STATUS/DISABLED/ENABLED, 1 = REFUSED/FAILED/UNKNOWN/
UNSUPPORTED/NEEDS_RECONCILIATION/ERROR, 2 = usage.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = dispatch(&args);
    ExitCode::from(code)
}

fn dispatch(args: &[String]) -> u8 {
    let Some(command) = args.first().map(String::as_str) else {
        print!("{USAGE}");
        return 2;
    };
    let rest = &args[1..];
    match command {
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            0
        }
        "version" | "--version" => {
            emit(Json::obj(vec![
                ("status", Json::str("VERSION")),
                (
                    "tool",
                    Json::str(format!("flow {}", env!("CARGO_PKG_VERSION"))),
                ),
                ("policy_version", Json::str(state::POLICY_VERSION)),
                ("git", Json::str(gitcli::Git::new(".").version())),
                ("schema", Json::str(state::SCHEMA_RECEIPT)),
                // Every document this build can emit, so a client that discovers
                // schemas from `version` learns the whole contract instead of
                // half of it.
                (
                    "schemas",
                    Json::str(
                        [
                            state::SCHEMA_CONFIG,
                            state::SCHEMA_RECEIPT,
                            state::SCHEMA_JOURNAL,
                            state::SCHEMA_PROPOSAL,
                            state::SCHEMA_CANDIDATE,
                            state::SCHEMA_ADOPTION,
                            state::SCHEMA_PLAN,
                            state::SCHEMA_STATUS,
                            state::SCHEMA_PROPOSAL_LIST,
                        ]
                        .join(", "),
                    ),
                ),
            ]));
            0
        }
        _ => {
            let root =
                match gitcli::toplevel(&std::env::current_dir().unwrap_or(PathBuf::from("."))) {
                    Ok(root) => root,
                    Err(err) => {
                        emit_error(&err);
                        return 1;
                    }
                };
            if let Err(reason) = validate_flags(command, rest) {
                emit_error(&reason);
                return 2;
            }
            let result = match command {
                "init" => commands::cmd_init(
                    &root,
                    has_flag(rest, "--force"),
                    has_flag(rest, "--adopt-policy"),
                ),
                "approve" => commands::cmd_approve(&root),
                "doctor" => commands::cmd_doctor(&root),
                "status" => commands::cmd_status(&root),
                "run" => {
                    // A timeout that is silently dropped would make the receipt
                    // prove a deadline that was never applied, so an unparseable
                    // or zero value is a usage error rather than a default.
                    let timeout = match flag_value(rest, "--timeout") {
                        None => None,
                        Some(raw) => match raw.parse::<u64>() {
                            Ok(seconds) if seconds >= 1 => Some(seconds),
                            _ => {
                                emit_error(&format!(
                                    "`--timeout` needs a whole number of seconds >= 1 (found {raw:?})"
                                ));
                                return 2;
                            }
                        },
                    };
                    commands::cmd_run(
                        &root,
                        collect_values(rest, &["--obligation", "--obligations"]),
                        timeout,
                        flag_value(rest, "--candidate"),
                    )
                }
                "publish" => commands::cmd_publish(
                    &root,
                    flag_value(rest, "--ref"),
                    flag_value(rest, "--expected-old"),
                    flag_value(rest, "--message"),
                    flag_value(rest, "--candidate"),
                    has_flag(rest, "--dry-run"),
                ),
                "propose" => commands::cmd_propose(
                    &root,
                    flag_value(rest, "--agent"),
                    flag_value(rest, "--task"),
                    flag_value(rest, "--base"),
                    flag_value(rest, "--patch"),
                    flag_value(rest, "--key"),
                ),
                "proposals" => {
                    let sub = rest.first().map(String::as_str).unwrap_or("list");
                    match sub {
                        "list" => commands::cmd_proposals(&root, None),
                        "show" => match rest.get(1) {
                            Some(id) => commands::cmd_proposals(&root, Some(id.clone())),
                            None => {
                                emit_error("`flow proposals show <id>` needs a proposal id");
                                return 2;
                            }
                        },
                        other => {
                            emit_error(&format!(
                                "unknown `proposals` subcommand {other:?} (list | show <id>)"
                            ));
                            return 2;
                        }
                    }
                }
                "assemble" => match collect_values(rest, &["--proposal"]) {
                    Some(ids) => commands::cmd_assemble(&root, &ids),
                    None => commands::cmd_assemble(&root, &[]),
                },
                "reconcile" => commands::cmd_reconcile(&root),
                "explain" => commands::cmd_explain(
                    &root,
                    flag_value(rest, "--tree"),
                    flag_value(rest, "--candidate"),
                ),
                "disable" => commands::cmd_set_disabled(&root, true),
                "enable" => commands::cmd_set_disabled(&root, false),
                other => {
                    emit_error(&format!("unknown command {other:?} (run `flow help`)"));
                    return 2;
                }
            };
            match result {
                Ok(value) => {
                    let mut code = exit_code_for(&value);
                    // A diagnostic that found a broken guarantee is not a success.
                    // The documented contract is `0 when overall=OK, 1 when
                    // overall=FAILED`, and it is tested.
                    if command == "doctor"
                        && value.get("overall").and_then(Json::as_str) == Some("FAILED")
                    {
                        code = 1;
                    }
                    emit(value);
                    code
                }
                Err(err) => {
                    emit_error(&err);
                    1
                }
            }
        }
    }
}

/// Flags that take a value, per command. An unknown flag, or a value flag
/// without a value, is a usage error: silently ignoring `--dry-run` on a program
/// that does not know it would publish when the operator asked for a plan.
fn value_flags(command: &str) -> &'static [&'static str] {
    match command {
        "run" => &["--obligation", "--obligations", "--timeout", "--candidate"],
        "publish" => &["--ref", "--expected-old", "--message", "--candidate"],
        "propose" => &["--agent", "--task", "--base", "--patch", "--key"],
        "assemble" => &["--proposal"],
        "explain" => &["--tree", "--candidate"],
        _ => &[],
    }
}

/// Boolean flags accepted by any command: output mode and help. Any other boolean
/// is per command, below.
const GLOBAL_BOOL_FLAGS: &[&str] = &["--json", "--help"];

/// Boolean flags that a command actually implements.
///
/// Accepting a boolean a command ignores is not harmless: `flow run --dry-run`
/// would execute the obligations and write a receipt for an operator who asked for
/// a plan. That is the same defect class as `publish --dry-run=true` (which the
/// second review found and this file already refuses), so an unimplemented
/// boolean is a usage error.
fn bool_flags(command: &str) -> &'static [&'static str] {
    match command {
        "init" => &["--force", "--adopt-policy"],
        "publish" => &["--dry-run"],
        _ => &[],
    }
}

/// Reject an unknown flag, a value flag without a value, or a stray positional
/// argument, before any mutation.
fn validate_flags(command: &str, args: &[String]) -> Result<(), String> {
    let takes_value = value_flags(command);
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            index += 1;
            continue;
        }
        if let Some(body) = arg.strip_prefix("--") {
            let (name, inline_value) = match body.split_once('=') {
                Some((head, value)) => (format!("--{head}"), Some(value)),
                None => (format!("--{body}"), None),
            };
            if takes_value.contains(&name.as_str()) {
                if inline_value.is_none() {
                    match args.get(index + 1) {
                        None => return Err(format!("`{name}` needs a value (flow {command} ...)")),
                        Some(value) if value.starts_with("--") => {
                            return Err(format!(
                                "`{name}` needs a value, found the flag {value:?} instead"
                            ))
                        }
                        Some(_) => {}
                    }
                    index += 2;
                    continue;
                }
            } else if GLOBAL_BOOL_FLAGS.contains(&name.as_str())
                || bool_flags(command).contains(&name.as_str())
            {
                // A boolean flag is a token, not a `key=value` pair. Accepting
                // `--dry-run=true` here and then matching only the exact token in
                // `has_flag` would run a real publication for an operator who
                // asked for a plan, so the shape is refused rather than guessed.
                if let Some(value) = inline_value {
                    return Err(format!(
                        "`{name}` takes no value (found {value:?}); write `{name}` on its own"
                    ));
                }
            } else if matches!(name.as_str(), "--dry-run" | "--force" | "--adopt-policy") {
                return Err(format!(
                    "`{name}` is not implemented by `flow {command}` (see `flow help`); an option that would be ignored is a usage error"
                ));
            } else {
                return Err(format!(
                    "unknown option {name:?} for `flow {command}` (see `flow help`)"
                ));
            }
        } else if command != "proposals" {
            return Err(format!(
                "unexpected argument {arg:?} for `flow {command}` (see `flow help`)"
            ));
        }
        index += 1;
    }
    Ok(())
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == name {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = args[index].strip_prefix(&format!("{name}=")) {
            return Some(value.to_string());
        }
        index += 1;
    }
    None
}

/// Collect every occurrence of any of `names`, in command-line order.
/// `--name value` and `--name=value` are both accepted; a repeated flag keeps
/// its order, which is the assembly order of a candidate.
fn collect_values(args: &[String], names: &[&str]) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if names.contains(&arg.as_str()) {
            if let Some(value) = args.get(index + 1) {
                out.push(value.clone());
            }
            index += 2;
            continue;
        }
        let mut matched = false;
        for name in names {
            if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
                out.push(value.to_string());
                matched = true;
                break;
            }
        }
        let _ = matched;
        index += 1;
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn exit_code_for(value: &Json) -> u8 {
    // A document that names USAGE is a usage error whatever its status: the
    // published contract reserves exit 2 for "you called me wrong", and a script
    // must be able to tell that apart from a policy refusal (exit 1).
    if value.get("code").and_then(Json::as_str) == Some("USAGE") {
        return 2;
    }
    match value.get("status").and_then(Json::as_str) {
        Some("PASSED") | Some("PUBLISHED") | Some("PLAN") | Some("INITIALIZED")
        | Some("APPROVED") | Some("ADMITTED") | Some("ASSEMBLED") | Some("RECORD")
        | Some("RECONCILED") | Some("EXPLAIN") | Some("STATUS") | Some("DISABLED")
        | Some("ENABLED") | Some("REPORT") => 0,
        Some(_) => 1,
        None => 1,
    }
}

fn emit(value: Json) {
    println!("{}", value.to_string_pretty());
}

fn emit_error(message: &str) {
    emit(Json::obj(vec![
        ("status", Json::str("ERROR")),
        ("error", Json::str(message)),
    ]));
}
