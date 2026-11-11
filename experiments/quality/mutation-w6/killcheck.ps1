# Targeted killchecks for the wave-6 mutation campaign.
#
# The campaign scopes each module to a few integration suites, so a survivor may
# simply be a site that an *unselected* suite exercises. Re-running a whole module
# with the full suite list costs hours; `cargo mutants --in-diff` restricts the run
# to mutants on the lines a diff touches, which is exactly the reported sites.
#
# The diff is built by appending a trailing comment to each survivor line and
# reverting the file afterwards. The tree the killcheck runs on is a committed
# one (`-Repo` is a throwaway worktree whose HEAD already contains the tests), so
# the probe is the only change in the diff and uncommitted work cannot leak into
# it.
#
# Usage (arguments are comma-separated strings so both `pwsh -File` and an
# interactive call parse them the same way):
#   pwsh -File killcheck.ps1 -Repo D:\App\Ravum-flow-wt\mut-w6 -Module gitcli `
#       -Lines 134,153,177 -Tests cli,proposals,operator_surfaces,error_paths

param(
    [Parameter(Mandatory = $true)][string]$Repo,
    [Parameter(Mandatory = $true)][string]$Module,
    [Parameter(Mandatory = $true)][string]$Lines,
    [string]$Tests = 'cli,proposals,operator_surfaces,error_paths,isolation,bounds,git_fidelity',
    [int]$Jobs = 2,
    [string]$LibtestArgs = '',
    [string]$OutDir = 'D:\App\Ravum-flow\experiments\quality\mutation-w6'
)

$ErrorActionPreference = 'Stop'
if (Test-Path Env:CARGO_TARGET_DIR) {
    Remove-Item Env:CARGO_TARGET_DIR
}
$env:TEMP = 'D:\mutants-w6-tmp'
$env:TMP = $env:TEMP

$relative = "crates/flow/src/$Module.rs"
$file = Join-Path $Repo $relative
if (-not (Test-Path $file)) { throw "no such module: $file" }
if (git -C $Repo status --porcelain -- $relative) {
    throw "refusing to probe: $relative has uncommitted changes in $Repo"
}

$diff = Join-Path $OutDir "probe-$Module.diff"
$dir = Join-Path $OutDir "killcheck-$Module"
$log = Join-Path $OutDir "killcheck-$Module.log"

$original = [System.IO.File]::ReadAllText($file)
$content = $original -split "`n"
foreach ($line in $Lines.Split(',')) {
    $index = [int]$line.Trim() - 1
    if ($index -lt 0 -or $index -ge $content.Count) { throw "line $line is outside $relative" }
    $content[$index] = $content[$index].TrimEnd("`r") + ' // survival-probe' + $(if ($content[$index].EndsWith("`r")) { "`r" } else { '' })
}
try {
    [System.IO.File]::WriteAllText($file, ($content -join "`n"))
    # `cmd` redirection keeps git's bytes untouched (PowerShell would rewrite the
    # line endings of a unified diff, which the parser must see verbatim).
    cmd /c "git -C `"$Repo`" diff -U0 -- `"$relative`" > `"$diff`""

    # The probe has to stay in the tree for the whole run: cargo-mutants checks
    # that the diff's post-image matches the source it is about to mutate, and it
    # copies the workspace per mutant. Reverting before the run makes it refuse
    # the diff ("Diff content doesn't match source file").
    $testArgs = @('--bins')
    foreach ($test in $Tests.Split(',')) {
        if ($test.Trim()) { $testArgs += @('--test', $test.Trim()) }
    }
    if ($LibtestArgs) { $testArgs += @('--', $LibtestArgs) }

    Push-Location $Repo
    try {
        cargo mutants --file $relative --in-diff $diff --output $dir --jobs $Jobs --timeout 240 `
            --gitignore true --colors never --no-times -- $testArgs *> $log
    }
    finally {
        Pop-Location
    }
}
finally {
    [System.IO.File]::WriteAllText($file, $original)
}
Get-Content $log
