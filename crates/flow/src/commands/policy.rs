//! Policy identity, validation and the effective-policy digest.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::gitcli::{self, Git, Obligation};
use crate::json::Json;
use crate::sha256;
use crate::state::{self, Config};

// ------------------------------------------------------- policy identity
pub(crate) fn obligation_json(o: &Obligation) -> Json {
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
}

/// The effective verification policy: every input that changes what a `PASSED`
/// means. Anything that can silently weaken the gate belongs here.
pub(crate) fn effective_policy(config: &Config, obligations: &[Obligation]) -> Json {
    Json::obj(vec![
        ("policy_version", Json::str(config.policy_version.clone())),
        (
            "execution_boundary",
            Json::str(config.execution_boundary.clone()),
        ),
        (
            "obligations",
            Json::Arr(obligations.iter().map(obligation_json).collect()),
        ),
        (
            "obligation_env",
            Json::Arr(
                gitcli::env_allowlist()
                    .iter()
                    .map(|n| Json::str(*n))
                    .collect(),
            ),
        ),
        // A declared probe is an arbitrary program Flow executes between `run`
        // and `publish`; it changes what a `PASSED` receipt means, so it belongs
        // to the approved policy, not only to the context. Without this, adding a
        // probe to a cloned `.flow/config.json` would execute code that was never
        // approved (found by the adversarial pre-review).
        (
            "context_probes",
            Json::Arr(
                config
                    .context_probes
                    .iter()
                    .map(|probe| Json::Arr(probe.iter().map(|a| Json::str(a.clone())).collect()))
                    .collect(),
            ),
        ),
    ])
}

pub(crate) fn policy_digest(policy: &Json) -> String {
    sha256::sha256_hex(policy.to_string().as_bytes())
}

/// Resolve a declared program the way the obligation runner will: an explicit
/// relative/absolute path is used as given, a bare name is searched on `PATH`
/// (with `PATHEXT` on Windows). `None` means the program was not found.
pub(crate) fn resolve_program_path(program: &str) -> Option<PathBuf> {
    let as_path = Path::new(program);
    let explicit = as_path.is_absolute() || program.contains('/') || program.contains('\\');
    if explicit {
        return if as_path.is_file() {
            Some(
                as_path
                    .canonicalize()
                    .unwrap_or_else(|_| as_path.to_path_buf()),
            )
        } else {
            None
        };
    }
    let path = std::env::var_os("PATH")?;
    #[cfg(windows)]
    let extensions: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_ascii_lowercase())
        .collect();
    for dir in std::env::split_paths(&path) {
        #[cfg(windows)]
        {
            let mut candidates: Vec<PathBuf> = Vec::new();
            if as_path.extension().is_some() {
                candidates.push(dir.join(program));
            } else {
                for extension in &extensions {
                    candidates.push(dir.join(format!("{program}{extension}")));
                }
                candidates.push(dir.join(program));
            }
            for candidate in candidates {
                if candidate.is_file() {
                    return Some(candidate.canonicalize().unwrap_or(candidate));
                }
            }
        }
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            let candidate = dir.join(program);
            if candidate.is_file() {
                if let Ok(meta) = candidate.metadata() {
                    if meta.permissions().mode() & 0o111 != 0 {
                        return Some(candidate.canonicalize().unwrap_or(candidate));
                    }
                }
            }
        }
    }
    None
}

/// Identity of a declared program: its resolved path and the digest of the
/// bytes the obligation will execute. A replaced or upgraded binary changes the
/// context, so a receipt written against the old one is refused at publish.
pub(crate) fn program_identity(program: &str) -> Json {
    let mut pairs = vec![("program", Json::str(program.to_string()))];
    match resolve_program_path(program) {
        Some(path) => {
            pairs.push(("resolved", Json::str(path.to_string_lossy().into_owned())));
            let digest = fs::metadata(&path).ok().and_then(|meta| {
                if meta.len() > 256 * 1024 * 1024 {
                    Some(format!("TOO_LARGE:{}", meta.len()))
                } else {
                    fs::read(&path).ok().map(|bytes| sha256::sha256_hex(&bytes))
                }
            });
            pairs.push((
                "file_sha256",
                match digest {
                    Some(digest) => Json::str(digest),
                    None => Json::Null,
                },
            ));
        }
        None => {
            pairs.push(("resolved", Json::Null));
            pairs.push(("file_sha256", Json::Null));
        }
    }
    Json::obj(pairs)
}

/// The allowlisted environment, by name and value digest. The values themselves
/// are never written: a path can still identify a machine, and the context is
/// often copied into agent logs.
pub(crate) fn env_identity() -> Json {
    let entries: Vec<Json> = gitcli::env_allowlist()
        .iter()
        .map(|name| {
            let digest = match std::env::var_os(name) {
                Some(value) => format!(
                    "sha256:{}",
                    sha256::sha256_hex(value.to_string_lossy().as_bytes())
                ),
                None => "ABSENT".to_string(),
            };
            Json::obj(vec![
                ("name", Json::str((*name).to_string())),
                ("value_digest", Json::str(digest)),
            ])
        })
        .collect();
    Json::Arr(entries)
}

/// Declared toolchain probes, hashed. A probe is a policy-controlled command
/// (`rustc -Vv`, `cargo -V`); it runs under the same allowlisted environment as
/// an obligation and only its digest enters the context.
pub(crate) fn probe_identity(config: &Config, root: &Path) -> Json {
    let probes: Vec<Json> = config
        .context_probes
        .iter()
        .map(|probe| {
            let Some(program) = probe.first() else {
                return Json::obj(vec![
                    ("probe", Json::Arr(Vec::new())),
                    ("status", Json::str("INVALID")),
                ]);
            };
            let obligation = Obligation {
                id: "context-probe".to_string(),
                program: program.clone(),
                args: probe[1..].to_vec(),
                required: false,
                timeout_s: 30,
            };
            let outcome = gitcli::run_obligation(&obligation, root);
            Json::obj(vec![
                (
                    "probe",
                    Json::Arr(probe.iter().map(|s| Json::str(s.clone())).collect()),
                ),
                ("status", Json::str(outcome.status)),
                (
                    "exit",
                    outcome
                        .exit
                        .map(|c| Json::Num(c as f64))
                        .unwrap_or(Json::Null),
                ),
                (
                    "stdout_sha256",
                    Json::str(sha256::sha256_hex(outcome.stdout.as_bytes())),
                ),
                (
                    "stderr_sha256",
                    Json::str(sha256::sha256_hex(outcome.stderr.as_bytes())),
                ),
            ])
        })
        .collect();
    Json::Arr(probes)
}

pub(crate) fn build_context_id(
    git: &Git,
    tree: &str,
    policy: &Json,
    config: &Config,
    root: &Path,
) -> String {
    let programs: Vec<Json> = config
        .obligations
        .iter()
        .map(|obligation| program_identity(&obligation.program))
        .collect();
    let context = Json::obj(vec![
        ("codec", Json::str(state::CONTEXT_CODEC)),
        ("tree_oid", Json::str(tree)),
        ("policy", policy.clone()),
        ("git_version", Json::str(git.version())),
        ("os", Json::str(std::env::consts::OS)),
        ("arch", Json::str(std::env::consts::ARCH)),
        ("env", env_identity()),
        ("programs", Json::Arr(programs)),
        ("probes", probe_identity(config, root)),
    ]);
    sha256::sha256_hex(context.to_string().as_bytes())
}

/// Refuse a policy that cannot express a meaningful proof, or whose ids would
/// collide as log file names.
pub(crate) fn validate_obligations(obligations: &[Obligation]) -> Result<(), String> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut folded: BTreeMap<String, String> = BTreeMap::new();
    for ob in obligations {
        crate::paths::validate_relative(&format!(".flow/logs/{}", ob.id))
            .map_err(|reason| format!("unsafe obligation id {:?}: {reason}", ob.id))?;
        if !seen.insert(ob.id.as_str()) {
            return Err(format!(
                "duplicate obligation id {:?}: ids key the logs and the receipt, so they must be unique",
                ob.id
            ));
        }
        // Log files are named after the id. On a case-insensitive filesystem two
        // ids differing only by case would write to one file and lose evidence,
        // so the collision is refused on every platform.
        if let Some(other) = folded.get(&ob.id.to_lowercase()) {
            return Err(format!(
                "obligation ids {:?} and {:?} differ only by case: their log files would collide on a case-insensitive filesystem",
                other, ob.id
            ));
        }
        folded.insert(ob.id.to_lowercase(), ob.id.clone());
    }
    if !obligations.iter().any(|o| o.required) {
        return Err(
            "the policy declares no required obligation: nothing would be proven, so no receipt is publishable"
                .into(),
        );
    }
    Ok(())
}
