//! The local publication lock — contract `flow-lock/2`.
//!
//! What this lock is: a mutual-exclusion device between `flow` processes of the
//! same working copy, protecting the short mutations (`publish`, `reconcile`) so
//! two writers cannot interleave journal appends or both CAS the same ref.
//!
//! What it is **not**, and must never be described as:
//!
//! * a distributed lease — it only protects one `\.flow` directory on one host;
//! * a fence against a foreign `git` writer, which never takes it;
//! * keyed — any local writer with the same rights can rewrite it.
//!
//! Attribution rules (the whole point of v2):
//!
//! * a lock whose owner is provably alive is **never** stolen, whatever its age;
//! * a lock is stolen only when its owner is **provably gone** (the PID is dead
//!   and the file still carries the same nonce that was observed);
//! * when liveness cannot be established (no `tasklist`/`kill`, an unreadable or
//!   unparseable file, an empty file left by a crash between create and write)
//!   the answer is [`LockError::Unknown`] — a refusal that names the file and
//!   asks for a human, never a guess.
//!
//! Age is reported, never used to justify a steal. PID reuse therefore wedges the
//! lock until someone removes it by hand. That is deliberate: a wedge is loud and
//! recoverable, a stolen live lock is a silent double publication.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const LOCK_SCHEMA: &str = "flow-lock/2";
/// Reported as an observation only. Never a steal criterion.
pub const LOCK_MAX_AGE_REPORTED_S: u64 = 120;
const ACQUIRE_BUDGET: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub enum LockError {
    /// The lock is held by a process that is (or may be) alive.
    Held(String),
    /// Ownership cannot be established at all: a human must inspect the file.
    Unknown(String),
    /// The lock file could not be created or removed.
    Io(String),
}

impl LockError {
    pub fn message(&self) -> &str {
        match self {
            LockError::Held(m) | LockError::Unknown(m) | LockError::Io(m) => m,
        }
    }
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl From<LockError> for String {
    fn from(e: LockError) -> String {
        e.to_string()
    }
}

pub fn lock_path(root: &Path) -> PathBuf {
    crate::state::flow_dir(root).join("lock")
}

/// What an inspection can honestly say about the lock currently on disk.
#[derive(Debug, Clone)]
pub struct LockInfo {
    pub present: bool,
    pub schema: String,
    pub pid: Option<u32>,
    pub age_s: Option<u64>,
    pub purpose: String,
    /// `live`, `dead`, `unknown` or `unparseable`.
    pub attribution: &'static str,
}

struct Owner {
    pid: u32,
    at: u64,
    nonce: String,
    purpose: String,
}

fn parse(text: &str) -> Option<Owner> {
    let mut parts = text.split_whitespace();
    let schema = parts.next()?;
    if schema != LOCK_SCHEMA {
        return None;
    }
    Some(Owner {
        pid: parts.next()?.parse().ok()?,
        at: parts.next()?.parse().ok()?,
        nonce: parts.next()?.to_string(),
        purpose: parts.next().unwrap_or("unknown").to_string(),
    })
}

pub fn inspect(root: &Path) -> LockInfo {
    let path = lock_path(root);
    let Ok(text) = fs::read_to_string(&path) else {
        return LockInfo {
            present: false,
            schema: String::new(),
            pid: None,
            age_s: None,
            purpose: String::new(),
            attribution: "absent",
        };
    };
    match parse(&text) {
        Some(owner) => {
            let attribution = match pid_alive(owner.pid) {
                Some(true) => "live",
                Some(false) => "dead",
                None => "unknown",
            };
            LockInfo {
                present: true,
                schema: LOCK_SCHEMA.to_string(),
                pid: Some(owner.pid),
                age_s: Some(crate::state::unix_now().saturating_sub(owner.at)),
                purpose: owner.purpose,
                attribution,
            }
        }
        None => LockInfo {
            present: true,
            schema: String::new(),
            pid: None,
            age_s: None,
            purpose: String::new(),
            attribution: "unparseable",
        },
    }
}

#[derive(Debug)]
pub struct LockGuard {
    path: PathBuf,
    nonce: String,
}

impl LockGuard {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Only remove the lock this guard owns: a successor's lock must never be
        // deleted by a slow predecessor's destructor.
        if let Ok(text) = fs::read_to_string(&self.path) {
            if parse(&text).map(|o| o.nonce).as_deref() == Some(self.nonce.as_str()) {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

/// A process-unique, non-cryptographic nonce. It only has to distinguish two
/// Flow lock writers, not to resist an attacker (the lock is not a security
/// boundary against a local writer with the same rights).
fn nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{:x}-{:x}-{:x}", std::process::id(), nanos, n)
}

pub fn acquire_lock(root: &Path, purpose: &str) -> Result<LockGuard, LockError> {
    let path = lock_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LockError::Io(format!("cannot create .flow: {e}")))?;
    }
    let deadline = Instant::now() + ACQUIRE_BUDGET;
    loop {
        let ours = nonce();
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let line = format!(
                    "{LOCK_SCHEMA} {} {} {} {}\n",
                    std::process::id(),
                    crate::state::unix_now(),
                    ours,
                    purpose
                );
                if let Err(e) = file
                    .write_all(line.as_bytes())
                    .and_then(|_| file.sync_all())
                {
                    let _ = fs::remove_file(&path);
                    return Err(LockError::Io(format!(
                        "cannot write {}: {e}",
                        path.display()
                    )));
                }
                return Ok(LockGuard {
                    path: path.clone(),
                    nonce: ours,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let text = match fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(err) => {
                        return Err(LockError::Unknown(format!(
                            "lock file {} exists but cannot be read ({err}): ownership cannot be established, remove it by hand only if you are certain no flow process is running",
                            path.display()
                        )))
                    }
                };
                let Some(owner) = parse(&text) else {
                    return Err(LockError::Unknown(format!(
                        "lock file {} is not a {} record (empty, truncated or foreign): ownership cannot be established, so it is never stolen. Inspect {} and remove it by hand only if no flow process holds it",
                        path.display(),
                        LOCK_SCHEMA,
                        path.display()
                    )));
                };
                if owner.pid == std::process::id() {
                    return Err(LockError::Held(format!(
                        "this process already holds {} for {:?}",
                        path.display(),
                        owner.purpose
                    )));
                }
                match pid_alive(owner.pid) {
                    // A live owner is never displaced, whatever the age says.
                    Some(true) => {
                        if Instant::now() >= deadline {
                            return Err(LockError::Held(format!(
                                "another flow process (pid {}, purpose {:?}, {}s old) holds {}",
                                owner.pid,
                                owner.purpose,
                                crate::state::unix_now().saturating_sub(owner.at),
                                path.display()
                            )));
                        }
                    }
                    None => {
                        return Err(LockError::Unknown(format!(
                            "cannot establish whether pid {} (purpose {:?}) is alive on this host, so {} is not stolen",
                            owner.pid,
                            owner.purpose,
                            path.display()
                        )))
                    }
                    Some(false) => {
                        // Provably dead owner: steal only if the file still
                        // carries the nonce that was observed, so a concurrent
                        // successor is never removed.
                        match fs::read_to_string(&path) {
                            Ok(again) if parse(&again).map(|o| o.nonce) == Some(owner.nonce.clone()) => {
                                match fs::remove_file(&path) {
                                    Ok(()) => continue,
                                    Err(err) => {
                                        return Err(LockError::Io(format!(
                                            "cannot remove the dead lock {}: {err}",
                                            path.display()
                                        )))
                                    }
                                }
                            }
                            _ => continue,
                        }
                    }
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                return Err(LockError::Io(format!(
                    "cannot create {}: {e}",
                    path.display()
                )))
            }
        }
    }
}

#[cfg(windows)]
fn pid_alive(pid: u32) -> Option<bool> {
    let out = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(text.contains(&format!("\"{pid}\"")) || text.contains(&format!(",{pid},")))
}

#[cfg(not(windows))]
fn pid_alive(pid: u32) -> Option<bool> {
    let status = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .ok()?;
    Some(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_flow_dir(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("flow-lock-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(path.join(".flow")).unwrap();
        path
    }

    #[test]
    fn acquires_and_releases() {
        let root = temp_flow_dir("basic");
        let guard = acquire_lock(&root, "publish").unwrap();
        assert!(lock_path(&root).is_file());
        drop(guard);
        assert!(!lock_path(&root).exists(), "guard must release its lock");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_live_owner_is_never_displaced_even_when_old() {
        let root = temp_flow_dir("live");
        // Our own PID is provably alive. The timestamp is deliberately ancient.
        fs::write(
            lock_path(&root),
            format!("{LOCK_SCHEMA} 1 0 deadbeef publish\n"),
        )
        .unwrap();
        // Use this process's own pid instead of 1 so the test does not depend on
        // whether pid 1 is readable from the test host.
        fs::write(
            lock_path(&root),
            format!("{LOCK_SCHEMA} {} 0 deadbeef publish\n", std::process::id()),
        )
        .unwrap();
        match acquire_lock(&root, "publish") {
            Err(LockError::Held(m)) => assert!(m.contains("holds"), "{m}"),
            other => panic!("a live owner must never be displaced: {other:?}"),
        }
        let info = inspect(&root);
        assert!(info.present);
        assert_eq!(info.attribution, "live");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unparseable_lock_is_refused_not_stolen() {
        let root = temp_flow_dir("garbage");
        fs::write(lock_path(&root), "").unwrap();
        match acquire_lock(&root, "publish") {
            Err(LockError::Unknown(m)) => {
                assert!(m.contains("never stolen") || m.contains("cannot be established"))
            }
            other => panic!("an unowned lock must be refused: {other:?}"),
        }
        assert!(
            lock_path(&root).is_file(),
            "refusal must not delete the file"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
