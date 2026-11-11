# Wave-6 mutation campaign (continuation of wave 5): the four modules that were
# still `NOT_RUN`. Same method, same suites, same guards; the difference is only
# that this campaign starts from a dedicated worktree pinned at the wave-5 head
# commit, so the sources under mutation are exactly a committed tree and cannot
# drift while the campaign is running.
#
# The scope is part of the result: a survivor may still be killed by a suite that
# was not selected, so each module's kill rate is a lower bound for that module
# and is never presented as a whole-crate score.
#
# CARGO_TARGET_DIR is removed on purpose, as in wave 5. cargo-mutants copies the
# workspace per mutant and expects a *private* target directory inside that copy;
# a shared CARGO_TARGET_DIR makes two concurrent mutants link the same
# `flow-*.exe`, which was observed in wave 4 as LNK1104 build failures and as a
# test suite that passed against the wrong build. A contaminated run is not a
# result, so the guard below fails closed instead of reporting one.
#
# Usage (from the pinned worktree):
#   pwsh -File D:\App\Ravum-flow\experiments\quality\mutation-w6\run-mutations.ps1
# The output directory defaults to the main checkout, so the evidence lands in
# the repository that will be committed even though the sources come from the
# worktree.

param(
    [string]$OutDir = 'D:\App\Ravum-flow\experiments\quality\mutation-w6'
)

if (Test-Path Env:CARGO_TARGET_DIR) {
    Remove-Item Env:CARGO_TARGET_DIR
}
$env:TEMP = if ($env:MUTANTS_TEMP) { $env:MUTANTS_TEMP } else { 'D:\mutants-w6-tmp' }
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $env:TEMP | Out-Null

$ErrorActionPreference = 'Continue'
New-Item -ItemType Directory -Force $OutDir | Out-Null

$head = (git rev-parse HEAD).Trim()
Add-Content -Path (Join-Path $OutDir 'progress.txt') -Value "campaign-start $head $(Get-Date -Format o)"

$runs = @(
    @{ n = 'state';    f = 'crates/flow/src/state.rs';    t = @('--bins', '--test', 'cli', '--test', 'error_paths', '--test', 'operator_surfaces', '--test', 'crash_run') },
    @{ n = 'gitcli';   f = 'crates/flow/src/gitcli.rs';   t = @('--bins', '--test', 'isolation', '--test', 'bounds', '--test', 'error_paths') },
    @{ n = 'main';     f = 'crates/flow/src/main.rs';     t = @('--bins', '--test', 'cli', '--test', 'operator_surfaces') },
    @{ n = 'commands'; f = 'crates/flow/src/commands.rs'; t = @('--bins', '--test', 'cli', '--test', 'git_fidelity', '--test', 'operator_surfaces') }
)

foreach ($run in $runs) {
    $log = Join-Path $OutDir ("{0}.log" -f $run.n)
    $dir = Join-Path $OutDir $run.n
    Write-Host "=== $($run.n) -> $dir"
    cargo mutants --file $run.f --output $dir --jobs 2 --timeout 240 --gitignore true `
        --colors never --no-times -- $run.t *> $log
    $stamp = Get-Date -Format o
    Add-Content -Path (Join-Path $OutDir 'progress.txt') -Value "$($run.n) $stamp"
}

'ALL_DONE' | Set-Content (Join-Path $OutDir 'all-done.txt')
