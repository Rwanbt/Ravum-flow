# Resume the two wave-6 module runs that were interrupted by the host.
#
# `state` and `gitcli` stopped after 42/155 and 93/104 mutants with no summary line
# and no error from cargo-mutants: the process disappeared mid-mutant (the debug log
# ends inside a `Test` phase and a build directory was left behind). A partial
# module is not a result, so the remaining mutants are re-run here with the same
# scope, one job instead of two, and `--iterate` so the already-caught mutants are
# not paid for twice. Every run's exit code is recorded; a second interruption is
# reported, never smoothed over.

if (Test-Path Env:CARGO_TARGET_DIR) {
    Remove-Item Env:CARGO_TARGET_DIR
}
$env:TEMP = 'D:\mutants-w6-tmp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $env:TEMP | Out-Null

$ErrorActionPreference = 'Continue'
$out = 'D:\App\Ravum-flow\experiments\quality\mutation-w6'

$runs = @(
    @{ n = 'state';  f = 'crates/flow/src/state.rs';  t = @('--bins', '--test', 'cli', '--test', 'error_paths', '--test', 'operator_surfaces', '--test', 'crash_run') },
    @{ n = 'gitcli'; f = 'crates/flow/src/gitcli.rs'; t = @('--bins', '--test', 'isolation', '--test', 'bounds', '--test', 'error_paths') }
)

foreach ($run in $runs) {
    $log = Join-Path $out ("{0}-resume.log" -f $run.n)
    $dir = Join-Path $out $run.n
    Write-Host "=== resume $($run.n) -> $dir"
    cargo mutants --file $run.f --output $dir --iterate --jobs 1 --timeout 240 --gitignore true `
        --colors never --no-times -- $run.t *> $log
    $code = $LASTEXITCODE
    Add-Content -Path (Join-Path $out 'resume.txt') -Value "$($run.n) exit=$code $(Get-Date -Format o)"
    Write-Host "=== resume $($run.n) exit=$code"
}
'RESUME_DONE' | Set-Content (Join-Path $out 'resume-done.txt')
