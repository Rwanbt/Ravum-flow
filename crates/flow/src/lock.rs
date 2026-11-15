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
//! * a lock whose owner is provably alive **on this host** is **never** stolen,
//!   whatever its age. "This host" is the host-name hash of `flow-lock/2`; two
//!   machines that share a host name are indistinguishable, which is why a shared
//!   `.flow/` across machines is unsupported rather than "protected";
//! * a dead lock is stolen only when its record names **this host** and the steal
//!   itself is claimed by a per-generation marker (`lock.steal.<nonce>`): the
//!   marker's creator alone may remove that exact lock, so two stealers can
//!   never delete each other's fresh lock;
//! * a record without a host token, or written by another host, is never stolen:
//!   a shared `.flow/` across hosts has incomparable PID spaces;
//! * when liveness cannot be established (no `tasklist`/`kill`, an unreadable or
//!   unparseable file, an empty file left by a crash between create and write, a
//!   `kill -0` that failed for any reason other than "no such process") the
//!   answer is [`LockError::Unknown`] — a refusal that names the file and asks
//!   for a human, never a guess. `EPERM` is not "gone": a group-writable checkout
//!   can hold a live lock owned by another user.
//!
//! Age is reported, never used to justify a steal. PID reuse therefore wedges the
//! lock until someone removes it by hand. That is deliberate: a wedge is loud and
//! recoverable, a stolen live lock is a silent double publication.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const LOCK_SCHEMA: &str = "flow-lock/2";

const ACQUIRE_BUDGET: Duration = Duration::from_secs(3);
/// How long a writer waits for another process to finish recovering a dead lock
/// before giving up with `LOCK_UNKNOWN`. Separate from `ACQUIRE_BUDGET`: the
/// recovery is a short, claimed operation, and a burst of publishers must not
/// turn a healthy recovery into a manual-intervention refusal.
const STEAL_GRACE: Duration = Duration::from_secs(15);
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
    /// The host that wrote the record. `None` for records written before the
    /// host token existed: such a lock is never stolen, because a shared
    /// `.flow/` across hosts (network share, bind mount, WSL/Windows) can make a
    /// foreign live PID look dead locally.
    host: Option<String>,
}

use crate::host::local_host_id;

fn parse(text: &str) -> Option<Owner> {
    let mut parts = text.split_whitespace();
    let schema = parts.next()?;
    if schema != LOCK_SCHEMA {
        return None;
    }
    let pid = parts.next()?.parse().ok()?;
    let at = parts.next()?.parse().ok()?;
    let nonce = parts.next()?.to_string();
    let purpose = parts.next().unwrap_or("unknown").to_string();
    let host = parts
        .next()
        .and_then(|token| token.strip_prefix("host:"))
        .map(str::to_string);
    Some(Owner {
        pid,
        at,
        nonce,
        purpose,
        host,
    })
}

pub fn inspect(root: &Path) -> LockInfo {
    let path = lock_path(root);
    let Ok(text) = fs::read_to_string(&path) else {
        return LockInfo {
            present: false,
            pid: None,
            age_s: None,
            purpose: String::new(),
            attribution: "absent",
        };
    };
    match parse(&text) {
        Some(owner) => {
            let local = local_host_id();
            let foreign = match (&owner.host, &local) {
                (Some(writer), Some(here)) => writer != here,
                // A record without a host token predates the identity, so it
                // cannot be attributed to this machine: it is not "dead".
                (None, _) => true,
                (Some(_), None) => true,
            };
            let attribution = if foreign {
                "foreign"
            } else {
                match pid_alive(owner.pid) {
                    Some(true) => "live",
                    Some(false) => "dead",
                    None => "unknown",
                }
            };
            LockInfo {
                present: true,
                pid: Some(owner.pid),
                age_s: Some(crate::state::unix_now().saturating_sub(owner.at)),
                purpose: owner.purpose,
                attribution,
            }
        }
        None => LockInfo {
            present: true,
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

/// How long a reader waits for a concurrent creator to finish writing the lock
/// record before concluding that the file is unattributable.
const CREATE_GRACE: Duration = Duration::from_millis(750);

/// Create the lock file **with its full content** in one atomic step.
///
/// `create_new` followed by `write` exposes a window in which another process
/// reads an empty file — a real race that was observed on Linux with eight
/// concurrent publishers. Writing the record to a unique temporary file and then
/// hard-linking it makes the appearance of the lock atomic: the name never exists
/// without content, and the link fails if a lock already exists. The temporary
/// name is removed on every path.
fn create_lock_file(path: &Path, nonce: &str, purpose: &str) -> std::io::Result<()> {
    let tmp = path.with_file_name(format!("lock.{nonce}.tmp"));
    let host = local_host_id()
        .map(|h| format!(" host:{h}"))
        .unwrap_or_default();
    let line = format!(
        "{LOCK_SCHEMA} {} {} {nonce} {purpose}{host}\n",
        std::process::id(),
        crate::state::unix_now()
    );
    {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        file.write_all(line.as_bytes())?;
        file.sync_all()?;
    }
    let linked = fs::hard_link(&tmp, path);
    let _ = fs::remove_file(&tmp);
    linked
}

/// Read the lock record, tolerating a creator that is still writing it.
fn wait_for_record(path: &Path, first: String) -> Option<Owner> {
    if let Some(owner) = parse(&first) {
        return Some(owner);
    }
    let deadline = Instant::now() + CREATE_GRACE;
    while Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
        let Ok(text) = fs::read_to_string(path) else {
            return None;
        };
        if let Some(owner) = parse(&text) {
            return Some(owner);
        }
    }
    None
}

/// The per-generation steal marker. Its name carries the dead owner's nonce, so
/// one generation has exactly one marker and a second stealer waits instead of
/// racing the first.
fn steal_marker_path(path: &Path, nonce: &str) -> PathBuf {
    path.with_file_name(format!("lock.steal.{nonce}"))
}

pub fn acquire_lock(root: &Path, purpose: &str) -> Result<LockGuard, LockError> {
    let path = lock_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LockError::Io(format!("cannot create .flow: {e}")))?;
    }
    let deadline = Instant::now() + ACQUIRE_BUDGET;
    let mut steal_waited: Option<Instant> = None;
    loop {
        let ours = nonce();
        match create_lock_file(&path, &ours, purpose) {
            Ok(()) => {
                return Ok(LockGuard {
                    path: path.clone(),
                    nonce: ours,
                })
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
                // A creator may have created the file and not finished writing it
                // yet. Wait for the record to appear before concluding anything:
                // "empty right now" is not "unattributable forever".
                let owner = match wait_for_record(&path, text) {
                    Some(owner) => owner,
                    None => {
                        return Err(LockError::Unknown(format!(
                            "lock file {} is not a {} record (empty, truncated or foreign): ownership cannot be established, so it is never stolen. Inspect {} and remove it by hand only if no flow process holds it",
                            path.display(),
                            LOCK_SCHEMA,
                            path.display()
                        )))
                    }
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
                        // Provably dead owner. The steal itself is serialized by
                        // a per-generation marker: only the process that creates
                        // it may remove that exact lock, so two stealers can
                        // never delete each other's fresh lock. A foreign or
                        // host-less record is refused, never guessed.
                        let same_host = match (owner.host.as_deref(), local_host_id()) {
                            (Some(writer), Some(here)) => writer == here,
                            _ => false,
                        };
                        if !same_host {
                            return Err(LockError::Unknown(format!(
                                "lock {} cannot be attributed to this host (foreign or host-less record): refusing to steal it. Inspect it and remove it by hand only if no flow process holds it",
                                path.display()
                            )));
                        }
                        let marker = steal_marker_path(&path, &owner.nonce);
                        let marker_body = format!(
                            "{LOCK_SCHEMA} steal {} {} {} {}\n",
                            std::process::id(),
                            crate::state::unix_now(),
                            nonce(),
                            owner.nonce
                        );
                        match OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&marker)
                        {
                            Ok(mut file) => {
                                let written = file.write_all(marker_body.as_bytes());
                                drop(file);
                                if let Err(err) = written {
                                    let _ = fs::remove_file(&marker);
                                    return Err(LockError::Io(format!(
                                        "cannot record the steal of {}: {err}",
                                        path.display()
                                    )));
                                }
                                // Last liveness re-check before removing anything:
                                // a probe that answered "dead" must not let a live
                                // writer's lock be deleted. A third answer that is
                                // not a clear "gone" aborts the steal.
                                if pid_alive(owner.pid) != Some(false) {
                                    let _ = fs::remove_file(&marker);
                                    continue;
                                }
                                // We are the only stealer for this generation.
                                // Remove only the lock whose nonce we observed;
                                // a successor cannot have taken the name while
                                // the dead record was still present.
                                match fs::read_to_string(&path) {
                                    Ok(again)
                                        if parse(&again).map(|o| o.nonce)
                                            == Some(owner.nonce.clone()) =>
                                    {
                                        match fs::remove_file(&path) {
                                            Ok(()) => {
                                                let _ = fs::remove_file(&marker);
                                                continue;
                                            }
                                            Err(err) => {
                                                let _ = fs::remove_file(&marker);
                                                return Err(LockError::Io(format!(
                                                    "cannot remove the dead lock {}: {err}",
                                                    path.display()
                                                )));
                                            }
                                        }
                                    }
                                    _ => {
                                        let _ = fs::remove_file(&marker);
                                        continue;
                                    }
                                }
                            }
                            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                                // Another process owns this recovery. Wait for it
                                // on a dedicated window: the acquire budget alone
                                // is too short for a burst of eight publishers.
                                let now = Instant::now();
                                let since = *steal_waited.get_or_insert(now);
                                if now.duration_since(since) >= STEAL_GRACE {
                                    return Err(LockError::Unknown(format!(
                                        "lock {} has a provably dead owner but its recovery is already claimed by {}: the claiming process did not finish within {}s. Inspect both files and remove them by hand only if no flow process is running",
                                        path.display(),
                                        marker.display(),
                                        STEAL_GRACE.as_secs()
                                    )));
                                }
                            }
                            Err(err) => {
                                // A transient sharing/AV denial is not a reason
                                // to fail a publisher: retry within the budget.
                                if Instant::now() >= deadline {
                                    return Err(LockError::Io(format!(
                                        "cannot record the steal of {}: {err}",
                                        path.display()
                                    )));
                                }
                            }
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
pub fn pid_alive(pid: u32) -> Option<bool> {
    // Two consecutive answers are required before concluding "dead": `tasklist`
    // can answer an empty or truncated list under load (the first release run
    // observed exactly that), and a false "dead" would steal a live writer's
    // lock. An answer we do not understand is never proof of death.
    let mut dead_answers = 0;
    for attempt in 0..2 {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        if text.contains(&format!("\"{pid}\"")) || text.contains(&format!(",{pid},")) {
            return Some(true);
        }
        let lower = text.to_ascii_lowercase();
        let definitively_empty = lower.contains("no tasks")
            || lower.contains("aucune t")
            || lower.contains("keine aufgabe")
            || lower.contains("ninguna")
            || lower.contains("nenhuma")
            || lower.contains("nessuna");
        if !definitively_empty {
            return None;
        }
        dead_answers += 1;
        if attempt == 0 {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    // Two definitive "no such task" answers: the pid is gone.
    debug_assert_eq!(dead_answers, 2);
    Some(false)
}

/// Ask `kill -0` and read its message.
///
/// The exit status alone cannot answer the question: it is non-zero both for a
/// process that is gone (ESRCH) and for a live process owned by another user
/// (EPERM). Liveness is only "provable" when the message says the process does
/// not exist; anything else is unknown, and unknown is never stolen.
#[cfg(not(windows))]
fn kill_liveness(pid: u32) -> Option<bool> {
    let out = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    if out.status.success() {
        return Some(true);
    }
    let message = String::from_utf8_lossy(&out.stderr).to_ascii_lowercase();
    if message.contains("no such process") || message.contains("does not exist") {
        Some(false)
    } else {
        None
    }
}

/// On Linux the kernel's own view comes first: `/proc/<pid>` exists exactly while
/// the pid exists, and it cannot confuse "gone" with "not mine". Only when the
/// entry is invisible (hidepid, another PID namespace) does the probe ask `kill`.
#[cfg(target_os = "linux")]
pub fn pid_alive(pid: u32) -> Option<bool> {
    if Path::new(&format!("/proc/{pid}")).exists() {
        return Some(true);
    }
    kill_liveness(pid)
}

#[cfg(all(unix, not(target_os = "linux")))]
pub fn pid_alive(pid: u32) -> Option<bool> {
    kill_liveness(pid)
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
        let host = crate::host::local_host_id().expect("the test host has an identity");
        fs::write(
            lock_path(&root),
            format!(
                "{LOCK_SCHEMA} {} 0 deadbeef publish host:{host}\n",
                std::process::id()
            ),
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

    #[test]
    fn a_state_directory_that_is_a_file_is_an_io_error() {
        // The state directory is created before the first lock attempt. If the
        // path exists as a file, `create_dir_all` fails and the failure has to be
        // reported as an IO refusal: a panic here would take down every command
        // that takes the lock.
        let root = std::env::temp_dir().join(format!("flow-lock-as-file-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".flow"), "not a directory").unwrap();
        match acquire_lock(&root, "publish") {
            Err(LockError::Io(m)) => assert!(m.contains("cannot create .flow"), "{m}"),
            other => panic!("a file where the state directory belongs is an IO error: {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn inspect_attributes_a_lock_to_a_dead_pid() {
        // `flow status` prints this attribution. A reaped child has a PID that no
        // live process holds, so the probe must answer "dead" instead of leaving
        // an operator with a stale lock they cannot interpret.
        let root = temp_flow_dir("dead");
        let mut child = spawn_idle_child();
        let pid = child.id();
        child.kill().ok();
        let _ = child.wait();
        let host = crate::host::local_host_id().expect("the test host has an identity");
        fs::write(
            lock_path(&root),
            format!("{LOCK_SCHEMA} {pid} 0 deadbeef publish host:{host}\n"),
        )
        .unwrap();
        let info = inspect(&root);
        assert!(info.present);
        assert_eq!(info.pid, Some(pid));
        assert_eq!(info.attribution, "dead", "{info:?}");
        let _ = fs::remove_dir_all(&root);
    }

    // The tests below pin the invariants a mutation run found unprotected: the
    // nonce only has to be unique, but nothing observed it; the liveness probe was
    // never exercised for a foreign or a dead PID; and the acquire budget was
    // never measured because the live-owner test took the "it is me" branch.

    /// A child process that stays alive until the test kills it, so its PID is a
    /// provably live PID that is not this process.
    fn spawn_idle_child() -> std::process::Child {
        let mut command = {
            #[cfg(windows)]
            {
                let mut c = std::process::Command::new("ping");
                c.args(["-n", "600", "127.0.0.1"]);
                c
            }
            #[cfg(unix)]
            {
                let mut c = std::process::Command::new("sleep");
                c.arg("600");
                c
            }
        };
        command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the test needs a long-lived child process (ping/sleep)")
    }

    #[test]
    fn a_hostless_or_foreign_record_is_attributed_to_another_host() {
        // A record written before the host token existed cannot be attributed to
        // this machine: its pid space is not comparable, so `inspect` says so
        // instead of claiming the writer is dead.
        let root = temp_flow_dir("hostless");
        fs::write(
            lock_path(&root),
            format!("{LOCK_SCHEMA} {} 0 deadbeef publish\n", std::process::id()),
        )
        .unwrap();
        let info = inspect(&root);
        assert!(info.present);
        assert_eq!(info.attribution, "foreign", "{info:?}");
        assert_eq!(info.pid, Some(std::process::id()));
        // And it is never stolen, even though its pid is our own (which the
        // "this process already holds it" check would otherwise report).
        match acquire_lock(&root, "publish") {
            Err(LockError::Held(m)) => assert!(m.contains("already holds"), "{m}"),
            other => panic!("a host-less record naming our pid is still ours: {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_steal_claimed_by_another_writer_is_waited_for_then_reported_unknown() {
        let root = temp_flow_dir("claimed-steal");
        let host = crate::host::local_host_id().expect("the test host has an identity");
        // A provably dead owner on this host, and a marker that says someone else
        // already owns its recovery. The second writer must not race it.
        let dead = 4_194_304u32;
        fs::write(
            lock_path(&root),
            format!("{LOCK_SCHEMA} {dead} 0 deadbeef publish host:{host}\n"),
        )
        .unwrap();
        fs::write(
            steal_marker_path(&lock_path(&root), "deadbeef"),
            format!("{LOCK_SCHEMA} steal 1 0 other deadbeef\n"),
        )
        .unwrap();
        let started = Instant::now();
        let result = acquire_lock(&root, "publish");
        match result {
            Err(LockError::Unknown(m)) => assert!(m.contains("already claimed"), "{m}"),
            other => panic!("a claimed recovery is never raced: {other:?}"),
        }
        assert!(
            started.elapsed() >= STEAL_GRACE * 9 / 10,
            "the second stealer must wait for the claimant: {:?}",
            started.elapsed()
        );
        assert!(
            lock_path(&root).is_file(),
            "a refused steal leaves the dead record in place"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(not(windows))]
    #[test]
    fn kill_liveness_answers_a_successful_probe_for_the_caller_group() {
        // `kill -0 0` names the caller's own process group: a successful probe.
        // This is the only portable way to observe the "probe says alive" branch
        // without a /proc entry, and it proves the branch is not dead code.
        assert_eq!(kill_liveness(0), Some(true));
        // A pid that no process holds is provably gone: only that message is
        // allowed to mean "dead".
        assert_eq!(kill_liveness(4_194_304), Some(false));
    }

    #[cfg(unix)]
    #[test]
    fn a_state_directory_that_cannot_be_written_is_an_io_refusal() {
        use std::os::unix::fs::PermissionsExt;
        let root = temp_flow_dir("read-only");
        let flow = root.join(".flow");
        let mut perms = fs::metadata(&flow).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&flow, perms).unwrap();
        let result = acquire_lock(&root, "publish");
        // Running as root ignores the permission bits; skip rather than lie.
        if fs::write(flow.join("probe"), b"").is_err() {
            match result {
                Err(LockError::Io(m)) => assert!(m.contains("cannot create"), "{m}"),
                other => panic!("an unwritable state directory is an IO refusal: {other:?}"),
            }
        } else {
            println!("SKIP: this user can write to a 0o555 directory (running as root)");
        }
        let _ = fs::remove_file(flow.join("probe"));
        let mut perms = fs::metadata(&flow).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&flow, perms).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_lock_error_renders_its_message() {
        // A mutant that makes any of these render as "" would print an empty
        // error on the CLI: the operator would get no reason and no file name.
        let held = LockError::Held("held-by-pid-42".into());
        let unknown = LockError::Unknown("unknown-lock-shape".into());
        let io = LockError::Io("cannot-create-.flow".into());
        for (error, expected) in [
            (&held, "held-by-pid-42"),
            (&unknown, "unknown-lock-shape"),
            (&io, "cannot-create-.flow"),
        ] {
            assert_eq!(error.message(), expected);
            assert_eq!(error.to_string(), expected, "Display must not lose it");
        }
        assert_eq!(String::from(held), "held-by-pid-42");
        assert_eq!(String::from(unknown), "unknown-lock-shape");
        assert_eq!(String::from(io), "cannot-create-.flow");
    }

    #[test]
    fn nonces_are_unique_and_file_safe() {
        let first = nonce();
        let second = nonce();
        assert_ne!(first, second, "two lock writers must never share a nonce");
        for value in [&first, &second] {
            assert!(!value.is_empty());
            assert!(
                value.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
                "{value} must stay safe inside a temporary file name"
            );
            assert_eq!(
                value.split('-').count(),
                3,
                "{value} must carry pid, timestamp and counter"
            );
        }
    }

    #[test]
    fn liveness_is_answered_for_live_and_dead_pids() {
        let mut child = spawn_idle_child();
        let pid = child.id();
        assert_eq!(pid_alive(pid), Some(true), "a running child is alive");
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(
            pid_alive(pid),
            Some(false),
            "a reaped child is provably dead, never unknown"
        );
    }

    #[test]
    fn a_live_foreign_owner_is_waited_for_then_reported_held() {
        let root = temp_flow_dir("foreign-live");
        let mut child = spawn_idle_child();
        fs::write(
            lock_path(&root),
            format!("{LOCK_SCHEMA} {} 0 feedface publish\n", child.id()),
        )
        .unwrap();
        let started = Instant::now();
        let result = acquire_lock(&root, "publish");
        let elapsed = started.elapsed();
        let _ = child.kill();
        let _ = child.wait();
        match result {
            Err(LockError::Held(m)) => assert!(m.contains("another flow process"), "{m}"),
            other => panic!("a live foreign owner must not be displaced: {other:?}"),
        }
        assert!(
            elapsed >= ACQUIRE_BUDGET * 9 / 10,
            "a live owner must be waited for, not refused instantly: {elapsed:?}"
        );
        assert!(
            lock_path(&root).is_file(),
            "refusal must not delete a live owner's lock"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_guard_never_removes_a_successors_lock() {
        let root = temp_flow_dir("successor");
        let guard = acquire_lock(&root, "publish").unwrap();
        // A successor replaced the record while the guard was still alive.
        fs::write(
            lock_path(&root),
            format!("{LOCK_SCHEMA} {} 0 cafebabe publish\n", std::process::id()),
        )
        .unwrap();
        drop(guard);
        assert!(
            lock_path(&root).is_file(),
            "a slow predecessor must not delete its successor's lock"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_record_written_after_the_first_read_is_still_attributed() {
        let root = temp_flow_dir("late-writer");
        let path = lock_path(&root);
        fs::write(&path, "").unwrap();
        let late = path.clone();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            fs::write(
                &late,
                format!("{LOCK_SCHEMA} {} 0 abcdef publish\n", std::process::id()),
            )
            .unwrap();
        });
        let owner =
            wait_for_record(&path, String::new()).expect("a late record must still be attributed");
        writer.join().unwrap();
        assert_eq!(owner.pid, std::process::id());
        assert_eq!(owner.nonce, "abcdef");
        let _ = fs::remove_dir_all(&root);
    }

    // The tests below pin the release paths a predecessor or a crash can create:
    // a guard whose file vanished, a lock path that cannot hold a record, and a
    // record that never becomes readable. None of them may panic, and none of
    // them may be reported as an owned lock.

    #[test]
    fn a_guard_whose_file_vanished_is_dropped_without_panicking() {
        let root = temp_flow_dir("vanished");
        let guard = acquire_lock(&root, "publish").unwrap();
        fs::remove_file(lock_path(&root)).unwrap();
        drop(guard);
        assert!(
            !lock_path(&root).exists(),
            "dropping a guard must not recreate anything"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_lock_path_that_cannot_hold_a_record_is_unattributable_not_stolen() {
        let root = temp_flow_dir("directory");
        // A directory where the record belongs: the link fails, then the read
        // fails for a reason that is not "it disappeared".
        fs::create_dir(lock_path(&root)).unwrap();
        match acquire_lock(&root, "publish") {
            Err(LockError::Unknown(m)) => assert!(
                m.contains("cannot be established"),
                "the refusal must ask for a human: {m}"
            ),
            other => panic!("an unattributable lock is never stolen: {other:?}"),
        }
        assert!(
            lock_path(&root).is_dir(),
            "the refusal must preserve whatever is on disk"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_record_that_disappears_while_waiting_is_not_attributed() {
        let root = temp_flow_dir("disappear");
        let path = lock_path(&root);
        fs::write(&path, "").unwrap();
        let late = path.clone();
        let remover = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let _ = fs::remove_file(&late);
        });
        let owner = wait_for_record(&path, String::new());
        remover.join().unwrap();
        assert!(
            owner.is_none(),
            "a record that vanished cannot be attributed to anyone"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_record_is_never_attributed_however_long_we_wait() {
        let root = temp_flow_dir("empty-timeout");
        let path = lock_path(&root);
        fs::write(&path, "").unwrap();
        let started = Instant::now();
        let owner = wait_for_record(&path, String::new());
        assert!(owner.is_none(), "an empty record names no owner");
        assert!(
            started.elapsed() >= CREATE_GRACE * 9 / 10,
            "the reader must give a concurrent creator its grace period: {:?}",
            started.elapsed()
        );
        let _ = fs::remove_dir_all(&root);
    }
}
