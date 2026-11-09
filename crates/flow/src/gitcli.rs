//! Git plumbing through the `git` executable (no libgit2, no dependency) and a
//! bounded obligation runner with an explicit environment allowlist.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct Output {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct Git {
    pub root: PathBuf,
}

impl Git {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Git { root: root.into() }
    }

    fn base(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .env("GIT_EDITOR", "true")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LC_ALL", "C");
        cmd
    }

    pub fn run(&self, args: &[&str]) -> Result<Output, String> {
        let out = self
            .base()
            .args(args)
            .output()
            .map_err(|e| format!("cannot run git {args:?}: {e}"))?;
        Ok(Output {
            status: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        })
    }

    pub fn capture(&self, args: &[&str]) -> Result<String, String> {
        let out = self.run(args)?;
        if out.status != 0 {
            return Err(format!(
                "git {args:?} failed ({}): {}",
                out.status,
                out.stderr.trim()
            ));
        }
        Ok(out.stdout.trim().to_string())
    }

    pub fn head_oid(&self) -> Result<String, String> {
        self.capture(&["rev-parse", "HEAD"])
    }

    pub fn head_tree(&self) -> Result<String, String> {
        self.capture(&["rev-parse", "HEAD^{tree}"])
    }

    pub fn write_tree(&self) -> Result<String, String> {
        self.capture(&["write-tree"])
    }

    pub fn commit_tree(&self, tree: &str, parent: &str, message: &str) -> Result<String, String> {
        self.capture(&["commit-tree", tree, "-p", parent, "-m", message])
    }

    pub fn update_ref(&self, reference: &str, new: &str, old: &str) -> Result<Output, String> {
        self.run(&["update-ref", reference, new, old])
    }

    pub fn show_ref(&self, reference: &str) -> Result<Option<String>, String> {
        let out = self.run(&["rev-parse", "--verify", "--quiet", reference])?;
        if out.status == 0 {
            Ok(Some(out.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn status_porcelain(&self) -> Result<Vec<String>, String> {
        let out = self.capture(&["status", "--porcelain=v1", "-uall"])?;
        Ok(out.lines().map(str::to_string).collect())
    }

    pub fn version(&self) -> String {
        self.run(&["--version"])
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_else(|e| format!("UNKNOWN: {e}"))
    }
}

/// Locate the repository root, refusing to run outside a Git work tree.
pub fn toplevel(cwd: &Path) -> Result<PathBuf, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["rev-parse", "--show-toplevel"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err("not inside a Git work tree".into());
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    PathBuf::from(path)
        .canonicalize()
        .map_err(|e| format!("cannot canonicalise repository root: {e}"))
}

// ------------------------------------------------------------ obligations
#[derive(Debug, Clone)]
pub struct Obligation {
    pub id: String,
    pub program: String,
    pub args: Vec<String>,
    pub required: bool,
    pub timeout_s: u64,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub id: String,
    pub status: &'static str, // PASSED | FAILED | TIMEOUT | UNSUPPORTED | SKIPPED
    pub exit: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
}

const ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "PATHEXT",
    "SystemRoot",
    "SYSTEMROOT",
    "windir",
    "WINDIR",
    "COMSPEC",
    "TEMP",
    "TMP",
    "TMPDIR",
    "HOME",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "LANG",
    "LC_ALL",
    "TERM",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "SHELL",
    "TZ",
];

/// The environment names an obligation may inherit. This list is part of the
/// policy identity: widening it changes what a receipt proves, so it is hashed
/// into the policy digest and invalidates older receipts.
pub fn env_allowlist() -> &'static [&'static str] {
    ENV_ALLOWLIST
}

fn allowed_env() -> Vec<(String, String)> {
    let mut kept = Vec::new();
    for name in ENV_ALLOWLIST {
        // Windows environment names are case-insensitive: scan for the first match.
        if let Some((k, v)) = std::env::vars().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
            kept.push((k, v));
        }
    }
    kept
}

pub fn run_obligation(ob: &Obligation, cwd: &Path) -> Outcome {
    let started = Instant::now();
    let mut cmd = Command::new(&ob.program);
    cmd.args(&ob.args)
        .current_dir(cwd)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in allowed_env() {
        cmd.env(k, v);
    }
    let mut child: Child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            return Outcome {
                id: ob.id.clone(),
                status: "UNSUPPORTED",
                exit: None,
                timed_out: false,
                duration_ms: started.elapsed().as_millis(),
                stdout: String::new(),
                stderr: format!("cannot start {:?}: {e}", ob.program),
            }
        }
    };
    let mut stdout_pipe = child.stdout.take().expect("piped stdout");
    let mut stderr_pipe = child.stderr.take().expect("piped stderr");
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut buf);
        buf
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        buf
    });
    let deadline = Duration::from_secs(ob.timeout_s.max(1));
    let mut timed_out = false;
    let exit = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if started.elapsed() > deadline {
                    timed_out = true;
                    kill_tree(child.id());
                    let _ = child.kill();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                let _ = child.kill();
                break None;
            }
        }
    };
    let stdout = String::from_utf8_lossy(&out_thread.join().unwrap_or_default()).to_string();
    let stderr = String::from_utf8_lossy(&err_thread.join().unwrap_or_default()).to_string();
    let status = if timed_out {
        "TIMEOUT"
    } else if exit == Some(0) {
        "PASSED"
    } else {
        "FAILED"
    };
    Outcome {
        id: ob.id.clone(),
        status,
        exit,
        timed_out,
        duration_ms: started.elapsed().as_millis(),
        stdout,
        stderr,
    }
}

/// Kill the child and, where the platform supports it, its descendants.
/// Windows: `taskkill /T`. POSIX: direct kill only (no process group without a
/// dependency) — the survivor limitation is documented, never hidden.
pub fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
    }
}

/// Redact values of secret-looking environment variables and known token shapes
/// BEFORE a log is written to disk.
pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for (name, value) in std::env::vars() {
        let upper = name.to_ascii_uppercase();
        let looks_secret = [
            "SECRET",
            "TOKEN",
            "PASSWORD",
            "PASSWD",
            "PRIVATE_KEY",
            "API_KEY",
        ]
        .iter()
        .any(|needle| upper.contains(needle));
        if looks_secret && value.len() >= 8 && out.contains(&value) {
            out = out.replace(&value, "[REDACTED]");
        }
    }
    for shape in ["ghp_", "gho_", "ghs_", "github_pat_", "sk-"] {
        while let Some(idx) = out.find(shape) {
            let end = out[idx..]
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
                .map(|off| idx + off)
                .unwrap_or(out.len());
            if end <= idx + shape.len() {
                break;
            }
            out.replace_range(idx..end, "[REDACTED]");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secret_env_value() {
        std::env::set_var("FLOW_TEST_SECRET_CANARY_X", "super-secret-value-1234");
        let out = redact("value=super-secret-value-1234 end");
        assert!(!out.contains("super-secret-value-1234"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_token_shape() {
        let out = redact("token ghp_abcdefghijklmnopqrstuvwxyz1234 done");
        assert!(!out.contains("abcdefghijklmnopqrstuvwxyz1234"));
    }

    #[test]
    fn missing_program_is_unsupported_not_failed_green() {
        let ob = Obligation {
            id: "x".into(),
            program: "definitely-not-a-real-program-flow".into(),
            args: vec![],
            required: true,
            timeout_s: 2,
        };
        let out = run_obligation(&ob, Path::new("."));
        assert_eq!(out.status, "UNSUPPORTED");
    }
}
