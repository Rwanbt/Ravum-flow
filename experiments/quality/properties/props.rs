//! Seeded property harness for Ravum Flow — standalone, no dependency, no cargo
//! project membership.
//!
//! Compiled with the crate's own modules reached through `#[path]` (exactly the
//! mechanism `crates/flow/tests/cli.rs` already uses for `json.rs`):
//!
//! ```text
//! rustc --edition 2021 -O experiments/quality/properties/props.rs -o props.exe
//! props.exe --flow target\debug\flow.exe            (default seed 20261110)
//! props.exe --flow target\debug\flow.exe --seed 7
//! ```
//!
//! `experiments/` is excluded from the workspace, so this file is never built by
//! `cargo test` or CI; it cannot add a dependency to `crates/flow`.
//!
//! Properties:
//!   P1  JSON printing/parsing round-trips for generated values (compact + pretty)
//!   P2  duplicate object names are refused, at any nesting depth
//!   P3  `paths::validate_relative` never accepts `..`, an absolute path, NUL /
//!       control bytes, an empty path, or a reserved device name
//!   P4  every path it *does* accept satisfies the safety invariants (no `.`/`..`
//!       component, no empty component, no `:`, no trailing dot/space, no
//!       reserved stem, not absolute, <= 4096 bytes, no control byte)
//!   P5  proposal-id derivation is stable and collision-resistant over 100 000
//!       generated inputs, and the documented formula matches the real CLI
//!   P6  driving the CLI: assembly order changes the candidate id but not the
//!       tree of two disjoint proposals
//!
//! Every generator is a seeded SplitMix64; the seed is printed. Re-running the
//! same seed reproduces the same input sequence.

#![allow(dead_code)]

#[path = "../../../crates/flow/src/json.rs"]
mod json;
#[path = "../../../crates/flow/src/paths.rs"]
mod paths;
#[path = "../../../crates/flow/src/sha256.rs"]
mod sha256;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use json::Json;

// ------------------------------------------------------------------ seeding

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        let index = self.below(xs.len() as u64) as usize;
        &xs[index]
    }

    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.below(den) < num
    }
}

// ---------------------------------------------------------------- reporting

struct Report {
    failures: Vec<String>,
    checks: Vec<(String, String)>,
}

impl Report {
    fn new() -> Self {
        Report {
            failures: Vec::new(),
            checks: Vec::new(),
        }
    }

    fn pass(&mut self, name: &str, detail: String) {
        println!("[PASS] {name}: {detail}");
        self.checks.push((name.to_string(), detail));
    }

    fn fail(&mut self, name: &str, detail: String) {
        println!("[FAIL] {name}: {detail}");
        self.failures.push(format!("{name}: {detail}"));
    }

    fn require(&mut self, name: &str, condition: bool, detail: String) {
        if condition {
            self.pass(name, detail);
        } else {
            self.fail(name, detail);
        }
    }
}

// ------------------------------------------------------------------- P1/P2

const STRING_ALPHABET: &[char] = &[
    'a', 'z', '0', '9', ' ', '/', '\\', ':', '.', '-', '_', '~', '"', '\'', '\\', '\n', '\r', '\t',
    '\u{0}', '\u{1}', '\u{1f}', 'é', '中', '🦀', '\u{7f}', '\u{85}', '\u{a0}', '\u{2028}',
    '\u{feff}',
];

const NUMBER_POOL: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    42.5,
    -0.125,
    1e-7,
    123456789.0,
    9007199254740992.0,
    9007199254740993.0,
    1e21,
    1e300,
    -1e300,
    f64::MAX,
    f64::MIN_POSITIVE,
    0.1 + 0.2,
];

fn gen_string(rng: &mut Rng) -> String {
    let len = rng.below(12) as usize;
    let mut out = String::new();
    for _ in 0..len {
        out.push(*rng.pick(STRING_ALPHABET));
    }
    out
}

fn gen_json(rng: &mut Rng, depth: u32) -> Json {
    let choice = if depth == 0 {
        rng.below(5)
    } else {
        rng.below(7)
    };
    match choice {
        0 => Json::Null,
        1 => Json::Bool(rng.chance(1, 2)),
        2 => Json::Num(*rng.pick(NUMBER_POOL)),
        3 | 4 => Json::Str(gen_string(rng)),
        5 => {
            let n = rng.below(5);
            let mut items = Vec::new();
            for _ in 0..n {
                items.push(gen_json(rng, depth - 1));
            }
            Json::Arr(items)
        }
        _ => {
            let n = rng.below(5);
            let mut pairs: Vec<(String, Json)> = Vec::new();
            let mut used: HashSet<String> = HashSet::new();
            for _ in 0..n {
                let mut key = gen_string(rng);
                let mut guard = 0;
                while used.contains(&key) {
                    key.push('k');
                    guard += 1;
                    if guard > 16 {
                        key = format!("{key}-{guard}-{n}");
                    }
                }
                used.insert(key.clone());
                pairs.push((key, gen_json(rng, depth - 1)));
            }
            Json::Obj(pairs)
        }
    }
}

fn p1_json_round_trip(report: &mut Report, rng: &mut Rng) {
    let trials = 2000;
    for i in 0..trials {
        let value = gen_json(rng, 3);
        let compact = value.to_string();
        let pretty = value.to_string_pretty();
        match json::parse(&compact) {
            Err(err) => {
                report.fail(
                    "P1_json_round_trip",
                    format!("trial {i}: compact output did not re-parse: {err}; text={compact:?}"),
                );
                return;
            }
            Ok(parsed) => {
                if parsed != value {
                    report.fail(
                        "P1_json_round_trip",
                        format!(
                            "trial {i}: compact round-trip changed the value; text={compact:?}"
                        ),
                    );
                    return;
                }
                // Printing is canonical: re-printing the parse of our own output
                // must be byte-identical.
                if parsed.to_string() != compact {
                    report.fail(
                        "P1_json_round_trip",
                        format!("trial {i}: printing is not canonical for {compact:?}"),
                    );
                    return;
                }
            }
        }
        match json::parse(&pretty) {
            Err(err) => {
                report.fail(
                    "P1_json_round_trip",
                    format!("trial {i}: pretty output did not re-parse: {err}"),
                );
                return;
            }
            Ok(parsed) => {
                if parsed != value {
                    report.fail(
                        "P1_json_round_trip",
                        format!("trial {i}: pretty round-trip changed the value"),
                    );
                    return;
                }
                if parsed.to_string_pretty() != pretty {
                    report.fail(
                        "P1_json_round_trip",
                        format!("trial {i}: pretty printing is not canonical"),
                    );
                    return;
                }
            }
        }
    }
    report.pass(
        "P1_json_round_trip",
        format!("{trials} generated values round-tripped exactly (compact and pretty)"),
    );
}

fn p2_duplicate_keys_refused(report: &mut Report, rng: &mut Rng) {
    let trials = 1000;
    for i in 0..trials {
        let key = gen_string(rng);
        let first = gen_json(rng, 1);
        let second = gen_json(rng, 1);
        let flat = format!(
            "{{{key}:{first},{key}:{second}}}",
            key = Json::str(key.clone())
        );
        let nested = format!(
            "{{\"outer\":{{\"deep\":{{{key}:{first},{key}:{second}}}}}}}",
            key = Json::str(key.clone())
        );
        // Identical duplicate values are still ambiguous, so they must be refused.
        let identical = format!(
            "{{{key}:{first},{key}:{first}}}",
            key = Json::str(key.clone())
        );
        for (label, text) in [
            ("flat", flat),
            ("nested", nested),
            ("identical", identical),
        ] {
            if json::parse(&text).is_ok() {
                report.fail(
                    "P2_duplicate_keys_refused",
                    format!("trial {i} ({label}): duplicate key was accepted: {text:?}"),
                );
                return;
            }
        }
    }
    report.pass(
        "P2_duplicate_keys_refused",
        format!("{trials} duplicate-name objects refused (flat, nested, identical values)"),
    );
}

// ------------------------------------------------------------------- P3/P4

const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM9", "LPT1", "LPT5", "LPT9",
];

/// A string that is *guaranteed* to be in the dangerous class, built by
/// composition rather than by hoping a random draw lands there.
fn gen_dangerous_path(rng: &mut Rng) -> String {
    let filler = |rng: &mut Rng| -> String {
        let len = 1 + rng.below(8) as usize;
        let mut s = String::new();
        for _ in 0..len {
            s.push(*rng.pick(&['a', 'b', 'x', 'y', '0', '_', '-']));
        }
        s
    };
    match rng.below(9) {
        // parent-directory traversal, alone or embedded
        0 => match rng.below(4) {
            0 => "..".to_string(),
            1 => "..\\..\\etc".to_string(),
            2 => format!("{}/../{}", filler(rng), filler(rng)),
            _ => format!("{}/..", filler(rng)),
        },
        // absolute path
        1 => match rng.below(4) {
            0 => format!("/{}", filler(rng)),
            1 => format!("\\{}", filler(rng)),
            2 => "C:\\Windows\\system32".to_string(),
            _ => format!("z:/{}", filler(rng)),
        },
        // NUL or another control byte
        2 => {
            let c = match rng.below(4) {
                0 => '\u{0}',
                1 => '\u{1}',
                2 => '\u{1f}',
                _ => '\n',
            };
            format!("{}{c}{}", filler(rng), filler(rng))
        }
        // empty, or an empty component
        3 => match rng.below(4) {
            0 => String::new(),
            1 => format!("{}/", filler(rng)),
            2 => format!("/{}", filler(rng)),
            _ => format!("{}//{}", filler(rng), filler(rng)),
        },
        // reserved device name, bare or with an extension and/or a directory
        4 => {
            let name = *rng.pick(RESERVED_NAMES);
            match rng.below(3) {
                0 => name.to_string(),
                1 => format!("{}.txt", name.to_lowercase()),
                _ => format!("dir/{}.log", name),
            }
        }
        // component ending with a dot or a space
        5 => format!("{}{}", filler(rng), *rng.pick(&['.', ' '])),
        // colon / NTFS alternate data stream
        6 => format!("{}:{}", filler(rng), filler(rng)),
        // over-long component or over-long whole path
        7 => match rng.below(2) {
            0 => "a".repeat(256),
            _ => "b".repeat(4097),
        },
        _ => format!("{}/./{}", filler(rng), filler(rng)),
    }
}

fn p3_never_accepts_dangerous(report: &mut Report, rng: &mut Rng) {
    let fixed = [
        "",
        ".",
        "..",
        "../x",
        "..\\x",
        "a/../../b",
        "a/../b",
        "/etc/passwd",
        "\\\\server\\share",
        "C:\\Windows",
        "c:x",
        "a//b",
        "a/",
        "/a",
        "a\\",
        "a\u{0}b",
        "a\u{1f}b",
        "CON",
        "con.txt",
        "AUX.ini",
        "nul",
        "com1.log",
        "LPT9",
        "dir/nul/x",
        "x.",
        "x ",
        "a/b:/c",
        "a:b",
        ".",
    ];
    for bad in fixed {
        match paths::validate_relative(bad) {
            Ok(()) => {
                report.fail(
                    "P3_never_accepts_dangerous",
                    format!("fixed dangerous path {bad:?} was accepted"),
                );
                return;
            }
            Err(reason) => {
                if reason.is_empty() {
                    report.fail(
                        "P3_never_accepts_dangerous",
                        format!("refusal for {bad:?} has an empty reason"),
                    );
                    return;
                }
            }
        }
    }
    let trials = 20_000;
    for i in 0..trials {
        let bad = gen_dangerous_path(rng);
        if paths::validate_relative(&bad).is_ok() {
            report.fail(
                "P3_never_accepts_dangerous",
                format!("generated dangerous path {bad:?} was accepted (trial {i})"),
            );
            return;
        }
    }
    report.pass(
        "P3_never_accepts_dangerous",
        format!(
            "{} fixed cases + {trials} generated dangerous paths all refused",
            fixed.len()
        ),
    );
}

/// Safety invariants that any accepted path must satisfy. This restates the
/// documented rules; it is a conformance check, not an independent oracle.
fn accepted_path_is_safe(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("accepted an empty path".into());
    }
    if path.len() > 4096 {
        return Err("accepted a path longer than 4096 bytes".into());
    }
    if path.bytes().any(|b| b < 0x20) {
        return Err("accepted a path with a control byte".into());
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("accepted an absolute path".into());
    }
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err("accepted a drive-qualified path".into());
    }
    for component in path.split(['/', '\\']) {
        if component.is_empty() {
            return Err("accepted an empty component".into());
        }
        if component == "." || component == ".." {
            return Err(format!("accepted traversal component {component:?}"));
        }
        if component.len() > 255 {
            return Err("accepted an over-long component".into());
        }
        if component.contains(':') {
            return Err("accepted a component with ':'".into());
        }
        if component.ends_with('.') || component.ends_with(' ') {
            return Err("accepted a component ending with a dot or space".into());
        }
        let stem = component
            .split('.')
            .next()
            .unwrap_or(component)
            .to_ascii_uppercase();
        if RESERVED_NAMES.contains(&stem.as_str()) {
            return Err(format!("accepted a reserved device name {component:?}"));
        }
    }
    Ok(())
}

fn p4_acceptance_is_sound(report: &mut Report, rng: &mut Rng) {
    let alphabet: &[char] = &[
        'a', 'b', 'Z', '0', '9', '/', '\\', ':', '.', ' ', '-', '_', '~', 'é', '中', '🦀',
        '\u{0}',
    ];
    let words: &[&str] = &[
        "CON", "con", "aux", "NUL", "com1", "lpt9", "src", "lib", "main.rs", "a", "..", ".",
    ];
    let trials = 20_000;
    let mut accepted = 0usize;
    for i in 0..trials {
        let mut path = String::new();
        let parts = rng.below(5);
        for part in 0..parts {
            if part > 0 {
                path.push(*rng.pick(&['/', '\\']));
            }
            if rng.chance(1, 3) {
                path.push_str(*rng.pick(words));
            } else {
                let len = rng.below(6) as usize;
                for _ in 0..len {
                    path.push(*rng.pick(alphabet));
                }
            }
        }
        if rng.chance(1, 20) {
            path.push_str(*rng.pick(&[".", " ", ":", ".txt", " "]));
        }
        if paths::validate_relative(&path).is_ok() {
            accepted += 1;
            if let Err(violation) = accepted_path_is_safe(&path) {
                report.fail(
                    "P4_acceptance_is_sound",
                    format!("trial {i}: {violation} for accepted path {path:?}"),
                );
                return;
            }
        }
    }
    report.pass(
        "P4_acceptance_is_sound",
        format!("{trials} random paths, {accepted} accepted, none violated an invariant"),
    );
}

// ---------------------------------------------------------------------- P5

fn derive_proposal_id(agent: &str, task: &str, base: &str, key: &str) -> String {
    let material = format!(
        "flow-proposal/1\nagent:{}\ntask:{}\nbase:{}\nkey:{}",
        agent.trim(),
        task.trim(),
        base.trim(),
        key.trim()
    );
    sha256::sha256_hex(material.as_bytes())[..32].to_string()
}

fn gen_ident(rng: &mut Rng) -> String {
    let alphabet: &[char] = &[
        'a', 'b', 'c', 'x', 'y', 'z', 'A', 'B', '0', '1', '7', '9', '-', '_', '.',
    ];
    let len = 1 + rng.below(16) as usize;
    let mut out = String::new();
    for _ in 0..len {
        out.push(*rng.pick(alphabet));
    }
    // `ident` rejects a leading '.', so keep generated identities admissible.
    if out.starts_with('.') {
        out.insert(0, 'a');
    }
    out
}

fn gen_key(rng: &mut Rng) -> String {
    let base = gen_ident(rng);
    match rng.below(4) {
        0 => format!("  {base}"),
        1 => format!("{base}  "),
        2 => format!("\t{base} "),
        _ => base,
    }
}

fn p5_proposal_ids(report: &mut Report, rng: &mut Rng) {
    let trials = 100_000usize;
    let base = "ea181d4c0c1d3f1b9c0e5d2a7f6b4a3c1e0d9f8b";
    let mut seen: HashSet<String> = HashSet::with_capacity(trials * 2);
    for i in 0..trials {
        let agent = gen_ident(rng);
        let task = gen_ident(rng);
        let key = gen_key(rng);
        let id = derive_proposal_id(&agent, &task, base, &key);
        if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            report.fail(
                "P5_proposal_ids",
                format!("input {i}: derived id {id:?} is not 32 hex characters"),
            );
            return;
        }
        if id.bytes().any(|b| b.is_ascii_uppercase()) {
            report.fail(
                "P5_proposal_ids",
                format!("input {i}: derived id {id:?} is not lowercase hex"),
            );
            return;
        }
        // Stability: the same input must derive the same id.
        if derive_proposal_id(&agent, &task, base, &key) != id {
            report.fail(
                "P5_proposal_ids",
                format!("input {i}: derivation is not deterministic"),
            );
            return;
        }
        // Whitespace around an identity is trimmed, so it must not change the id.
        if derive_proposal_id(&format!(" {agent} "), &task, base, &format!("{key}\n")) != id {
            report.fail(
                "P5_proposal_ids",
                format!("input {i}: trimming changed the derived id"),
            );
            return;
        }
        if !seen.insert(id.clone()) {
            report.fail(
                "P5_proposal_ids",
                format!("input {i}: id collision on {id} (agent={agent:?} task={task:?} key={key:?})"),
            );
            return;
        }
    }
    report.pass(
        "P5_proposal_ids",
        format!("{trials} generated inputs: 32 lowercase hex, stable, 0 collisions"),
    );
}

// ------------------------------------------------------------------- P6/P5 CLI

#[derive(Default)]
struct Fixture {
    repo: PathBuf,
    base: String,
    patches: Vec<PathBuf>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = remove_tree(&self.repo);
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
    fn clear(p: &Path) {
        if let Ok(meta) = fs::symlink_metadata(p) {
            let perms = meta.permissions();
            if perms.readonly() {
                let mut writable = perms.clone();
                writable.set_readonly(false);
                let _ = fs::set_permissions(p, writable);
            }
        }
        if p.is_dir() {
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    clear(&entry.path());
                }
            }
        }
    }
    clear(path);
    fs::remove_dir_all(path)
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn git_raw(repo: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

struct Cli {
    binary: PathBuf,
}

impl Cli {
    fn run(&self, repo: &Path, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(&self.binary)
            .current_dir(repo)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("cannot run flow {}: {e}", self.binary.display()));
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        )
    }

    fn json(&self, repo: &Path, args: &[&str]) -> (i32, Json) {
        let (code, stdout, stderr) = self.run(repo, args);
        let value = if stdout.trim().is_empty() {
            Json::Null
        } else {
            json::parse(&stdout).unwrap_or_else(|e| {
                panic!("flow {args:?} printed non-JSON ({e}):\n{stdout}\n{stderr}")
            })
        };
        (code, value)
    }
}

fn field(value: &Json, key: &str) -> String {
    value
        .get(key)
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string()
}

fn build_fixture(cli: &Cli, seed: u64) -> Fixture {
    let dir = std::env::temp_dir().join(format!(
        "flow-quality-props-{}-{seed}",
        std::process::id()
    ));
    let _ = remove_tree(&dir);
    fs::create_dir_all(&dir).expect("temp dir");

    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.name", "Flow Quality"]);
    git(&dir, &["config", "user.email", "flow@example.invalid"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    fs::write(dir.join("alpha.txt"), "one\ntwo\nthree\n").unwrap();
    fs::write(dir.join("shared.txt"), "red\ngreen\nblue\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    let base = git(&dir, &["rev-parse", "HEAD"]);

    let (code, out) = cli.json(&dir, &["init"]);
    assert_eq!(code, 0, "flow init failed: {out}");
    let config = r#"{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}],"disabled":false}"#;
    fs::write(dir.join(".flow").join("config.json"), config).unwrap();
    let (code, out) = cli.json(&dir, &["approve"]);
    assert_eq!(code, 0, "flow approve failed: {out}");

    let mut fixture = Fixture {
        repo: dir,
        base,
        patches: Vec::new(),
    };
    for (name, content) in [
        ("props-a.txt", "a\n"),
        ("props-b.txt", "b\n"),
        ("props-c.txt", "c\n"),
    ] {
        fixture.patches.push(make_patch(&fixture.repo, &fixture.base, name, content, seed));
    }
    fixture
}

fn make_patch(repo: &Path, base: &str, name: &str, content: &str, seed: u64) -> PathBuf {
    let work = std::env::temp_dir().join(format!(
        "flow-quality-props-work-{}-{seed}-{name}",
        std::process::id()
    ));
    let _ = remove_tree(&work);
    git(
        repo,
        &[
            "worktree",
            "add",
            "--detach",
            "-q",
            &work.to_string_lossy(),
            base,
        ],
    );
    fs::write(work.join(name), content).unwrap();
    git(&work, &["add", "-A"]);
    let patch = git_raw(&work, &["diff", "--cached", "--binary", "--no-color", base]);
    let patch_path = std::env::temp_dir().join(format!(
        "flow-quality-props-patch-{}-{seed}-{name}.patch",
        std::process::id()
    ));
    fs::write(&patch_path, patch).unwrap();
    git(repo, &["worktree", "remove", "--force", &work.to_string_lossy()]);
    let _ = remove_tree(&work);
    patch_path
}

fn propose_repo(
    cli: &Cli,
    repo: &Path,
    agent: &str,
    task: &str,
    base: &str,
    patch: &Path,
    key: &str,
) -> (i32, Json) {
    cli.json(
        repo,
        &[
            "propose",
            "--agent",
            agent,
            "--task",
            task,
            "--base",
            base,
            "--patch",
            &patch.to_string_lossy(),
            "--key",
            key,
        ],
    )
}

fn assemble_repo(cli: &Cli, repo: &Path, ids: &[&str]) -> (i32, Json) {
    let mut args: Vec<&str> = vec!["assemble"];
    for id in ids {
        args.push("--proposal");
        args.push(id);
    }
    cli.json(repo, &args)
}

fn p6_assembly_order(cli: &Cli, report: &mut Report, seed: u64) -> Option<Fixture> {
    let fixture = build_fixture(cli, seed);
    let (code, a) = propose_repo(
        cli,
        &fixture.repo,
        "prop-agent-a",
        "T-a",
        &fixture.base,
        &fixture.patches[0],
        "key-a",
    );
    assert_eq!(code, 0, "proposal A: {a}");
    let (code, b) = propose_repo(
        cli,
        &fixture.repo,
        "prop-agent-b",
        "T-b",
        &fixture.base,
        &fixture.patches[1],
        "key-b",
    );
    assert_eq!(code, 0, "proposal B: {b}");
    let (ida, idb) = (field(&a, "proposal_id"), field(&b, "proposal_id"));
    if ida.is_empty() || idb.is_empty() || ida == idb {
        report.fail(
            "P6_assembly_order",
            format!("proposal ids are not distinct/usable: {ida:?} {idb:?}"),
        );
        return Some(fixture);
    }

    let (code, forward) = assemble_repo(cli, &fixture.repo, &[&ida, &idb]);
    let (code2, again) = assemble_repo(cli, &fixture.repo, &[&ida, &idb]);
    let (code3, reversed) = assemble_repo(cli, &fixture.repo, &[&idb, &ida]);
    if code != 0 || code2 != 0 || code3 != 0 {
        report.fail(
            "P6_assembly_order",
            format!("assembly failed: forward={code} again={code2} reversed={code3}"),
        );
        return Some(fixture);
    }
    let (cid, tree) = (field(&forward, "candidate_id"), field(&forward, "tree_oid"));
    let (cid_again, tree_again) = (field(&again, "candidate_id"), field(&again, "tree_oid"));
    let (cid_rev, tree_rev) = (
        field(&reversed, "candidate_id"),
        field(&reversed, "tree_oid"),
    );
    let mut ok = true;
    if cid.is_empty() || tree.is_empty() {
        report.fail(
            "P6_assembly_order",
            format!("assembly did not report a candidate id and tree: {forward}"),
        );
        ok = false;
    }
    if cid != cid_again || tree != tree_again {
        report.fail(
            "P6_assembly_order",
            format!("repeated assembly of the same order is not deterministic: {cid}+{tree} vs {cid_again}+{tree_again}"),
        );
        ok = false;
    }
    if cid_rev == cid {
        report.fail(
            "P6_assembly_order",
            "reversing the order did not change the candidate id".to_string(),
        );
        ok = false;
    }
    if tree_rev != tree {
        report.fail(
            "P6_assembly_order",
            format!("disjoint proposals produced different trees: {tree} vs {tree_rev}"),
        );
        ok = false;
    }
    if ok {
        report.pass(
            "P6_assembly_order",
            format!(
                "order changes candidate_id ({cid} vs {cid_rev}) but not tree_oid ({tree})"
            ),
        );
    }
    Some(fixture)
}

/// The documented derivation formula must describe what the shipped CLI does.
fn p5_cli_agreement(cli: &Cli, fixture: &Fixture, report: &mut Report, rng: &mut Rng, samples: usize) {
    let mut passed = 0usize;
    for i in 0..samples {
        let agent = gen_ident(rng);
        let task = gen_ident(rng);
        let key = format!("cli-key-{i}-{}", gen_ident(rng));
        let (code, out) = propose_repo(
            cli,
            &fixture.repo,
            &agent,
            &task,
            &fixture.base,
            &fixture.patches[2],
            &key,
        );
        if code != 0 {
            report.fail(
                "P5_cli_agreement",
                format!("sample {i}: flow propose failed ({code}): {out}"),
            );
            return;
        }
        let observed = field(&out, "proposal_id");
        let expected = derive_proposal_id(&agent, &task, &fixture.base, &key);
        if observed != expected {
            report.fail(
                "P5_cli_agreement",
                format!("sample {i}: CLI id {observed:?} != documented formula {expected:?}"),
            );
            return;
        }
        if i == 0 {
            let (code, again) = propose_repo(
                cli,
                &fixture.repo,
                &agent,
                &task,
                &fixture.base,
                &fixture.patches[2],
                &key,
            );
            if code != 0
                || field(&again, "proposal_id") != observed
                || again.get("idempotent").and_then(Json::as_bool) != Some(true)
            {
                report.fail(
                    "P5_cli_agreement",
                    "re-submitting the same key is not idempotent".to_string(),
                );
                return;
            }
        }
        passed += 1;
    }
    report.pass(
        "P5_cli_agreement",
        format!("{passed} CLI proposals matched the derived id; resubmission was idempotent"),
    );
}

// --------------------------------------------------------------------- main

fn main() {
    let mut flow_binary = PathBuf::from("target/debug/flow.exe");
    let mut seed: u64 = 20261110;
    let mut samples = 24usize;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--flow" => flow_binary = PathBuf::from(args.next().expect("--flow needs a path")),
            "--seed" => {
                seed = args
                    .next()
                    .expect("--seed needs a value")
                    .parse()
                    .expect("--seed must be a u64")
            }
            "--samples" => {
                samples = args
                    .next()
                    .expect("--samples needs a value")
                    .parse()
                    .expect("--samples must be a usize")
            }
            other => {
                eprintln!("unknown argument {other:?}");
                std::process::exit(2);
            }
        }
    }
    if !flow_binary.exists() {
        eprintln!(
            "flow binary {} does not exist: build it first (cargo build) or pass --flow",
            flow_binary.display()
        );
        std::process::exit(2);
    }

    println!("SEED={seed}");
    println!("FLOW={}", flow_binary.display());
    println!("SAMPLES={samples}");

    let mut report = Report::new();
    // Independent streams per property, so adding a property cannot shift the
    // inputs of an earlier one.
    let mut rng_p1 = Rng::new(seed ^ 0x01);
    let mut rng_p2 = Rng::new(seed ^ 0x02);
    let mut rng_p3 = Rng::new(seed ^ 0x03);
    let mut rng_p4 = Rng::new(seed ^ 0x04);
    let mut rng_p5 = Rng::new(seed ^ 0x05);
    let mut rng_p6 = Rng::new(seed ^ 0x06);

    p1_json_round_trip(&mut report, &mut rng_p1);
    p2_duplicate_keys_refused(&mut report, &mut rng_p2);
    p3_never_accepts_dangerous(&mut report, &mut rng_p3);
    p4_acceptance_is_sound(&mut report, &mut rng_p4);
    p5_proposal_ids(&mut report, &mut rng_p5);

    let cli = Cli { binary: flow_binary };
    let fixture = p6_assembly_order(&cli, &mut report, seed);
    if let Some(fixture) = fixture {
        p5_cli_agreement(&cli, &fixture, &mut report, &mut rng_p6, samples);
    }

    println!(
        "SUMMARY passed={} failed={}",
        report.checks.len(),
        report.failures.len()
    );
    if report.failures.is_empty() {
        println!("RESULT=PASSED");
    } else {
        println!("RESULT=FAILED");
        for failure in &report.failures {
            println!("FAILURE={failure}");
        }
        std::process::exit(1);
    }
}
