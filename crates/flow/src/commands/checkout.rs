//! Exact certification of a materialised checkout.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::short;
use crate::gitcli::Git;
use crate::state;

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
pub(crate) fn verify_checkout_fidelity(
    git: &Git,
    scratch: &Path,
    tree: &str,
) -> Result<(), String> {
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
pub(crate) fn ls_tree_entries(
    git: &Git,
    tree: &str,
) -> Result<Vec<(String, String, String, u64)>, String> {
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
pub(crate) fn parse_tree_entries(
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
pub(crate) fn ls_tree_dirs(git: &Git, tree: &str) -> Result<BTreeSet<String>, String> {
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
pub(crate) fn parse_tree_dirs(tree: &str, listing: &[u8]) -> Result<BTreeSet<String>, String> {
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
pub(crate) fn verify_checkout_modes(
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
pub(crate) fn verify_checkout_unchanged(
    git: &Git,
    scratch: &Path,
    tree: &str,
) -> Result<(), String> {
    let expected = ls_tree_entries(git, tree)?;
    verify_checkout_modes(scratch, tree, &expected)?;
    verify_checkout_content(git, scratch, tree, &expected, "while the obligations ran")
}

/// The budgets that bound the byte comparison.
///
/// They are a value rather than three scattered constants so each boundary can
/// be pinned with a tiny fixture instead of a 20 000-file checkout, and so the
/// documented envelope is one constant instead of a sentence.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ContentBudget {
    /// Regular files compared in one run. Beyond this the run refuses.
    pub(crate) entries: usize,
    /// Declared bytes packed into one `git cat-file --batch` request. The
    /// comparison is streamed chunk by chunk, so the total content of a
    /// repository is no longer a memory bound.
    pub(crate) chunk_bytes: u64,
    /// One blob larger than this cannot be carried by the bounded Git capture,
    /// so a tree that declares one is refused by the file's name.
    pub(crate) single_blob_bytes: u64,
}

/// A real monorepo fits: 200 000 files, 32 MiB of declared content per request,
/// and at most one 48 MiB file per request (the Git capture is capped at 64 MiB).
/// Every refusal still happens *before* an obligation runs.
pub(crate) const CONTENT_BUDGET: ContentBudget = ContentBudget {
    entries: 200_000,
    chunk_bytes: 32 * 1024 * 1024,
    single_blob_bytes: 48 * 1024 * 1024,
};

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
/// bounded `git cat-file --batch` requests.
///
/// Above the entry, chunk or per-file budget the comparison is **refused**, never
/// skipped: a tree Flow cannot prove is a tree Flow does not certify. What this
/// cannot detect is a filter whose output is byte-identical to the blob: then
/// there is nothing to detect.
///
/// `stage` names *when* the comparison runs (`during checkout` or `while the
/// obligations ran`) so a refusal says what changed and when.
pub(crate) fn verify_checkout_content(
    git: &Git,
    scratch: &Path,
    tree: &str,
    expected: &[(String, String, String, u64)],
    stage: &str,
) -> Result<(), String> {
    verify_checkout_content_with(git, scratch, tree, expected, stage, CONTENT_BUDGET)
}

pub(crate) fn verify_checkout_content_with(
    git: &Git,
    scratch: &Path,
    tree: &str,
    expected: &[(String, String, String, u64)],
    stage: &str,
    budget: ContentBudget,
) -> Result<(), String> {
    let regular: Vec<(&str, &str, u64)> = expected
        .iter()
        .filter(|(mode, oid, _, _)| (mode == "100644" || mode == "100755") && !oid.is_empty())
        .map(|(_, oid, path, size)| (oid.as_str(), path.as_str(), *size))
        .collect();
    if regular.is_empty() {
        return Ok(());
    }
    if regular.len() > budget.entries {
        return Err(format!(
            "tree {tree} has {} regular files: above the {}-entry fidelity budget, so it cannot be certified",
            regular.len(),
            budget.entries
        ));
    }
    // The comparison is streamed one request at a time, so the total content of
    // the repository is bounded by the disk, not by memory: a repository of up to
    // `budget.entries` files is compared with at most `budget.chunk_bytes` held in
    // memory per request. Nothing is skipped when a chunk is refused; the run
    // stops with the reason.
    let mut start = 0usize;
    while start < regular.len() {
        let mut end = start;
        let mut chunk_bytes: u64 = 0;
        while end < regular.len() {
            let (_, path, size) = regular[end];
            if size > budget.single_blob_bytes {
                return Err(format!(
                    "tree {tree} entry {path:?} declares {size} bytes, above the {}-byte single-file comparison budget: the bounded Git capture cannot read it, so the tree cannot be certified",
                    budget.single_blob_bytes
                ));
            }
            if end > start && chunk_bytes.saturating_add(size) > budget.chunk_bytes {
                break;
            }
            chunk_bytes = chunk_bytes.saturating_add(size);
            end += 1;
        }
        let chunk = &regular[start..end];
        let pairs: Vec<(&str, &str)> = chunk.iter().map(|(oid, path, _)| (*oid, *path)).collect();
        let mut request = String::new();
        for (oid, _, _) in chunk {
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
        // A chunk can hold `chunk_bytes` of small files, or one file up to the
        // per-file budget (the packer never puts a second file after it), so the
        // ceiling of one answer is the larger of the two.
        let ceiling = budget.chunk_bytes.max(budget.single_blob_bytes);
        let blobs = parse_cat_file_batch_with_budget(tree, &pairs, &out.stdout_bytes, ceiling)?;
        for ((oid, path, _), blob) in chunk.iter().zip(blobs) {
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
        start = end;
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
/// Test-only convenience: the default chunk budget, so a boundary test that does
/// not care about the byte ceiling still exercises the real default.
#[cfg(test)]
pub(crate) fn parse_cat_file_batch<'e, 'b>(
    tree: &str,
    regular: &[(&'e str, &'e str)],
    bytes: &'b [u8],
) -> Result<Vec<&'b [u8]>, String> {
    parse_cat_file_batch_with_budget(tree, regular, bytes, CONTENT_BUDGET.chunk_bytes)
}

/// The budget is a parameter so the boundary can be pinned with a few bytes
/// instead of a 32 MiB answer; the default is the documented chunk budget.
pub(crate) fn parse_cat_file_batch_with_budget<'e, 'b>(
    tree: &str,
    regular: &[(&'e str, &'e str)],
    bytes: &'b [u8],
    budget_bytes: u64,
) -> Result<Vec<&'b [u8]>, String> {
    let mut blobs: Vec<&'b [u8]> = Vec::with_capacity(regular.len());
    let mut position = 0usize;
    let mut total: u64 = 0;
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
        total += size as u64;
        if total > budget_bytes {
            return Err(format!(
                "tree {tree} exceeds the {budget_bytes}-byte content-fidelity budget: refusing to certify a tree Flow cannot compare"
            ));
        }
        blobs.push(blob);
        position = end + 1;
    }
    Ok(blobs)
}
pub(crate) fn collect_entries(
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
/// Compute the index tree **without** taking the shared `.git/index` lock.
///
/// `git write-tree` may refresh the index cache-tree and therefore takes
/// `index.lock`; two concurrent `run`s on the same working copy would make one
/// of them fail with "Another git process seems to be running". A byte copy of
/// the index passed through `GIT_INDEX_FILE` gives each run its own view, and
/// the user's index is never written. The copy is read while the user may be
/// editing; that is the same snapshot rule as any Git read.
pub(crate) fn index_tree_without_locking(git: &Git) -> Result<String, String> {
    let located = git.run_bounded(&["rev-parse", "--git-path", "index"], None, None)?;
    if located.status != 0 {
        return Err(format!(
            "cannot locate the index: {}",
            located.stderr.trim()
        ));
    }
    let raw = PathBuf::from(located.stdout.trim());
    let source = if raw.is_absolute() {
        raw
    } else {
        git.root.join(raw)
    };
    let copy = std::env::temp_dir().join(format!(
        "flow-run-index-{}-{:x}",
        std::process::id(),
        state::unique_suffix()
    ));
    if source.is_file() {
        fs::copy(&source, &copy).map_err(|e| {
            format!(
                "cannot copy the index {} for a lock-free tree read: {e}",
                source.display()
            )
        })?;
    }
    let written = git.run_bounded(&["write-tree"], None, Some(&copy))?;
    let _ = fs::remove_file(&copy);
    if written.status != 0 {
        return Err(format!(
            "cannot write the index tree: {}",
            written.stderr.trim()
        ));
    }
    Ok(written.stdout.trim().to_string())
}
