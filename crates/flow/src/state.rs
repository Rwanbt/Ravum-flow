//! Durable state: config, journal (append + fsync), receipts.
//!
//! Guarantees stated by this module, and only these:
//! * an intent appended and fsynced is not forgotten;
//! * an outcome is appended after the ref update attempt;
//! * a torn journal line is refused (`InvalidData`), never silently skipped.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::gitcli::Obligation;
use crate::json::{self, Json};

pub const POLICY_VERSION: &str = "FLOW-POLICY-1";
pub const SCHEMA_CONFIG: &str = "flow-config/1";
pub const SCHEMA_RECEIPT: &str = "flow-receipt/1";
pub const SCHEMA_JOURNAL: &str = "flow-journal/1";

pub struct Config {
    pub policy_version: String,
    pub execution_boundary: String,
    pub obligations: Vec<Obligation>,
    pub disabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            policy_version: POLICY_VERSION.to_string(),
            execution_boundary: "local-trusted".to_string(),
            obligations: Vec::new(),
            disabled: false,
        }
    }
}

impl Config {
    pub fn to_json(&self) -> Json {
        let obs = self
            .obligations
            .iter()
            .map(|o| {
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
            })
            .collect();
        Json::obj(vec![
            ("schema", Json::str(SCHEMA_CONFIG)),
            ("policy_version", Json::str(self.policy_version.clone())),
            (
                "execution_boundary",
                Json::str(self.execution_boundary.clone()),
            ),
            ("obligations", Json::Arr(obs)),
            ("disabled", Json::Bool(self.disabled)),
        ])
    }

    pub fn from_json(value: &Json) -> Result<Config, String> {
        let schema = value.get("schema").and_then(Json::as_str).unwrap_or("");
        if schema != SCHEMA_CONFIG {
            return Err(format!("unsupported config schema {schema:?}"));
        }
        let mut obligations = Vec::new();
        if let Some(items) = value.get("obligations").and_then(Json::as_arr) {
            for item in items {
                let id = item
                    .get("id")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string();
                let program = item
                    .get("program")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string();
                if id.is_empty() || program.is_empty() {
                    return Err("obligation needs a non-empty id and program".into());
                }
                let args = item
                    .get("args")
                    .and_then(Json::as_arr)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                obligations.push(Obligation {
                    id,
                    program,
                    args,
                    required: item.get("required").and_then(Json::as_bool).unwrap_or(true),
                    timeout_s: item
                        .get("timeout_s")
                        .and_then(Json::as_num)
                        .unwrap_or(300.0) as u64,
                });
            }
        }
        Ok(Config {
            policy_version: value
                .get("policy_version")
                .and_then(Json::as_str)
                .unwrap_or(POLICY_VERSION)
                .to_string(),
            execution_boundary: value
                .get("execution_boundary")
                .and_then(Json::as_str)
                .unwrap_or("local-trusted")
                .to_string(),
            obligations,
            disabled: value
                .get("disabled")
                .and_then(Json::as_bool)
                .unwrap_or(false),
        })
    }
}

pub fn flow_dir(root: &Path) -> PathBuf {
    root.join(".flow")
}

pub fn config_path(root: &Path) -> PathBuf {
    flow_dir(root).join("config.json")
}

pub fn journal_path(root: &Path) -> PathBuf {
    flow_dir(root).join("journal.jsonl")
}

pub fn receipts_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("receipts")
}

pub fn logs_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("logs")
}

pub fn receipt_path(root: &Path, tree_oid: &str) -> PathBuf {
    receipts_dir(root).join(format!("{tree_oid}.json"))
}

pub fn load_config(root: &Path) -> Result<Config, String> {
    let path = config_path(root);
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("not initialized: cannot read {}: {e}", path.display()))?;
    Config::from_json(&json::parse(&text)?)
}

pub fn save_config(root: &Path, config: &Config) -> Result<(), String> {
    let path = config_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create .flow: {e}"))?;
    }
    write_atomic(&path, &(config.to_json().to_string_pretty() + "\n"))
}

pub fn write_atomic(path: &Path, data: &str) -> Result<(), String> {
    let tmp = path.with_extension("tmp-flow");
    {
        let mut file =
            File::create(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
        file.write_all(data.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        file.sync_all()
            .map_err(|e| format!("cannot fsync {}: {e}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("cannot rename into {}: {e}", path.display()))
}

pub fn append_journal(root: &Path, record: &Json) -> Result<(), String> {
    // Every record carries its schema so a future reader can refuse a mismatch.
    let mut tagged = record.clone();
    if let Json::Obj(pairs) = &mut tagged {
        if !pairs.iter().any(|(k, _)| k == "schema") {
            pairs.insert(0, ("schema".to_string(), Json::str(SCHEMA_JOURNAL)));
        }
    }
    let record = &tagged;
    let path = journal_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create .flow: {e}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open journal: {e}"))?;
    let mut line = record.to_string();
    line.push('\n');
    file.write_all(line.as_bytes())
        .map_err(|e| format!("cannot append journal: {e}"))?;
    // fsync the file and the directory (best effort on Windows).
    file.sync_all()
        .map_err(|e| format!("cannot fsync journal: {e}"))?;
    #[cfg(not(windows))]
    if let Some(parent) = path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

pub fn read_journal(root: &Path) -> Result<Vec<Json>, String> {
    let path = journal_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(&path).map_err(|e| format!("cannot open journal: {e}"))?;
    let mut records = Vec::new();
    for (lineno, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| format!("cannot read journal line {}: {e}", lineno + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let record = json::parse(&line)
            .map_err(|e| format!("torn or invalid journal line {}: {e}", lineno + 1))?;
        records.push(record);
    }
    Ok(records)
}

pub struct Receipt {
    pub tree_oid: String,
    pub commit_oid: String,
    pub context_id: String,
    pub policy_version: String,
    pub execution_boundary: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub obligations: Vec<Json>,
    pub logs: Vec<String>,
    pub worktree_diff: Vec<String>,
    pub json: Json,
}

impl Receipt {
    pub fn load(root: &Path, tree_oid: &str) -> Result<Option<Receipt>, String> {
        let path = receipt_path(root, tree_oid);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|e| format!("cannot read receipt: {e}"))?;
        let value = json::parse(&text)?;
        let get = |key: &str| {
            value
                .get(key)
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string()
        };
        Ok(Some(Receipt {
            tree_oid: get("tree_oid"),
            commit_oid: get("commit_oid"),
            context_id: get("context_id"),
            policy_version: get("policy_version"),
            execution_boundary: get("execution_boundary"),
            status: get("status"),
            started_at: get("started_at"),
            finished_at: get("finished_at"),
            obligations: value
                .get("obligations")
                .and_then(Json::as_arr)
                .map(<[Json]>::to_vec)
                .unwrap_or_default(),
            logs: string_list(&value, "logs"),
            worktree_diff: string_list(&value, "worktree_diff"),
            json: value,
        }))
    }
}

fn string_list(value: &Json, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn save_receipt(root: &Path, receipt: &Json) -> Result<PathBuf, String> {
    let tree = receipt
        .get("tree_oid")
        .and_then(Json::as_str)
        .ok_or("receipt without tree_oid")?;
    let path = receipt_path(root, tree);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create receipts dir: {e}"))?;
    }
    write_atomic(&path, &(receipt.to_string_pretty() + "\n"))?;
    Ok(path)
}

/// Civil date/time from the Unix epoch, UTC, without a dependency.
pub fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Howard Hinnant's algorithm, adapted from days since 1970-01-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
    }

    #[test]
    fn config_round_trip() {
        let config = Config {
            obligations: vec![Obligation {
                id: "t".into(),
                program: "cargo".into(),
                args: vec!["test".into()],
                required: true,
                timeout_s: 30,
            }],
            ..Config::default()
        };
        let parsed = Config::from_json(&config.to_json()).unwrap();
        assert_eq!(parsed.obligations.len(), 1);
        assert_eq!(parsed.obligations[0].program, "cargo");
        assert_eq!(parsed.policy_version, POLICY_VERSION);
    }

    #[test]
    fn rejects_unknown_schema() {
        let mut value = Config::default().to_json();
        if let Json::Obj(pairs) = &mut value {
            pairs.retain(|(k, _)| k != "schema");
        }
        assert!(Config::from_json(&value).is_err());
    }
}
