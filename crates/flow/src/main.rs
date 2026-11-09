//! Ravum Flow — minimal verified-tree receipt CLI.
//!
//! Design constraints encoded here:
//! * no dependency, no daemon, no network, no LLM;
//! * the receipt names the exact Git tree OID that was checked;
//! * a publication is ref-CAS + a durable intent/outcome journal;
//! * anything not executed is `UNSUPPORTED`, `UNKNOWN` or `NOT_RUN` — never `PASSED`.

mod commands;
mod gitcli;
mod json;
mod paths;
mod sha256;
mod state;

use std::path::PathBuf;
use std::process::ExitCode;

use json::Json;

const USAGE: &str = "\
flow — verified-tree receipts for Git repositories (local, no daemon)

USAGE:
  flow <command> [options]

COMMANDS:
  init [--force]                    create .flow/ (config, journal, receipts)
  doctor                            read-only environment and guarantee report
  status                            repository, dirty state, latest receipt
  run [--obligation ID]... [--timeout S]
                                    verify the INDEX tree; writes a receipt
  publish [--ref REF] [--expected-old OID] [--message M]
                                    publish a PASSED receipt with ref CAS
  reconcile                         resolve pending intents (crash recovery)
  explain [--tree OID]              receipt + journal for a tree
  disable | enable                  refuse / allow mutating commands
  version                           tool and policy versions
  help                              this text

Every command prints machine-readable JSON on stdout.
Exit codes: 0 = PASSED/PUBLISHED/REPORT, 1 = REFUSED/FAILED/UNKNOWN/UNSUPPORTED, 2 = usage.
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
                "init" => commands::cmd_init(&root, rest.iter().any(|a| a == "--force")),
                "doctor" => commands::cmd_doctor(&root),
                "status" => commands::cmd_status(&root),
                "run" => {
                    let mut obligations = Vec::new();
                    let mut index = 0;
                    while index < rest.len() {
                        match rest[index].as_str() {
                            "--obligation" | "--obligations" => {
                                if let Some(value) = rest.get(index + 1) {
                                    obligations.push(value.clone());
                                    index += 2;
                                } else {
                                    index += 1;
                                }
                            }
                            other => {
                                if let Some(value) = other.strip_prefix("--obligation=") {
                                    obligations.push(value.to_string());
                                }
                                index += 1;
                            }
                        }
                    }
                    let timeout = flag_value(rest, "--timeout").and_then(|v| v.parse::<u64>().ok());
                    let only = if obligations.is_empty() {
                        None
                    } else {
                        Some(obligations)
                    };
                    commands::cmd_run(&root, only, timeout)
                }
                "publish" => commands::cmd_publish(
                    &root,
                    flag_value(rest, "--ref"),
                    flag_value(rest, "--expected-old"),
                    flag_value(rest, "--message"),
                ),
                "reconcile" => commands::cmd_reconcile(&root),
                "explain" => commands::cmd_explain(&root, flag_value(rest, "--tree")),
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

fn exit_code_for(value: &Json) -> u8 {
    match value.get("status").and_then(Json::as_str) {
        Some("PASSED") | Some("PUBLISHED") | Some("REPORT") | Some("INITIALIZED")
        | Some("RECONCILED") | Some("EXPLAIN") | Some("DISABLED") | Some("ENABLED") => 0,
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
