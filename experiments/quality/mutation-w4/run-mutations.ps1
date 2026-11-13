# Wave-4 mutation campaign: scoped, sequential per module, two jobs.
#
# Whole-suite mutation is ~70 s per viable mutant because every test spawns real
# `flow` processes, so a full-crate run needs ~24 h. Each module is therefore
# mutated against the test binaries that actually exercise it (`--bins` = the
# module's own unit tests; the named integration suites exercise it through the
# CLI). The scoping is reported with the numbers: a survivor may still be killed
# by a suite that was not selected, so the kill rate is a lower bound for that
# module and is never presented as a whole-crate result.
#
# `--gitignore true` keeps the build/coverage artefacts out of every copy; the
# repository's `.gitignore` names them all, which is why a copy is ~20 MB rather
# than the ~4 GB working directory. `--in-place` cannot be combined with
# `--jobs`, and serial runs would double the wall time.

$ErrorActionPreference = 'Continue'
$out = 'experiments\quality\mutation-w4'
New-Item -ItemType Directory -Force $out | Out-Null

$runs = @(
    @{ n = 'json';     f = 'crates/flow/src/json.rs';     t = @('--bins') },
    @{ n = 'paths';    f = 'crates/flow/src/paths.rs';    t = @('--bins') },
    @{ n = 'lock';     f = 'crates/flow/src/lock.rs';     t = @('--bins') }
)

foreach ($run in $runs) {
    $log = Join-Path $out ("{0}.log" -f $run.n)
    $dir = Join-Path $out $run.n
    Write-Host "=== $($run.n) -> $dir"
    cargo mutants --file $run.f --output $dir --jobs 2 --timeout 240 --gitignore true `
        --colors never --no-times -- $run.t *> $log
}

'ALL_DONE' | Set-Content (Join-Path $out 'all-done.txt')
