//! Path validation and canonicalisation (defence for PATH-08).
//!
//! The MVP never writes a path supplied by a remote proposal, but it does write
//! receipts/logs and must refuse a repository whose paths cannot be represented
//! faithfully (reserved names, trailing dots, case collisions are reported by
//! `doctor`, not silently normalised).

use std::path::{Path, PathBuf};

#[cfg(test)]
use std::path::Component;

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Validate a repository-relative path as written in a receipt or config.
/// True for a Git object id this build accepts: 40 or 64 hexadecimal characters.
///
/// Used wherever an identifier becomes a file name (`receipts/<tree>.json`): a
/// raw `../` or an absolute path must never reach the filesystem.
pub fn is_object_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// True for a Flow record id (proposal / candidate): 32 lowercase hex characters.
pub fn is_record_id(value: &str) -> bool {
    value.len() == 32 && value.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn validate_relative(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("empty path".into());
    }
    if path.len() > 4096 {
        return Err("path longer than 4096 bytes".into());
    }
    if path.bytes().any(|b| b < 0x20) {
        return Err("control character in path".into());
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("absolute path is not allowed".into());
    }
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err("drive-qualified path is not allowed".into());
    }
    for component in path.split(['/', '\\']) {
        if component.is_empty() {
            return Err("empty path component".into());
        }
        if component == "." || component == ".." {
            return Err(format!(
                "path traversal component {component:?} is not allowed"
            ));
        }
        if component.len() > 255 {
            return Err("path component longer than 255 bytes".into());
        }
        // Conservative portability rule: a colon is a drive separator or an NTFS
        // alternate-data-stream marker on Windows. Refusing it everywhere keeps
        // receipts portable across platforms instead of silently rewriting them.
        if component.contains(':') {
            return Err(format!("colon in component {component:?} is not portable"));
        }
        if component.ends_with('.') || component.ends_with(' ') {
            return Err(format!(
                "component {component:?} ends with a dot or space (not representable on Windows)"
            ));
        }
        let stem = component
            .split('.')
            .next()
            .unwrap_or(component)
            .to_ascii_uppercase();
        if RESERVED.contains(&stem.as_str()) {
            return Err(format!("reserved device name {component:?}"));
        }
    }
    Ok(())
}

/// Resolve `candidate` (which may not exist yet) and require it to stay under `root`.
pub fn canonical_within(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("cannot canonicalise root {}: {e}", root.display()))?;
    let mut probe = candidate.to_path_buf();
    let mut suffix: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if probe.exists() {
            let mut resolved = probe
                .canonicalize()
                .map_err(|e| format!("cannot canonicalise {}: {e}", probe.display()))?;
            for part in suffix.iter().rev() {
                resolved.push(part);
            }
            if !resolved.starts_with(&root) {
                return Err(format!(
                    "path {} escapes {} (symlink or junction)",
                    resolved.display(),
                    root.display()
                ));
            }
            return Ok(resolved);
        }
        match probe.file_name().map(|n| n.to_os_string()) {
            Some(name) => {
                suffix.push(name);
                probe = probe
                    .parent()
                    .ok_or_else(|| "path has no existing ancestor".to_string())?
                    .to_path_buf();
            }
            None => return Err("path has no file name".into()),
        }
    }
}

/// True when `candidate` is lexically inside `root`, used only by tests now:
/// containment is enforced by `scratch::materialise`, which also resolves the
/// canonical path (a lexical check alone does not see a junction).
#[cfg(test)]
pub fn lexically_inside(root: &Path, candidate: &Path) -> bool {
    let mut depth: i32 = 0;
    let rel = match candidate.strip_prefix(root) {
        Ok(rel) => rel,
        Err(_) => return false,
    };
    for component in rel.components() {
        match component {
            Component::ParentDir => depth -= 1,
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            _ => return false,
        }
        if depth < 0 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_absolute() {
        for bad in [
            "../x",
            "a/../../b",
            "/etc/passwd",
            "C:\\Windows\\x",
            "a//b",
            "",
        ] {
            assert!(validate_relative(bad).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn rejects_windows_hazards() {
        for bad in [
            "CON", "aux.txt", "com1.log", "dir/nul/", "x.", "x ", "a/b:/c",
        ] {
            assert!(validate_relative(bad).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn accepts_ordinary_paths() {
        for good in [
            "src/lib.rs",
            "docs/a b.md",
            "a/b/c.txt",
            ".flow/config.json",
        ] {
            assert!(validate_relative(good).is_ok(), "{good} should be accepted");
        }
    }

    #[test]
    fn lexical_boundary() {
        let root = Path::new("/repo");
        assert!(lexically_inside(root, Path::new("/repo/a/b")));
        assert!(!lexically_inside(root, Path::new("/repo/../etc")));
        assert!(!lexically_inside(root, Path::new("/other")));
    }
}
