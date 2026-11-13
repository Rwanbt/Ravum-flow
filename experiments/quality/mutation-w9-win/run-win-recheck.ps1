# Wave 9 — re-measure the five mutants that wave 8 could not run or that survived.
#
#  * main.rs:360:19 and :372:15 — the three `collect_values` index advances that
#    wave 8 reported as NOT_RUN after the WSL distro died. The commit under
#    measurement adds a watchdog around the call, so a non-advancing walk is a
#    fast failure instead of a 240 s suite stall.
#  * gitcli.rs:523:5 — the two `resolve_program` wrapper mutants that the wave-9
#    Windows campaign found MISSED. The commit under measurement adds a test that
#    drives the wrapper through the real process environment.
#
# Same method as the wave-9 first pass: a comment probe on the target line
# selects exactly those mutants through `--in-diff`, the probe is reverted, and
# the campaign scope is `--bins` (a lower bound, never a crate-wide claim).
$ErrorActionPreference = 'Stop'
$root = 'D:\App\Ravum-flow'
$wt = 'D:\App\flow-w9-win2'
$out = Join-Path $root 'experiments\quality\mutation-w9-win\recheck'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$log = Join-Path $out 'run.log'
Set-Content -Path $log -Value "wave9 windows recheck campaign" -Encoding utf8
function Note([string]$m) { Add-Content -Path $log -Value $m -Encoding utf8; Write-Output $m }

git -C $root worktree remove --force $wt 2>$null | Out-Null
if (Test-Path $wt) { Remove-Item -Recurse -Force $wt }
git -C $root worktree add --detach $wt HEAD 2>&1 | Out-Null
Note ("HEAD=" + (git -C $wt rev-parse HEAD).Trim())
Note ("cargo-mutants=" + ((cargo mutants --version) -join ' '))

function Probe([string]$rel, [string]$old, [string]$new) {
    $p = Join-Path $wt $rel
    $t = [System.IO.File]::ReadAllText($p)
    $n = ([regex]::Matches($t, [regex]::Escape($old))).Count
    if ($n -ne 1) { throw "probe anchor matched $n times in $rel (expected 1): $old" }
    [System.IO.File]::WriteAllText($p, $t.Replace($old, $new))
    Note ("probe ok: $rel <- $old")
}

# collect_values: the value-flag advance (12 spaces, distinct from the 20-space
# advance inside validate_flags). The match is anchored to the start of a line:
# a plain substring count also matches the 20-space line, whose leading spaces
# contain the 12-space prefix.
$mainPath = Join-Path $wt 'crates/flow/src/main.rs'
$mainText = [System.IO.File]::ReadAllText($mainPath)
$flagPattern = '(?m)^            index \+= 2;\r?$'
$flagHits = ([regex]::Matches($mainText, $flagPattern)).Count
if ($flagHits -ne 1) { throw "collect_values value-flag anchor matched $flagHits times (expected 1)" }
$mainText = [regex]::Replace($mainText, $flagPattern, '            index += 2; // w9probe')
[System.IO.File]::WriteAllText($mainPath, $mainText)
Note 'probe ok: crates/flow/src/main.rs <- collect_values value-flag advance'

# collect_values: the per-token advance, anchored on the line above so the
# identical advances in validate_flags/flag_value are not selected.
$mainText = [System.IO.File]::ReadAllText($mainPath)
$pattern = 'let _ = matched;\r?\n        index \+= 1;'
$hits = ([regex]::Matches($mainText, $pattern)).Count
if ($hits -ne 1) { throw "collect_values advance anchor matched $hits times (expected 1)" }
$mainText = [regex]::Replace($mainText, $pattern, 'let _ = matched;' + "`n" + '        index += 1; // w9probe')
[System.IO.File]::WriteAllText($mainPath, $mainText)
Note 'probe ok: crates/flow/src/main.rs <- collect_values per-token advance'

Probe 'crates/flow/src/gitcli.rs' '    resolve_program_in(' '    resolve_program_in( // w9probe'

$diffPath = Join-Path $out 'probe-recheck.diff'
$d = git -C $wt diff -U0 -- crates/flow/src/main.rs crates/flow/src/gitcli.rs
[System.IO.File]::WriteAllText($diffPath, (($d -join "`n") + "`n"))
Note "--- probe diff ---"
Note ($d -join "`n")

$runOut = Join-Path $out 'mutants'
if (Test-Path $runOut) { Remove-Item -Recurse -Force $runOut }
Push-Location $wt
try {
    $args = @('mutants', '--in-diff', $diffPath, '--jobs', '1', '--timeout', '240',
        '--gitignore', 'true', '--output', $runOut, '--', '--bins')
    & cargo @args 2>&1 | Tee-Object -FilePath (Join-Path $out 'cargo-mutants.log') | Out-Null
    Note ("cargo mutants exit=" + $LASTEXITCODE)
} finally {
    Pop-Location
    git -C $wt checkout -- . 2>&1 | Out-Null
}

Note ("restored tree status: " + ((git -C $wt status --porcelain) -join '; '))
foreach ($f in 'caught', 'missed', 'unviable', 'timeout', 'success') {
    $p = Join-Path $runOut "mutants.out\$f.txt"
    Note "--- $f ---"
    if (Test-Path $p) { Note ((Get-Content $p) -join "`n") } else { Note '(none)' }
}
