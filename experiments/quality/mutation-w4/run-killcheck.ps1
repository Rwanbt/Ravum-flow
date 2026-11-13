# Wave-4 targeted re-verification.
#
# 1. `killcheck-json-paths`: the survivors the first json/paths runs reported are
#    re-tested after the assertions that close them (escaped/multi-byte strings,
#    space not escaped, and one refusal reason per absolute/traversal shape).
#    This is a killcheck of exactly those mutation sites, not a new campaign.
# 2. `lock2`: the lock module is re-run against `--test cli --test concurrency`
#    as well, because the dead-owner steal path (and its nonce guard) is only
#    exercised from the CLI/concurrency suites -- scoping the first run to
#    `--bins` was too narrow and is corrected here instead of explained away.

$ErrorActionPreference = 'Continue'
$out = 'experiments\quality\mutation-w4'

Write-Host '=== killcheck json/paths'
cargo mutants --file crates/flow/src/json.rs --file crates/flow/src/paths.rs `
    -F '(json\.rs:(145|295|299|309|345|347|348)|paths\.rs:(44|48|55))' `
    --output (Join-Path $out 'killcheck-json-paths') --jobs 2 --timeout 240 --gitignore true `
    --colors never --no-times -- --bins *> (Join-Path $out 'killcheck-json-paths.log')

Write-Host '=== lock2 (with cli + concurrency)'
cargo mutants --file crates/flow/src/lock.rs `
    --output (Join-Path $out 'lock2') --jobs 2 --timeout 240 --gitignore true `
    --colors never --no-times -- --bins --test cli --test concurrency *> (Join-Path $out 'lock2.log')

'KILLCHECK_DONE' | Set-Content (Join-Path $out 'killcheck-done.txt')
