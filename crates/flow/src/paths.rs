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

/// True for a Git object id this build accepts: 40 or 64 hexadecimal characters.
///
/// Used wherever an identifier becomes a file name (`receipts/<tree>.json`): a
/// raw `../` or an absolute path must never reach the filesystem.
pub fn is_object_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// True for a Flow record id (proposal / candidate): 32 hexadecimal characters.
/// Hex case is not significant: the ids this crate writes are lowercase, and
/// accepting an uppercase spelling only widens what can be *read*.
pub fn is_record_id(value: &str) -> bool {
    value.len() == 32 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate a repository-relative path as written in a receipt or config.
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

    // The following tests exist to pin guarantees that a mutation run showed were
    // not observed by the suite: `canonical_within`'s return value was discarded
    // by its only caller, so an implementation returning an empty path passed.

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("flow-paths-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
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
    fn canonical_within_returns_the_resolved_path() {
        let root = temp_root("cw-ok");
        std::fs::create_dir_all(root.join("a").join("b")).unwrap();
        let resolved = canonical_within(&root, &root.join("a/b/log.json")).unwrap();
        assert_eq!(
            resolved,
            root.canonicalize()
                .unwrap()
                .join("a")
                .join("b")
                .join("log.json"),
            "the existing ancestor is resolved and the missing suffix re-applied"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn canonical_within_refuses_a_link_that_leaves_the_root() {
        let root = temp_root("cw-escape");
        let base = temp_root("cw-escape-out");
        let outside = base.join("out");
        std::fs::create_dir_all(&outside).unwrap();
        let link = root.join("link");
        // This is a required gate: a host that cannot create a directory link
        // cannot certify containment, and a silent skip would report "ok"
        // without the refusal ever running.
        make_dir_link(&outside, &link).expect(
            "this gate proves that a link out of the root is refused, so the host must be able to create one",
        );
        let err = canonical_within(&root, &link.join("receipt.json"))
            .expect_err("a link out of the root must be refused, even for a missing child");
        assert!(err.contains("escapes"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn length_limits_are_exact() {
        // 255-byte components are representable, 256 are not.
        assert!(validate_relative(&"a".repeat(255)).is_ok());
        assert!(validate_relative(&"a".repeat(256)).is_err());

        // A path of exactly 4096 bytes built from legal components, then 4097.
        let mut path = "a".repeat(100);
        for _ in 0..38 {
            path.push('/');
            path.push_str(&"a".repeat(100));
        }
        path.push('/');
        path.push_str(&"a".repeat(157));
        assert_eq!(path.len(), 4096);
        assert!(validate_relative(&path).is_ok(), "4096 bytes is allowed");
        path.push('a');
        assert_eq!(path.len(), 4097);
        assert!(validate_relative(&path).is_err(), "4097 bytes is refused");

        assert!(
            validate_relative("a\nb").is_err(),
            "control characters are refused"
        );
    }

    #[test]
    fn identifiers_are_exactly_sized_hex() {
        assert!(is_object_id(&"a".repeat(40)));
        assert!(is_object_id(&"0123456789abcdef".repeat(4)));
        assert!(!is_object_id(&"a".repeat(39)));
        assert!(!is_object_id(&"a".repeat(41)));
        assert!(!is_object_id(&"g".repeat(40)));

        assert!(is_record_id(&"0123456789abcdef".repeat(2)));
        assert!(!is_record_id(&"a".repeat(31)));
        assert!(!is_record_id(&"a".repeat(33)));
        assert!(!is_record_id(&"z".repeat(32)));
    }

    #[test]
    fn every_absolute_and_traversal_shape_is_refused() {
        // A leading backslash is as absolute as a leading slash on Windows, and
        // a single dot is a traversal component like `..`.
        for bad in [r"\server\share", r"\x", "./a", "a/./b", ".", ".."] {
            assert!(validate_relative(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn each_refusal_names_its_own_reason() {
        // The reason is what an operator or an agent acts on: "absolute",
        // "drive-qualified", "traversal" and "empty component" must not collapse
        // into one message, or the fix is not obvious from the refusal.
        for (path, needle) in [
            (r"\x", "absolute path is not allowed"),
            ("/x", "absolute path is not allowed"),
            ("C:/x", "drive-qualified path is not allowed"),
            ("a/./b", "path traversal component"),
            ("a/../b", "path traversal component"),
            ("a//b", "empty path component"),
        ] {
            let reason = validate_relative(path).expect_err("this path must be refused");
            assert!(reason.contains(needle), "{path:?}: {reason}");
        }
    }

    #[test]
    fn a_path_without_a_name_cannot_be_resolved() {
        let root = temp_root("no-name");
        let err = canonical_within(&root, Path::new(""))
            .expect_err("a path with no file name has no existing ancestor to resolve");
        assert!(err.contains("no file name"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn the_windows_link_helper_reports_failure_instead_of_claiming_success() {
        // A gate that cannot create its fixture must be loud; and a helper that
        // failed must not return `Ok(())`, which would make that gate vacuous.
        let root = temp_root("link-twice");
        let outside = temp_root("link-twice-out");
        let link = root.join("link");
        make_dir_link(&outside, &link).unwrap();
        assert!(
            make_dir_link(&outside, &link).is_err(),
            "re-creating an existing junction must be reported as a failure"
        );
        let _ = std::fs::remove_dir(&link);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
