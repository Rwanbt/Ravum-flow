# Rust fixture — `cargo` obligations

A dependency-free crate that Flow can verify end to end. It is a verification
target, not a member of the Flow crate workspace: the `[workspace]` table in
`Cargo.toml` deliberately makes it its own workspace root.

## 1. What is here

| Path | Purpose |
|---|---|
| `src/lib.rs` | `add(i64, i64)` plus two unit tests and one doc test |
| `Cargo.toml` | standalone workspace root, no dependencies |
| `Cargo.lock` | committed so `cargo test --locked` is meaningful |
| `.flow/config.json` | the declared policy (`flow-config/1`, `local-trusted`) |
| `.flow/.gitignore` | `*` — Flow's own state never enters the verified tree |

## 2. Declared policy

```json
{
  "schema": "flow-config/1",
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "obligations": [
    { "id": "fmt",    "program": "cargo", "args": ["fmt", "--check"],                  "required": true, "timeout_s": 120 },
    { "id": "clippy", "program": "cargo", "args": ["clippy", "--", "-D", "warnings"],   "required": true, "timeout_s": 300 },
    { "id": "test",   "program": "cargo", "args": ["test", "--locked", "--offline"],    "required": true, "timeout_s": 300 }
  ],
  "disabled": false
}
```

All three are required: a `PASSED` receipt is publishable only when every one of
them ran and passed.

## 3. Exact commands

Run the obligations directly (this is what Flow executes inside the verified
tree, in the same order):

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --locked --offline
```

Run them through Flow from a disposable copy (the drivers do this for you):

```bash
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture rust
./examples/polyglot/run-demo.sh rust
```

## 4. Measured results (this host, 2026-11-09)

Windows 11, `cargo 1.95.0`, `rustc 1.95.0`, `rustfmt 1.9.0-stable`,
`clippy 0.1.95`, `git 2.51.1.windows.1`:

| Command | Result |
|---|---|
| `cargo fmt --check` | PASS (exit 0) |
| `cargo clippy -- -D warnings` | PASS (exit 0), no warnings |
| `cargo test --locked --offline` | PASS (exit 0): 2 unit tests + 1 doc test |
| `flow run` on the fixture index tree | `PASSED`, exit 0, 3/3 obligations `PASSED` |
| `flow publish --expected-old <head>` | `PUBLISHED`, exit 0 |

Linux (WSL2 Ubuntu), `cargo 1.94.1`, `rustfmt 1.8.0`, `clippy 0.1.94`,
`git 2.43.0`: the same three obligations ran through Flow and the demo reported
`PASSED` and `PUBLISHED`. This is a local WSL measurement, not CI evidence.

## 5. Red path (deliberately available)

`src/lib.rs:24`:

```rust
assert_eq!(add(2, 3), 5);
```

Change `5` to `6`, then `git add -A` and run Flow:

```bash
flow run          # FAILED, exit 1, obligation "test" FAILED (cargo test exit 101)
flow publish      # REFUSED, exit 1, code REQUIRED_NOT_PASSED; the ref does not move
```

Restore the line to return to green. This is the exact edit the demo drivers
apply; both fail loudly if the line has drifted.

## 6. Notes

- No network is used: the crate has no dependencies and the test command passes
  `--offline`.
- `target/` is ignored and is never created in this directory by Flow: the run
  materialises the tree in a throwaway directory outside the repository.
- `Cargo.lock` is committed because `--locked` refuses a missing or stale lock.
