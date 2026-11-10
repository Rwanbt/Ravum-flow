//! Git plumbing through the `git` executable (no libgit2, no dependency) and a
//! bounded obligation runner with an explicit environment allowlist.
//!
//! Three independent hardenings live here:
//!
//! * every `git` invocation is built from a scrubbed environment, so a hostile
//!   host (or a test harness) cannot redirect plumbing at another repository,
//!   object store, index or config through `GIT_*` variables;
//! * obligations run with a bounded capture (at most [`MAX_STREAM_BYTES`] kept
//!   per stream) and a bounded wall clock, and their reader threads are never
//!   joined, so a descendant that survives the kill and keeps a pipe open can
//!   delay the call by at most [`READER_GRACE`], never indefinitely.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Output {
    pub status: i32,
    pub stdout: String,
    /// Raw stdout bytes. `stdout` is a lossy view for humans; content comparison
    /// must never go through it, because a blob is not guaranteed to be UTF-8.
    pub stdout_bytes: Vec<u8>,
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
        git_command(&self.root)
    }

    pub fn run(&self, args: &[&str]) -> Result<Output, String> {
        self.run_bounded(args, None, None)
    }

    /// Run one Git plumbing command with bounded memory, bounded wall clock and
    /// an optional explicit index.
    ///
    /// * `stdin` is written to the child and the pipe is then closed; it is
    ///   written from a detached thread so a child that produces output before
    ///   reading its input cannot deadlock the call.
    /// * `index` sets `GIT_INDEX_FILE` **for this invocation only**. The ambient
    ///   environment is still scrubbed, so an inherited `GIT_INDEX_FILE` is
    ///   ignored; the path must be absolute, and the caller is responsible for
    ///   having created it (this is how proposal assembly avoids the user's
    ///   index).
    /// * At most [`MAX_GIT_STREAM_BYTES`] is retained per stream. Past that the
    ///   call **fails** with `GIT_OUTPUT_TOO_LARGE` instead of truncating: a
    ///   clipped Git answer is not an answer, and it must never be parsed as one.
    /// * The wall clock is bounded by [`GIT_TIMEOUT_S`]; the child's tree is
    ///   killed and the call fails with `GIT_TIMEOUT`.
    pub fn run_bounded(
        &self,
        args: &[&str],
        stdin: Option<&[u8]>,
        index: Option<&Path>,
    ) -> Result<Output, String> {
        let mut cmd = self.base();
        cmd.args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(path) = index {
            if !path.is_absolute() {
                return Err(format!(
                    "GIT_INDEX_REFUSED: index override {} is not an absolute path",
                    path.display()
                ));
            }
            cmd.env("GIT_INDEX_FILE", path);
        }
        let started = Instant::now();
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot run git {args:?}: {e}"))?;
        let stdout_pipe = child.stdout.take().expect("piped stdout");
        let stderr_pipe = child.stderr.take().expect("piped stderr");
        let out_rx = spawn_reader_with_cap(stdout_pipe, MAX_GIT_STREAM_BYTES);
        let err_rx = spawn_reader_with_cap(stderr_pipe, MAX_GIT_STREAM_BYTES);
        if let Some(bytes) = stdin {
            if let Some(mut pipe) = child.stdin.take() {
                let owned = bytes.to_vec();
                // Detached on purpose: the child may exit before reading all of
                // its input, and a broken pipe is not an error worth reporting.
                let _ = std::thread::Builder::new()
                    .name("flow-git-stdin".into())
                    .spawn(move || {
                        let _ = pipe.write_all(&owned);
                    });
            }
        }

        let deadline = started + Duration::from_secs(GIT_TIMEOUT_S);
        let mut timed_out = false;
        let mut exit = None;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    exit = status.code();
                    break;
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        timed_out = true;
                        kill_tree(child.id());
                        let _ = child.kill();
                        reap_bounded(&mut child);
                        break;
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(e) => {
                    let _ = child.kill();
                    reap_bounded(&mut child);
                    return Err(format!("git {args:?} could not be waited for: {e}"));
                }
            }
        }
        let reader_deadline = Instant::now() + READER_GRACE;
        let stdout = receive(&out_rx, reader_deadline);
        let stderr = receive(&err_rx, reader_deadline);
        if timed_out {
            return Err(format!(
                "GIT_TIMEOUT: git {args:?} exceeded {GIT_TIMEOUT_S}s and was killed; its output is incomplete"
            ));
        }
        if stdout.truncated {
            return Err(format!(
                "GIT_OUTPUT_TOO_LARGE: git {args:?} wrote more than {MAX_GIT_STREAM_BYTES} bytes to stdout; refusing to parse a truncated answer"
            ));
        }
        if stderr.truncated {
            return Err(format!(
                "GIT_OUTPUT_TOO_LARGE: git {args:?} wrote more than {MAX_GIT_STREAM_BYTES} bytes to stderr; refusing to parse a truncated answer"
            ));
        }
        Ok(Output {
            status: exit.unwrap_or(-1),
            stdout: String::from_utf8_lossy(&stdout.bytes).to_string(),
            stdout_bytes: stdout.bytes,
            stderr: String::from_utf8_lossy(&stderr.bytes).to_string(),
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
///
/// The command is built with the same scrubbed environment as every other Git
/// invocation: a host `GIT_DIR`/`GIT_WORK_TREE` cannot make this answer come
/// from somewhere else.
pub fn toplevel(cwd: &Path) -> Result<PathBuf, String> {
    let git = Git::new(cwd);
    let out = git.run_bounded(&["rev-parse", "--show-toplevel"], None, None)?;
    if out.status != 0 {
        return Err("not inside a Git work tree".into());
    }
    let path = out.stdout.trim().to_string();
    PathBuf::from(path)
        .canonicalize()
        .map_err(|e| format!("cannot canonicalise repository root: {e}"))
}

/// Variables that redirect Git at another repository, index, object store,
/// configuration or helper program. Every occurrence is removed from the child
/// environment (case-insensitively, because Windows environment names are).
///
/// `GIT_CONFIG_KEY_*`/`GIT_CONFIG_VALUE_*` are dynamic and handled by prefix.
const SCRUBBED_GIT_ENV: &[&str] = &[
    "GIT_ALLOW_PROTOCOL",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_ASKPASS",
    "GIT_CEILING_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_CONFIG",
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_SYSTEM",
    "GIT_DIR",
    "GIT_DISCOVERY_ACROSS_FILESYSTEM",
    "GIT_EDITOR",
    "GIT_EXTERNAL_DIFF",
    "GIT_INDEX_FILE",
    "GIT_NAMESPACE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PAGER",
    "GIT_PROTOCOL_FROM_USER",
    "GIT_SEQUENCE_EDITOR",
    "GIT_SSH",
    "GIT_SSH_COMMAND",
    "GIT_TERMINAL_PROMPT",
    "GIT_WORK_TREE",
    "SSH_ASKPASS",
];

/// Read-only environment allowlist for hashing and for obligation processes.
///
/// Sorted (byte order) and duplicate-free so a hash over it is stable across
/// hosts; [`env_allowlist`] exposes exactly this slice.
pub static ENV_ALLOWLIST: &[&str] = &[
    "APPDATA",
    "CARGO_HOME",
    "COMSPEC",
    "HOME",
    "INCLUDE",
    "LANG",
    "LC_ALL",
    "LIB",
    "LIBPATH",
    "LOCALAPPDATA",
    "NUMBER_OF_PROCESSORS",
    "PATH",
    "PATHEXT",
    "PROCESSOR_ARCHITECTURE",
    "RUSTUP_HOME",
    "SHELL",
    "SYSTEMROOT",
    "SystemRoot",
    "TEMP",
    "TERM",
    "TMP",
    "TMPDIR",
    "TZ",
    "UCRTVersion",
    "USERPROFILE",
    // MSVC toolchain configuration. Without these a native link cannot find its
    // libraries (`LNK1181: cannot open input file`) when the host exports the
    // environment instead of relying on registry discovery — the normal case in
    // CI and in a Visual Studio developer prompt. They are paths to a toolchain,
    // not credentials, and they are part of the hashed effective policy, so
    // adding them invalidates older receipts instead of silently changing what
    // `PASSED` meant.
    "VCINSTALLDIR",
    "VCToolsInstallDir",
    "VSCMD_ARG_TGT_ARCH",
    "VSINSTALLDIR",
    "WINDIR",
    "WindowsSDKVersion",
    "WindowsSdkDir",
    "windir",
];

/// The stable, sorted environment allowlist (for hashing and reporting).
pub fn env_allowlist() -> &'static [&'static str] {
    ENV_ALLOWLIST
}

/// Build `git -C <root>` with the redirecting part of the host environment
/// removed and the non-interactive defaults applied.
fn git_command(root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root);
    scrub_env(&mut cmd);
    cmd
}

/// Remove every environment variable that could redirect Git, then set the
/// non-interactive, deterministic defaults. Host variables are inspected with
/// `vars_os` so a non-UTF-8 value cannot panic the CLI; only the *names* are
/// compared.
fn scrub_env(cmd: &mut Command) {
    for (name, _) in std::env::vars_os() {
        let upper = name.to_string_lossy().to_ascii_uppercase();
        let redirects = SCRUBBED_GIT_ENV.contains(&upper.as_str())
            || upper.starts_with("GIT_CONFIG_KEY_")
            || upper.starts_with("GIT_CONFIG_VALUE_");
        if redirects {
            cmd.env_remove(&name);
        }
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("GIT_EDITOR", "true")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("LC_ALL", "C");
}

fn allowed_env() -> Vec<(OsString, OsString)> {
    let host: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    let mut kept = Vec::new();
    for wanted in env_allowlist() {
        // Windows environment names are case-insensitive: scan for the first match.
        if let Some((name, value)) = host
            .iter()
            .find(|(name, _)| name.to_string_lossy().eq_ignore_ascii_case(wanted))
        {
            kept.push((name.clone(), value.clone()));
        }
    }
    kept
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
    /// True when the child wrote more than [`MAX_STREAM_BYTES`] to stdout; the
    /// surplus was drained and discarded (it is not buffered anywhere).
    pub stdout_truncated: bool,
    /// True when the child wrote more than [`MAX_STREAM_BYTES`] to stderr.
    pub stderr_truncated: bool,
}

/// Bytes retained per stream. Anything beyond is drained to keep the child
/// unblocked, then discarded.
pub const MAX_STREAM_BYTES: usize = 1024 * 1024;

/// Appended to a truncated stream. Deliberately free of a copy of the cap so it
/// cannot drift from [`MAX_STREAM_BYTES`].
pub const TRUNCATION_MARKER: &str = "\n[flow: output truncated; remaining bytes discarded]\n";

/// How long the reader threads may take to observe EOF after the child tree was
/// killed. Streams that are still held open by a survivor are abandoned after
/// this budget; their threads are detached, never joined.
const READER_GRACE: Duration = Duration::from_secs(2);

/// How long to wait for the killed child itself to be reaped.
const REAP_GRACE: Duration = Duration::from_secs(2);

const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Capture returned by a reader thread: retained bytes plus whether the stream
/// exceeded the cap.
#[derive(Default)]
struct Captured {
    bytes: Vec<u8>,
    truncated: bool,
}

/// Retained Git output per stream before the call fails.
///
/// Larger than an obligation's cap because legitimate plumbing (`ls-tree -r` on
/// a big tree) can be large. It is still a bound: past it the answer is refused,
/// never clipped.
pub const MAX_GIT_STREAM_BYTES: usize = 64 * 1024 * 1024;

/// Wall-clock bound for one Git invocation. Generous: it exists to stop a wedged
/// child from hanging the CLI forever, not to police slow repositories.
pub const GIT_TIMEOUT_S: u64 = 300;

/// Read `pipe` to EOF while retaining at most `cap` bytes.
///
/// The surplus is still read and discarded: stopping the read would fill the
/// pipe and deadlock a child that is writing (for example during a timeout).
fn read_capped_at<R: Read>(mut pipe: R, cap: usize) -> Captured {
    let mut captured = Captured::default();
    let mut chunk = [0u8; 8192];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let room = cap.saturating_sub(captured.bytes.len());
                if room == 0 {
                    captured.truncated = true;
                    continue;
                }
                let take = n.min(room);
                captured.bytes.extend_from_slice(&chunk[..take]);
                if take < n {
                    captured.truncated = true;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    captured
}

/// Start a detached reader thread. The thread is never joined: it sends its
/// result if it ever reaches EOF and otherwise stays parked on the pipe (see
/// the non-guarantees on [`run_obligation`]).
fn spawn_reader<R: Read + Send + 'static>(pipe: R) -> mpsc::Receiver<Captured> {
    spawn_reader_with_cap(pipe, MAX_STREAM_BYTES)
}

fn spawn_reader_with_cap<R: Read + Send + 'static>(
    pipe: R,
    cap: usize,
) -> mpsc::Receiver<Captured> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("flow-obligation-reader".into())
        .spawn(move || {
            let _ = tx.send(read_capped_at(pipe, cap));
        });
    rx
}

fn finish(captured: Captured) -> (String, bool) {
    let mut text = String::from_utf8_lossy(&captured.bytes).to_string();
    if captured.truncated {
        text.push_str(TRUNCATION_MARKER);
    }
    (text, captured.truncated)
}

fn receive(rx: &mpsc::Receiver<Captured>, deadline: Instant) -> Captured {
    // A timed-out receive leaves the thread detached; the channel is dropped
    // with this function, so a later send is simply ignored.
    rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .unwrap_or_default()
}

/// Run one obligation with bounded output and bounded wall-clock containment.
///
/// Bounds:
/// * at most [`MAX_STREAM_BYTES`] retained per stream (the rest is drained and
///   discarded), flagged through [`Outcome::stdout_truncated`] /
///   [`Outcome::stderr_truncated`];
/// * one timeout in seconds, after which the child tree is killed;
/// * after the kill, readers get at most [`READER_GRACE`] to observe EOF.
///
/// Non-guarantees (explicit, not sandboxing):
/// * Windows kills the tree with `taskkill /T /F`; a descendant that has
///   re-parented itself or otherwise left the tree survives.
/// * Unix spawns the child as a process-group leader and signals the group with
///   `SIGKILL`; a descendant that calls `setsid`/`setpgid` escapes the group
///   and survives.
/// * A surviving descendant that inherited stdout/stderr keeps its pipe open.
///   The reader thread for that stream is detached after the grace period, so
///   the call returns; the thread (and the pipe) lives until the survivor exits
///   or the process exits.
// Resolve a program name through PATH and PATHEXT on Windows.
//
// CreateProcess only appends .exe, so a tool installed as a .cmd or .bat shim
// (which is how npm installs tsc, npx, gradle, ...) is not found by name. Flow
// resolves it the way a shell would and only then spawns it. When nothing
// matches, the caller keeps the declared name so the failure is reported as
// UNSUPPORTED with the name the policy asked for.
#[cfg(windows)]
fn resolve_program(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_ascii_lowercase())
        .collect();
    let has_extension = std::path::Path::new(program).extension().is_some();
    for dir in std::env::split_paths(&path) {
        if has_extension {
            let candidate = dir.join(program);
            if candidate.is_file() {
                return Some(candidate);
            }
        } else {
            for extension in &extensions {
                let candidate = dir.join(format!("{program}{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            let candidate = dir.join(program);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn resolve_program(_program: &str) -> Option<PathBuf> {
    None
}

pub fn run_obligation(ob: &Obligation, cwd: &Path) -> Outcome {
    let started = Instant::now();
    // A resolved absolute path is used when PATH+PATHEXT finds one; otherwise the
    // declared name is kept, so the message names what the policy asked for.
    let program: PathBuf =
        resolve_program(&ob.program).unwrap_or_else(|| PathBuf::from(&ob.program));
    let mut cmd = Command::new(&program);
    cmd.args(&ob.args)
        .current_dir(cwd)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in allowed_env() {
        cmd.env(k, v);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Process-group leader: a negative PID then addresses the whole group.
        cmd.process_group(0);
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
                stdout_truncated: false,
                stderr_truncated: false,
            }
        }
    };
    let stdout_pipe = child.stdout.take().expect("piped stdout");
    let stderr_pipe = child.stderr.take().expect("piped stderr");
    let out_rx = spawn_reader(stdout_pipe);
    let err_rx = spawn_reader(stderr_pipe);

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
                    reap_bounded(&mut child);
                    break None;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(_) => {
                let _ = child.kill();
                reap_bounded(&mut child);
                break None;
            }
        }
    };

    let reader_deadline = Instant::now() + READER_GRACE;
    let stdout = finish(receive(&out_rx, reader_deadline));
    let stderr = finish(receive(&err_rx, reader_deadline));

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
        stdout: stdout.0,
        stderr: stderr.0,
        stdout_truncated: stdout.1,
        stderr_truncated: stderr.1,
    }
}

/// Wait, bounded, for a killed child to be reaped so no zombie accumulates.
fn reap_bounded(child: &mut Child) {
    let deadline = Instant::now() + REAP_GRACE;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => std::thread::sleep(POLL_INTERVAL),
        }
    }
}

/// Kill the child and, where the platform supports it, its descendants.
///
/// Windows: `taskkill /T /F` walks the process tree.
///
/// POSIX: the child was spawned as a process-group leader (see
/// [`run_obligation`]), so the shell's `kill` builtin is asked to signal
/// `-<pid>`; the plain PID is signalled as a fallback. A descendant that leaves
/// the process group is not reached — the limitation is documented, never
/// hidden. (No `unsafe` FFI: the POSIX shell builtin is used instead of
/// `libc::kill`, keeping the crate dependency-free.)
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
        // `-s KILL` avoids the `-9`/`-9 -pgid` option ambiguity; the operands
        // are quoted positional arguments, never interpreted by the shell.
        let group = format!("-{pid}");
        let direct = pid.to_string();
        let _ = Command::new("sh")
            .args([
                "-c",
                "kill -s KILL \"$1\" 2>/dev/null || kill -s KILL \"$2\" 2>/dev/null",
                "flow-kill-tree",
                &group,
                &direct,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
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
        assert!(!out.stdout_truncated);
        assert!(!out.stderr_truncated);
    }

    #[test]
    fn env_allowlist_is_sorted_and_unique() {
        let list = env_allowlist();
        assert!(
            list.windows(2).all(|pair| pair[0] < pair[1]),
            "the allowlist must be sorted and duplicate-free for stable hashing: {list:?}"
        );
        for required in ["PATH", "SystemRoot", "SYSTEMROOT", "HOME", "TEMP"] {
            assert!(list.contains(&required), "{required} must stay allowlisted");
        }
        assert_eq!(env_allowlist(), ENV_ALLOWLIST);
    }

    #[test]
    fn scrub_list_covers_every_documented_redirect() {
        for name in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_COMMON_DIR",
            "GIT_NAMESPACE",
            "GIT_CONFIG",
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_SYSTEM",
            "GIT_CONFIG_COUNT",
            "GIT_CEILING_DIRECTORIES",
            "GIT_DISCOVERY_ACROSS_FILESYSTEM",
            "GIT_ASKPASS",
            "SSH_ASKPASS",
            "GIT_SSH",
            "GIT_SSH_COMMAND",
            "GIT_EXTERNAL_DIFF",
            "GIT_PAGER",
            "GIT_EDITOR",
            "GIT_SEQUENCE_EDITOR",
            "GIT_PROTOCOL_FROM_USER",
            "GIT_ALLOW_PROTOCOL",
        ] {
            assert!(
                SCRUBBED_GIT_ENV.contains(&name),
                "{name} must be scrubbed from Git invocations"
            );
        }
    }

    #[test]
    fn read_capped_at_retains_at_most_the_cap_and_flags_truncation() {
        let data = vec![b'x'; MAX_STREAM_BYTES + 4096];
        let captured = read_capped_at(std::io::Cursor::new(data), MAX_STREAM_BYTES);
        assert_eq!(captured.bytes.len(), MAX_STREAM_BYTES);
        assert!(captured.truncated);
        let text = finish(captured).0;
        assert!(text.len() <= MAX_STREAM_BYTES + TRUNCATION_MARKER.len());
        assert!(text.contains("truncated"));
    }

    #[test]
    fn read_capped_at_keeps_small_streams_intact() {
        let captured = read_capped_at(std::io::Cursor::new(b"hello".to_vec()), MAX_STREAM_BYTES);
        assert_eq!(captured.bytes, b"hello");
        assert!(!captured.truncated);
        assert_eq!(finish(captured), ("hello".to_string(), false));
    }
}
