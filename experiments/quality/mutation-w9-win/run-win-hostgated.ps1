# Wave 9 — measure the eight #[cfg(windows)]-gated mutants on their native host.
#
# Wave 8 measured the crate on Linux and reported these eight mutants as
# "host-gated": they can only execute on Windows, so the Linux campaign could
# not judge them. This run pins HEAD in a detached worktree, drops a one-token
# comment probe on each target line (so `--in-diff` selects exactly those
# mutants), runs them with the pristine unit-test suite, and restores the files.
#
# The probe is a comment: it never changes behaviour, and it is reverted before
# the worktree is inspected. The campaign is a lower bound on the scoped suite
# (`--bins`), never a crate-wide claim.
$ErrorActionPreference = 'Stop'
$root = 'D:\App\Ravum-flow'
$wt = 'D:\App\flow-w9-win'
$out = Join-Path $root 'experiments\quality\mutation-w9-win'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$log = Join-Path $out 'run.log'
Set-Content -Path $log -Value "wave9 windows host-gated campaign" -Encoding utf8
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

# state.rs:316 — the reparse-point attribute test inside guard_below_flow.
Probe 'crates/flow/src/state.rs' 'meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0' 'meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 // w9probe'

# gitcli.rs:523 — the call that resolves a program name on Windows.
Probe 'crates/flow/src/gitcli.rs' '    resolve_program_in(' '    resolve_program_in( // w9probe'
# gitcli.rs:539 — the PATH-is-absent early return.
Probe 'crates/flow/src/gitcli.rs' '    let path = path?;' '    let path = path?; // w9probe'
# gitcli.rs:543 — the empty-extension filter in the PATHEXT split.
Probe 'crates/flow/src/gitcli.rs' '.filter(|e| !e.is_empty())' '.filter(|e| !e.is_empty()) // w9probe'

$diffPath = Join-Path $out 'probe-win.diff'
$d = git -C $wt diff -U0 -- crates/flow/src/state.rs crates/flow/src/gitcli.rs
[System.IO.File]::WriteAllText($diffPath, (($d -join "`n") + "`n"))
Note "--- probe diff ---"
Note ($d -join "`n")

$runOut = Join-Path $out 'mutants'
if (Test-Path $runOut) { Remove-Item -Recurse -Force $runOut }
Push-Location $wt
try {
    $args = @('mutants', '--in-diff', $diffPath, '--jobs', '2', '--timeout', '240',
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
