//! Seeded, deterministic mutation fuzzer for the crate's JSON parser
//! (`crates/flow/src/json.rs`), reached through `#[path]` so the exact shipped
//! parser is exercised with no cargo project and no dependency.
//!
//! Build and run (from the repository root):
//!
//! ```text
//! rustc --edition 2021 -O experiments/quality/fuzz/json_fuzz.rs -o json_fuzz.exe
//! json_fuzz.exe --seconds 60                      # default seed 20261110
//! json_fuzz.exe --seed 7 --seconds 60
//! json_fuzz.exe --iters 200000 --seed 20261110     # exact, replayable prefix
//! json_fuzz.exe --file experiments/quality/fuzz/crashes/00012.json
//! ```
//!
//! What it covers, exactly:
//! * the corpus-driven mutation loop over `json::parse` (chars-level mutations:
//!   insert / delete / replace / truncate / duplicate / splice / token injection);
//! * on every input: no panic, no non-termination (the whole run is time-boxed),
//!   and, for an accepted input, that printing and re-parsing it round-trips;
//! * for a refused input: the error message is non-empty.
//!
//! What it does **not** cover: allocation limits, `f64` parsing correctness beyond
//! self-consistency, memory safety (safe Rust), or any consumer of `Json`.
//!
//! `cargo-fuzz` needs a nightly target directory and libFuzzer; this harness is
//! hand-rolled and deterministic on purpose, so a seed replays a known prefix.

#![allow(dead_code)]

#[path = "../../../crates/flow/src/json.rs"]
mod json;

use std::collections::HashSet;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

    fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        let index = self.below(xs.len());
        &xs[index]
    }
}

// ------------------------------------------------------------------- corpus

const SEEDS: &[&str] = &[
    r#"{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","obligations":[{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}]}"#,
    r#"{"a":1,"b":-0.5,"c":1e10,"d":null,"e":true,"f":false}"#,
    r#"{"nested":{"deep":{"deeper":[1,2,3,{"x":"y"}]}}}"#,
    r#"["a",42,true,null,{"k":"v"}]"#,
    r#"{"escapes":"\n\r\t\"\\\/\b\f\u0041\u00e9\u4e2d"}"#,
    r#"{"unicode":"é中🦀","empty":"","zero":0}"#,
    r#"{"duplicate":1,"duplicate":2}"#,
    r#"{"truncated":[1,2"#,
    r#"{"trailing":true} gARBAGE"#,
    r#"{"bad_escape":"\x"}"#,
    r#"{"surrogate":"\ud800"}"#,
    r#"{"number":1.,"exponent":1e,"sign":-}"#,
    r#"{}"#,
    r#"[]"#,
    r#"null"#,
    r#"{"a":{"b":{"c":{"d":{"e":[]}}}}}"#,
];

const TOKENS: &[&str] = &[
    "{", "}", "[", "]", "\"", ":", ",", "\\", "\\u", "u", "0", "9", "x", "n", "t", "f", "-", "+",
    "e", ".", " ", "\n", "\t", "\u{0}", "\u{1}", "é", "🦀", "/", "b", "r", "N", "a", "null",
    "true", "false", "1e999", "-", "00", "\"\"", "\\ud800", "\\uZZZZ", "[]", "{}",
];

const CHARS: &[char] = &[
    '{', '}', '[', ']', '"', ':', ',', '\\', 'u', '0', '9', 'x', 'n', 't', 'f', '-', '+', 'e',
    '.', ' ', '\n', '\t', '\u{0}', '\u{1}', 'é', '🦀', '/', 'b', 'r', 'a', 'N',
];

// ---------------------------------------------------------------- invariants

struct Failure {
    kind: &'static str,
    detail: String,
    input: String,
    iteration: u64,
}

fn check(text: &str) -> Option<Failure> {
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| json::parse(text)));
    let parsed = match outcome {
        Err(_) => {
            return Some(Failure {
                kind: "panic",
                detail: "json::parse panicked".into(),
                input: text.to_string(),
                iteration: 0,
            })
        }
        Ok(Err(message)) => {
            if message.is_empty() {
                return Some(Failure {
                    kind: "empty-error",
                    detail: "refusal carried an empty message".into(),
                    input: text.to_string(),
                    iteration: 0,
                });
            }
            return None;
        }
        Ok(Ok(value)) => value,
    };
    // Accepting an input implies the printer must reproduce a parseable value,
    // and re-printing must be canonical (Flow writes receipts with this printer).
    let compact = parsed.to_string();
    match json::parse(&compact) {
        Err(err) => Some(Failure {
            kind: "round-trip-error",
            detail: format!("accepted input printed {compact:?}, which does not re-parse: {err}"),
            input: text.to_string(),
            iteration: 0,
        }),
        Ok(again) => {
            if again != parsed || again.to_string() != compact {
                Some(Failure {
                    kind: "round-trip-mismatch",
                    detail: format!("printing is not canonical for accepted input {text:?}"),
                    input: text.to_string(),
                    iteration: 0,
                })
            } else {
                let pretty = parsed.to_string_pretty();
                match json::parse(&pretty) {
                    Ok(pretty_again) if pretty_again == parsed => None,
                    _ => Some(Failure {
                        kind: "pretty-round-trip",
                        detail: format!("pretty printing {pretty:?} does not round-trip"),
                        input: text.to_string(),
                        iteration: 0,
                    }),
                }
            }
        }
    }
}

// ---------------------------------------------------------------- mutations

fn mutate(rng: &mut Rng, corpus: &[String]) -> String {
    let source = &corpus[rng.below(corpus.len())];
    let mut chars: Vec<char> = source.chars().collect();
    let rounds = 1 + rng.below(6);
    for _ in 0..rounds {
        if chars.is_empty() {
            chars.push(*rng.pick(CHARS));
            continue;
        }
        match rng.below(9) {
            0 => {
                let at = rng.below(chars.len() + 1);
                chars.insert(at, *rng.pick(CHARS));
            }
            1 => {
                let at = rng.below(chars.len());
                chars.remove(at);
            }
            2 => {
                let at = rng.below(chars.len());
                chars[at] = *rng.pick(CHARS);
            }
            3 => {
                // inject a whole token, which reaches deeper grammar states than
                // single-character edits
                let at = rng.below(chars.len() + 1);
                let token: Vec<char> = rng.pick(TOKENS).chars().collect();
                for (offset, ch) in token.into_iter().enumerate() {
                    chars.insert(at + offset, ch);
                }
            }
            4 => {
                let at = rng.below(chars.len());
                chars.truncate(at);
            }
            5 => {
                let at = rng.below(chars.len());
                let len = 1 + rng.below(chars.len() - at);
                let slice: Vec<char> = chars[at..at + len].to_vec();
                let into = rng.below(chars.len() + 1);
                for (offset, ch) in slice.into_iter().enumerate() {
                    chars.insert(into + offset, ch);
                }
            }
            6 => {
                let a = rng.below(chars.len());
                let b = rng.below(chars.len());
                chars.swap(a, b);
            }
            7 => {
                // splice a chunk of a different corpus entry
                let other = &corpus[rng.below(corpus.len())];
                let other_chars: Vec<char> = other.chars().collect();
                if other_chars.len() > 1 {
                    let at = rng.below(other_chars.len() - 1);
                    let len = 1 + rng.below(other_chars.len() - at);
                    let into = rng.below(chars.len() + 1);
                    let slice: Vec<char> = other_chars[at..at + len].to_vec();
                    for (offset, ch) in slice.into_iter().enumerate() {
                        chars.insert(into + offset, ch);
                    }
                }
            }
            _ => {
                // duplicate the whole buffer with a one-char alteration: grows
                // nested structures without exploding
                let at = rng.below(chars.len());
                chars[at] = *rng.pick(CHARS);
            }
        }
    }
    chars.into_iter().collect()
}

// --------------------------------------------------------------------- main

fn usage() -> ! {
    eprintln!(
        "usage: json_fuzz --seconds 60 [--seed N] [--iters N] [--out DIR] [--file PATH]\n\
         \x20      --iters takes precedence over --seconds; --file re-parses one saved input"
    );
    std::process::exit(2)
}

fn main() {
    let mut seed: u64 = 20261110;
    let mut seconds: u64 = 60;
    let mut iters: u64 = 0;
    let mut out = PathBuf::from("experiments/quality/fuzz");
    let mut single: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => seed = args.next().expect("--seed needs a value").parse().unwrap(),
            "--seconds" => {
                seconds = args.next().expect("--seconds needs a value").parse().unwrap()
            }
            "--iters" => iters = args.next().expect("--iters needs a value").parse().unwrap(),
            "--out" => out = PathBuf::from(args.next().expect("--out needs a path")),
            "--file" => single = Some(PathBuf::from(args.next().expect("--file needs a path"))),
            _ => usage(),
        }
    }
    if let Some(path) = single {
        let bytes = fs::read(&path).expect("read input");
        let text = String::from_utf8_lossy(&bytes).to_string();
        println!("input={path:?} chars={}", text.chars().count());
        match json::parse(&text) {
            Ok(value) => println!("result=PARSED value={}", value.to_string()),
            Err(err) => println!("result=REFUSED error={err}"),
        }
        return;
    }

    let crash_dir = out.join("crashes");
    let corpus_dir = out.join("corpus");
    let accepted_dir = corpus_dir.join("accepted");
    fs::create_dir_all(&crash_dir).expect("create crash dir");
    fs::create_dir_all(&accepted_dir).expect("create corpus dir");
    println!("SEED={seed}");
    println!("TARGET=json::parse (crates/flow/src/json.rs)");
    println!("OUT={}", out.display());

    let mut corpus: Vec<String> = SEEDS.iter().map(|s| s.to_string()).collect();
    for (index, text) in SEEDS.iter().enumerate() {
        fs::write(
            corpus_dir.join(format!("seed-{index:03}.json")),
            text.as_bytes(),
        )
        .expect("write seed");
    }

    let mut rng = Rng::new(seed);
    let start = Instant::now();
    let mut iterations: u64 = 0;
    let mut accepted: u64 = 0;
    let mut refused: u64 = 0;
    let mut saved_corpus: usize = 0;
    let mut seen: HashSet<u64> = HashSet::new();
    let mut failures: Vec<Failure> = Vec::new();
    let mut signatures: HashSet<String> = HashSet::new();
    let mut saved_findings: usize = 0;
    let budget = Duration::from_secs(seconds);

    loop {
        if iters > 0 {
            if iterations >= iters {
                break;
            }
        } else if start.elapsed() >= budget {
            break;
        }
        let text = mutate(&mut rng, &corpus);
        iterations += 1;
        match check(&text) {
            None => {
                if json::parse(&text).is_ok() {
                    accepted += 1;
                    // Keep inputs that reached a state we have not stored yet, so
                    // the saved corpus is a real artefact, not the seeds again.
                    let hash = fnv1a(&text);
                    if seen.insert(hash) && saved_corpus < 200 {
                        saved_corpus += 1;
                        let name = format!("accepted-{saved_corpus:03}-{hash:016x}.json");
                        fs::write(accepted_dir.join(name), text.as_bytes()).ok();
                        corpus.push(text);
                    }
                } else {
                    refused += 1;
                }
            }
            Some(mut failure) => {
                failure.iteration = iterations;
                // Deduplicate artefacts by defect signature: the same defect reached
                // again from a differently mutated input is one finding.
                let key = signature(failure.kind, &failure.detail);
                if signatures.insert(key) && saved_findings < 20 {
                    saved_findings += 1;
                    let name = format!(
                        "crash-{iterations:08}-{}.json",
                        sanitise(failure.kind)
                    );
                    fs::write(crash_dir.join(&name), failure.input.as_bytes()).ok();
                    eprintln!(
                        "FINDING at iteration {iterations} kind={} detail={} saved={name}",
                        failure.kind, failure.detail
                    );
                    eprintln!(
                        "REPRO: json_fuzz.exe --file {}",
                        crash_dir.join(&name).display()
                    );
                }
                if failures.len() < 50_000 {
                    failures.push(failure);
                }
            }
        }
        if iterations % 5000 == 0 {
            eprintln!(
                "progress iterations={iterations} accepted={accepted} refused={refused} elapsed={:.1}s",
                start.elapsed().as_secs_f64()
            );
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let mut stats = vec![
        ("seed".to_string(), Json::Num(seed as f64)),
        ("target".to_string(), Json::Str("json::parse".into())),
        ("iterations".to_string(), Json::Num(iterations as f64)),
        ("accepted".to_string(), Json::Num(accepted as f64)),
        ("refused".to_string(), Json::Num(refused as f64)),
        ("findings".to_string(), Json::Num(failures.len() as f64)),
        ("distinct_findings".to_string(), Json::Num(signatures.len() as f64)),
        ("elapsed_s".to_string(), Json::Num((elapsed * 1000.0).round() / 1000.0)),
        ("seconds_budget".to_string(), Json::Num(seconds as f64)),
        ("saved_corpus".to_string(), Json::Num(saved_corpus as f64)),
        (
            "note".to_string(),
            Json::Str(
                "seed fixes the input sequence; with --seconds the prefix length depends on machine speed"
                    .into(),
            ),
        ),
    ];
    if let Some(first) = failures.first() {
        stats.push((
            "first_finding".into(),
            Json::obj(vec![
                ("kind", Json::Str(first.kind.to_string())),
                ("detail", Json::Str(first.detail.clone())),
                ("iteration", Json::Num(first.iteration as f64)),
                ("input", Json::Str(first.input.clone())),
            ]),
        ));
    }
    let stats_json = Json::Obj(stats);
    fs::write(out.join("stats.json"), stats_json.to_string_pretty()).expect("write stats");
    println!("ITERATIONS={iterations}");
    println!("ACCEPTED={accepted}");
    println!("REFUSED={refused}");
    println!("FINDINGS={}", failures.len());
    println!("ELAPSED_S={elapsed:.3}");
    println!(
        "RESULT={}",
        if failures.is_empty() { "PASSED" } else { "FAILED" }
    );
    if !failures.is_empty() {
        std::process::exit(1);
    }
}

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

fn sanitise(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}

/// Coarse defect classification: one artefact per class, not per mutated input.
fn signature(kind: &str, detail: &str) -> String {
    for token in ["-inf", "inf", "NaN"] {
        if detail.contains(token) {
            return format!("{kind}|non-finite-number:{token}");
        }
    }
    let head: String = detail.chars().take(80).collect();
    format!("{kind}|{head}")
}

#[allow(dead_code)]
fn ensure_dir(path: &Path) {
    fs::create_dir_all(path).expect("create dir");
}
