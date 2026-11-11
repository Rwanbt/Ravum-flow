#!/usr/bin/env pwsh
# Reproduce the Flow quality gates in one command (Windows / PowerShell 7).
#
# The gates a reviewer applies are documented in several reports; this script is
# the single entry point so the commands cannot drift from the claims:
#
#   pwsh -File experiments/quality/reproduce.ps1                 # fast: fmt, clippy, tests
#   pwsh -File experiments/quality/reproduce.ps1 -Coverage       # + product-only coverage gate
#   pwsh -File experiments/quality/reproduce.ps1 -MutationFile crates/flow/src/lock.rs
#
# Exit code 0 only when every requested step passes. Nothing is downloaded: the
# crate has no dependencies, so `--offline` is meaningful. `-MutationFile` runs a
# whole module and takes minutes; a rate it prints is a lower bound for the
# selected suite (`--bins`), never a crate-wide score.
param(
    [switch]$Coverage,
    [string]$MutationFile = '',
    [string]$Repo = ''
)
$ErrorActionPreference = 'Stop'
if (-not $Repo) { $Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path }
Set-Location $Repo
$script:failed = 0
$script:rows = @()
Write-Host "repo   : $Repo"
Write-Host "head   : $(git rev-parse --short HEAD)"
Write-Host "rustc  : $(rustc --version)"

function Record([string]$name, [int]$exit, [double]$seconds, [string]$detail) {
    $verdict = if ($exit -eq 0) { 'PASS' } else { 'FAIL' }
    if ($exit -ne 0) { $script:failed++ }
    $script:rows += [pscustomobject]@{ Step = $name; Verdict = $verdict; Seconds = $seconds; Detail = $detail }
    Write-Host ("[{0}] {1} ({2}s) {3}" -f $verdict, $name, $seconds, $detail)
}

# 1. Formatting.
$sw = [Diagnostics.Stopwatch]::StartNew()
$text = (cargo fmt --all -- --check 2>&1 | Out-String)
$sw.Stop()
Record 'fmt' $LASTEXITCODE ([math]::Round($sw.Elapsed.TotalSeconds, 1)) 'cargo fmt --all -- --check'

# 2. Lints, warnings are errors.
$sw = [Diagnostics.Stopwatch]::StartNew()
$text = (cargo clippy --locked --offline --all-targets -- -D warnings 2>&1 | Out-String)
$sw.Stop()
Record 'clippy -D warnings' $LASTEXITCODE ([math]::Round($sw.Elapsed.TotalSeconds, 1)) 'cargo clippy --all-targets'

# 3. The full suite, counted: a suite that runs fewer tests is a regression even
#    when it is green, so the count is printed and compared by eye with the floor.
$sw = [Diagnostics.Stopwatch]::StartNew()
$text = (cargo test --locked --offline --all-targets 2>&1 | Out-String)
$sw.Stop()
$count = {
    param($kind)
    ([regex]::Matches($text, "(\d+) $kind") | ForEach-Object { [int]::Parse($_.Groups[1].Value) } | Measure-Object -Sum).Sum
}
$passed = & $count 'passed'; $failedCount = & $count 'failed'; $ignored = & $count 'ignored'
Record 'tests' $LASTEXITCODE ([math]::Round($sw.Elapsed.TotalSeconds, 1)) "passed=$passed failed=$failedCount ignored=$ignored"

# 4. Product-only branch coverage, on the toolchain the CI pins for `--branch`.
if ($Coverage) {
    $toolchain = 'nightly-2026-05-06'
    & rustup run $toolchain rustc --version *> $null
    if ($LASTEXITCODE -ne 0) {
        Record 'coverage' 1 0 "toolchain $toolchain is not installed (rustup toolchain install $toolchain --component llvm-tools-preview)"
    } else {
        $lcov = Join-Path $Repo 'target/flow-coverage-lcov.info'
        $sw = [Diagnostics.Stopwatch]::StartNew()
        cargo "+$toolchain" llvm-cov clean --workspace *> $null
        cargo "+$toolchain" llvm-cov --locked --offline --all-targets --branch --ignore-filename-regex '[\\/]tests[\\/]' --no-report *> $null
        $measure = $LASTEXITCODE
        if ($measure -eq 0) {
            cargo "+$toolchain" llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]' --lcov --output-path $lcov *> $null
            $measure = $LASTEXITCODE
        }
        $sw.Stop()
        if ($measure -eq 0) {
            $gate = (python experiments/quality/analyze_lcov.py $lcov --product-only --fail-under-lines 95 --fail-under-branches 90 2>&1 | Out-String)
            $measure = $LASTEXITCODE
            $total = ($gate -split "`n" | Where-Object { $_ -match 'PRODUCT-ONLY TOTAL' }) -join ''
            Record 'coverage' $measure ([math]::Round($sw.Elapsed.TotalSeconds, 1)) $total.Trim()
        } else {
            Record 'coverage' $measure ([math]::Round($sw.Elapsed.TotalSeconds, 1)) 'measurement failed'
        }
    }
}

# 5. One module of mutation, opt-in: minutes, not seconds.
if ($MutationFile) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $text = (cargo mutants --file $MutationFile --jobs 2 --timeout 240 --gitignore true --colors never --no-times -- --bins 2>&1 | Out-String)
    $sw.Stop()
    $summary = ($text -split "`n" | Where-Object { $_ -match 'mutants tested|caught|missed' } | Select-Object -Last 2) -join ' '
    Record "mutation $MutationFile" $LASTEXITCODE ([math]::Round($sw.Elapsed.TotalSeconds, 1)) $summary.Trim()
} else {
    Write-Host '[SKIP] mutation (pass -MutationFile <path> to run one module)'
}

Write-Host ''
$script:rows | Format-Table -AutoSize
if ($script:failed -gt 0) { Write-Host "$($script:failed) step(s) failed"; exit 1 }
Write-Host 'all requested steps passed'
exit 0
