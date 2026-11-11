# Minimal reproduction for the win32 `resolve_program -> None` mutant (campaign
# site gitcli.rs:515), without cargo: compile the crate directly (it has no
# dependencies), mutate one function body, and run the `.cmd`-shim obligation by
# hand. Used to decide whether that campaign survivor is real or a false negative.
$ErrorActionPreference = 'Stop'
$work = Join-Path $env:TEMP 'flow-probe515'
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $work | Out-Null
Copy-Item -Recurse 'D:\App\Ravum-flow\crates\flow' (Join-Path $work 'crate')
$env:CARGO_PKG_VERSION = '0.2.0-rc.1'
$env:CARGO_PKG_NAME = 'flow'

$source = Join-Path $work 'crate\src\main.rs'
$good = Join-Path $work 'flow-good.exe'
& rustc --edition 2021 $source -o $good
if ($LASTEXITCODE -ne 0) { throw 'unmutated build failed' }

$mutated = Join-Path $work 'crate-mutated'
Copy-Item -Recurse (Join-Path $work 'crate') $mutated
$gitcli = Join-Path $mutated 'src\gitcli.rs'
$text = [System.IO.File]::ReadAllText($gitcli)
$pattern = 'fn resolve_program\(program: &str\) -> Option<PathBuf> \{[\s\S]*?\n\}'
if ($text -notmatch $pattern) { throw 'the win32 resolve_program body was not found' }
$replacement = "fn resolve_program(_program: &str) -> Option<PathBuf> {`n    None`n}"
$text = [regex]::Replace($text, $pattern, $replacement, 1)
[System.IO.File]::WriteAllText($gitcli, $text)
$bad = Join-Path $work 'flow-515.exe'
& rustc --edition 2021 (Join-Path $mutated 'src\main.rs') -o $bad
if ($LASTEXITCODE -ne 0) { throw 'mutated build failed' }

$repo = Join-Path $work 'repo'
New-Item -ItemType Directory -Force $repo | Out-Null
& git -C $repo init -q -b main
& git -C $repo config user.name 'Flow Probe'
& git -C $repo config user.email 'flow@example.invalid'
& git -C $repo config commit.gpgsign false
Set-Content -Path (Join-Path $repo 'verified.txt') -Value 'content under verification'
& git -C $repo add -A
& git -C $repo commit -q -m seed
$shims = Join-Path $work 'shims'
New-Item -ItemType Directory -Force $shims | Out-Null
Set-Content -Path (Join-Path $shims 'flowprobe.cmd') -Value "@echo off`r`nexit /b 0`r`n"
$env:PATH = "$shims;$env:PATH"

Set-Location $repo
$obligations = '{"id":"shim","program":"flowprobe","args":[],"required":true,"timeout_s":30}'
$config = '{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[' + $obligations + '],"disabled":false}'

foreach ($binary in @(@{ name = 'unmutated'; exe = $good }, @{ name = 'mutant-515'; exe = $bad })) {
    Remove-Item -Recurse -Force (Join-Path $repo '.flow') -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force (Join-Path $repo '.flow') | Out-Null
    Set-Content -Path (Join-Path $repo '.flow\config.json') -Value $config
    $approve = (& $binary.exe approve 2>&1 | Out-String).Trim()
    $run = (& $binary.exe run 2>&1 | Out-String).Trim()
    $code = $LASTEXITCODE
    "=== $($binary.name): exit=$code"
    "    approve: $approve"
    "    run:     $run"
}
