//! Command implementation; shared helpers stay in the parent module.

use super::*;

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
