# Wave-5 mutation campaign: the modules that wave 4 left `NOT_RUN`, scoped to the
# suites that actually exercise them. Sequential per module and two jobs, because
# every test spawns real `flow` processes and the machine has 16 GB.
#
# The scope is part of the result: a survivor may still be killed by a suite that
# was not selected, so each module's kill rate is a lower bound for that module
# and is never presented as a whole-crate score. `commands` is the large one and
# is last, so the other modules' results are complete even if the campaign is
# interrupted.
#
# `--gitignore true` keeps build/coverage artefacts out of every copy (the
# repository's `.gitignore` names them all). `--in-place` cannot be combined with
# `--jobs`.

$ErrorActionPreference = 'Continue'
$out = 'experiments\quality\mutation-w5'
New-Item -ItemType Directory -Force $out | Out-Null

$runs = @(
    @{ n = 'proposal'; f = 'crates/flow/src/proposal.rs'; t = @('--bins', '--test', 'proposals', '--test', 'error_paths', '--test', 'operator_surfaces') },
    @{ n = 'scratch';  f = 'crates/flow/src/scratch.rs';  t = @('--bins', '--test', 'isolation', '--test', 'error_paths', '--test', 'git_fidelity', '--test', 'crash_run') },
    @{ n = 'state';    f = 'crates/flow/src/state.rs';    t = @('--bins', '--test', 'cli', '--test', 'error_paths', '--test', 'operator_surfaces', '--test', 'crash_run') },
    @{ n = 'gitcli';   f = 'crates/flow/src/gitcli.rs';   t = @('--bins', '--test', 'isolation', '--test', 'bounds', '--test', 'error_paths') },
    @{ n = 'main';     f = 'crates/flow/src/main.rs';     t = @('--bins', '--test', 'cli', '--test', 'operator_surfaces') },
    @{ n = 'commands'; f = 'crates/flow/src/commands.rs'; t = @('--bins', '--test', 'cli', '--test', 'git_fidelity', '--test', 'operator_surfaces') }
)

foreach ($run in $runs) {
    $log = Join-Path $out ("{0}.log" -f $run.n)
    $dir = Join-Path $out $run.n
    Write-Host "=== $($run.n) -> $dir"
    cargo mutants --file $run.f --output $dir --jobs 2 --timeout 240 --gitignore true `
        --colors never --no-times -- $run.t *> $log
    $stamp = Get-Date -Format o
    Add-Content -Path (Join-Path $out 'progress.txt') -Value "$($run.n) $stamp"
}

'ALL_DONE' | Set-Content (Join-Path $out 'all-done.txt')
