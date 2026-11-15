//! Stable, non-secret identity of the machine that writes a lock record.
//!
//! It is not a security boundary: a local writer can write any token. It only
//! answers "was this lock written by a process of this machine?". A record
//! without a token, or with another machine's token, is never stolen: PID spaces
//! across hosts are incomparable.

use std::sync::OnceLock;

use crate::sha256;

pub fn local_host_id() -> Option<String> {
    let name = host_name()?;
    let digest =
        sha256::sha256_hex(format!("flow-host/1\n{name}\n{}", std::env::consts::OS).as_bytes());
    Some(digest[..16].to_string())
}

fn host_name() -> Option<String> {
    static CACHE: OnceLock<Option<String>> = OnceLock::new();
    CACHE.get_or_init(discover_host_name).clone()
}

fn discover_host_name() -> Option<String> {
    for variable in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(variable) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    #[cfg(unix)]
    {
        // CI runners and minimal containers often export no host variable; the
        // `hostname` utility is the portable fallback, and `/proc` covers a
        // Linux host whose PATH lacks it.
        if let Ok(out) = std::process::Command::new("hostname").output() {
            if out.status.success() {
                let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        if let Ok(value) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}
