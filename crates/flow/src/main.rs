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
                ("schemas", Json::str("flow-config/1, flow-receipt/2, flow-journal/1, flow-proposal/1, flow-candidate/1, flow-adoption/1, flow-plan/1")),
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
            let result = match command {
                "init" => commands::cmd_init(
                    &root,
                    has_flag(rest, "--force"),
                    has_flag(rest, "--adopt-policy"),
                ),
                "approve" => commands::cmd_approve(&root),
                "doctor" => commands::cmd_doctor(&root),
                "status" => commands::cmd_status(&root),
                "run" => commands::cmd_run(
                    &root,
                    collect_values(rest, &["--obligation", "--obligations"]),
                    flag_value(rest, "--timeout").and_then(|v| v.parse::<u64>().ok()),
                    flag_value(rest, "--candidate"),
                ),
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
                    let code = exit_code_for(&value);
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
