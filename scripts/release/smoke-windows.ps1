#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Clean-room smoke matrix for a Windows Flow release binary.
.DESCRIPTION
  Runs the binary by absolute path from a private install directory, with a
  reduced environment (no repository, no .flow/), on throwaway Git repositories:
  version/init/run/publish, the negative cases, the test-crash hook check, and
  the "nothing was written outside the expected directories" audit.

  Results are emitted as JSON and as a summary table. The script exits 1 if any
  case does not match its expectation.
.EXAMPLE
  pwsh -File scripts/release/smoke-windows.ps1 `
    -Binary dist/flow-0.2.0-rc.1-x86_64-pc-windows-msvc/flow.exe `
    -OutJson wt-release-target/smoke-windows.json
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Binary,
    [string]$Root = (Join-Path $env:TEMP ("flow-smoke-" + [guid]::NewGuid().ToString('N').Substring(0, 8))),
    [string]$OutJson = ''
)

$ErrorActionPreference = 'Stop'
$binary = (Resolve-Path $Binary).Path
$root = [System.IO.Path]::GetFullPath($Root)
if (Test-Path $root) { Remove-Item -Recurse -Force $root }
foreach ($d in @('install', 'home', 'tmp', 'repos')) { New-Item -ItemType Directory -Force (Join-Path $root $d) | Out-Null }

$install = Join-Path $root 'install'
$flow = Join-Path $install 'flow.exe'
Copy-Item $binary $flow

# A reduced environment: no FLOW_* hooks, a private HOME/TMP, and only the
# directories a bare `git` needs. The product is a single binary; it must not
# need anything else.
$gitDir = Split-Path (Get-Command git).Source -Parent
$minimalPath = (@("$env:SystemRoot\System32", $env:SystemRoot, $gitDir) | Where-Object { $_ -and (Test-Path $_) }) -join ';'
$savedEnv = @{
    PATH         = $env:PATH
    HOME         = $env:HOME
    USERPROFILE  = $env:USERPROFILE
    TMP          = $env:TMP
    TEMP         = $env:TEMP
    FLOW_TEST_CRASH = $env:FLOW_TEST_CRASH
}

$results = New-Object System.Collections.Generic.List[object]
$failures = 0

function Set-MinimalEnv {
    param([hashtable]$Extra = @{})
    $env:PATH = $minimalPath
    $env:HOME = Join-Path $root 'home'
    $env:USERPROFILE = Join-Path $root 'home'
    $env:TMP = Join-Path $root 'tmp'
    $env:TEMP = Join-Path $root 'tmp'
    Remove-Item Env:FLOW_TEST_CRASH -ErrorAction SilentlyContinue
    foreach ($k in $Extra.Keys) { Set-Item -Path "Env:$k" -Value $Extra[$k] }
}
function Restore-Env {
    foreach ($k in $savedEnv.Keys) {
        if ($null -eq $savedEnv[$k]) { Remove-Item -Path "Env:$k" -ErrorAction SilentlyContinue }
        else { Set-Item -Path "Env:$k" -Value $savedEnv[$k] }
    }
    Remove-Item Env:FLOW_TEST_CRASH -ErrorAction SilentlyContinue
}

function New-Repo {
    param([string]$Name, [string]$Seed = 'x')
    $dir = Join-Path (Join-Path $root 'repos') $Name
    New-Item -ItemType Directory -Force $dir | Out-Null
    Set-Location $dir
    git init -q -b main . | Out-Null
    git config user.name 'Flow Smoke' | Out-Null
    git config user.email 'flow-smoke@example.invalid' | Out-Null
    git config commit.gpgsign false | Out-Null
    Set-Content -LiteralPath (Join-Path $dir 'a.txt') -Value $Seed -NoNewline
    Add-Content -LiteralPath (Join-Path $dir 'a.txt') -Value "`n"
    git add -A | Out-Null
    git commit -qm seed | Out-Null
    return $dir
}

function Write-Config {
    param([string]$Repo, [string]$Obligations)
    $cfg = '{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[' + $Obligations + '],"disabled":false}'
    New-Item -ItemType Directory -Force (Join-Path $Repo '.flow') | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $Repo '.flow\config.json'), $cfg)
}

function Run-Flow {
    param(
        [string]$Case,
        [string]$Repo,
        [string[]]$Arguments,
        [hashtable]$ExtraEnv = @{},
        [int]$ExpectExit = 0
    )
    Set-MinimalEnv -Extra $ExtraEnv
    Push-Location $Repo
    try {
        $outFile = Join-Path (Join-Path $root 'tmp') ("$Case.out")
        $errFile = Join-Path (Join-Path $root 'tmp') ("$Case.err")
        & $flow @Arguments > $outFile 2> $errFile
        $exit = $LASTEXITCODE
    }
    finally {
        Pop-Location
        Restore-Env
    }
    $text = Get-Content -LiteralPath $outFile -Raw -ErrorAction SilentlyContinue
    $json = $null
    if ($text -and $text.Trim()) { try { $json = $text | ConvertFrom-Json } catch { } }
    $status = if ($json -and $json.PSObject.Properties['status']) { $json.status } else { $null }
    $code = if ($json -and $json.PSObject.Properties['code']) { $json.code } else { $null }
    $pass = ($exit -eq $ExpectExit)
    if (-not $pass) { $failures++ }
    $results.Add([ordered]@{
            case       = $Case
            command    = "flow " + ($Arguments -join ' ')
            exit       = $exit
            expect_exit = $ExpectExit
            status     = $status
            code       = $code
            pass       = $pass
            json       = $json
        }) | Out-Null
    return [pscustomobject]@{ Exit = $exit; Status = $status; Code = $code; Json = $json; Text = $text }
}

# ---------------------------------------------------------------- positive cycle
$version = Run-Flow -Case 'version' -Repo (Join-Path $root 'install') -Arguments @('version')
$positive = New-Repo -Name 'positive'
$init = Run-Flow -Case 'init-default' -Repo $positive -Arguments @('init')
Write-Config -Repo $positive -Obligations '{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}'
$adopt = Run-Flow -Case 'init-adopt' -Repo $positive -Arguments @('init', '--adopt-policy')
Set-Content -LiteralPath (Join-Path $positive 'b.txt') -Value "y`n" -NoNewline
git -C $positive add -A | Out-Null
$run = Run-Flow -Case 'run-pos' -Repo $positive -Arguments @('run')
$publish = Run-Flow -Case 'publish-pos' -Repo $positive -Arguments @('publish')

# ---------------------------------------------------------------- negative cases
$missing = New-Repo -Name 'missing-config'
$missingRun = Run-Flow -Case 'missing-config' -Repo $missing -Arguments @('run') -ExpectExit 1

$corrupt = New-Repo -Name 'corrupt-config'
New-Item -ItemType Directory -Force (Join-Path $corrupt '.flow') | Out-Null
[System.IO.File]::WriteAllText((Join-Path $corrupt '.flow\config.json'), '{not json')
$corruptRun = Run-Flow -Case 'corrupt-config' -Repo $corrupt -Arguments @('run') -ExpectExit 1

$unapproved = New-Repo -Name 'unapproved-policy'
Write-Config -Repo $unapproved -Obligations '{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}'
$unapprovedRun = Run-Flow -Case 'unapproved-policy' -Repo $unapproved -Arguments @('run') -ExpectExit 1

$missingProgram = New-Repo -Name 'missing-program'
Write-Config -Repo $missingProgram -Obligations '{"id":"gone","program":"flow-definitely-missing-command-xyz","args":[],"required":true,"timeout_s":10}'
$adoptMissing = Run-Flow -Case 'init-adopt-missing-program' -Repo $missingProgram -Arguments @('init', '--adopt-policy')
$missingProgramRun = Run-Flow -Case 'missing-program' -Repo $missingProgram -Arguments @('run') -ExpectExit 1

$readOnly = New-Repo -Name 'read-only'
$me = (whoami).Trim()
# Deny only "add file / add subdirectory" (WD,AD), not generic write: denying
# FILE_GENERIC_WRITE also makes the directory unreadable and the test would
# fail at repository discovery instead of at the write it is meant to exercise.
icacls $readOnly /deny "${me}:(OI)(CI)(WD,AD)" | Out-Null
try {
    $readOnlyRun = Run-Flow -Case 'read-only-dir' -Repo $readOnly -Arguments @('init') -ExpectExit 1
}
finally {
    icacls $readOnly /remove:d $me | Out-Null
}

# ---------------------------------------------------------------- crash hook
$hookRepo = New-Repo -Name 'crash-hook'
Write-Config -Repo $hookRepo -Obligations '{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}'
$hookAdopt = Run-Flow -Case 'init-adopt-crash-hook' -Repo $hookRepo -Arguments @('init', '--adopt-policy')
Set-Content -LiteralPath (Join-Path $hookRepo 'b.txt') -Value "y`n" -NoNewline
git -C $hookRepo add -A | Out-Null
$hookRun = Run-Flow -Case 'run-crash-hook' -Repo $hookRepo -Arguments @('run')
$hookPublish = Run-Flow -Case 'crash-hook-publish' -Repo $hookRepo -Arguments @('publish') -ExtraEnv @{ FLOW_TEST_CRASH = 'after-intent' }

# ---------------------------------------------------------------- audit
$installFiles = @(Get-ChildItem -Force $install | ForEach-Object Name)
$scratch = @(Get-ChildItem -Force (Join-Path $root 'tmp') -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like 'flow-scratch*' } | ForEach-Object Name)
$repoFiles = @(Get-ChildItem -Recurse -Force (Join-Path $root 'repos') -ErrorAction SilentlyContinue |
    Where-Object { -not $_.PSIsContainer } |
    ForEach-Object { $_.FullName.Substring($root.Length + 1) })
$outsideCandidates = @(
    (Join-Path $env:APPDATA 'flow'),
    (Join-Path $env:LOCALAPPDATA 'flow'),
    $(if ($savedEnv.USERPROFILE) { Join-Path $savedEnv.USERPROFILE '.flow' } else { $null })
) | Where-Object { $_ } | ForEach-Object { [pscustomobject]@{ path = $_; exists = (Test-Path $_) } }

$audit = [ordered]@{
    install_dir      = $install
    install_files    = $installFiles
    flow_scratch_left = $scratch
    repo_file_count  = $repoFiles.Count
    repo_flow_files  = @($repoFiles | Where-Object { $_ -like '*.flow*' })
    outside_candidates = $outsideCandidates
}

# Expected statuses (measured, asserted after the fact so the raw values stay in the JSON).
$checks = [ordered]@{
    'version'                    = ($version.Status -eq 'VERSION' -and $version.Exit -eq 0)
    'init-default'               = ($init.Status -eq 'INITIALIZED')
    'init-adopt'                 = ($adopt.Status -eq 'INITIALIZED')
    'run-pos'                    = ($run.Status -eq 'PASSED' -and $run.Json.obligations[0].status -eq 'PASSED')
    'publish-pos'                = ($publish.Status -eq 'PUBLISHED' -and $publish.Json.ref -eq 'refs/heads/main')
    'missing-config'             = ($missingRun.Exit -eq 1)
    'corrupt-config'             = ($corruptRun.Exit -eq 1)
    'unapproved-policy'          = ($unapprovedRun.Status -eq 'REFUSED' -and $unapprovedRun.Code -eq 'POLICY_NOT_ADOPTED')
    'missing-program'            = ($missingProgramRun.Status -eq 'UNSUPPORTED' -and $missingProgramRun.Json.obligations[0].status -eq 'UNSUPPORTED')
    'missing-program-never-passed' = ($missingProgramRun.Json.status -ne 'PASSED')
    'read-only-dir'              = ($readOnlyRun.Exit -eq 1)
    'crash-hook-inert'           = ($hookPublish.Status -eq 'PUBLISHED' -and $hookPublish.Exit -eq 0)
    'installed-dir-clean'        = (@($installFiles | Where-Object { $_ -ne 'flow.exe' }).Count -eq 0)
    'no-scratch-left'            = ($scratch.Count -eq 0)
}
foreach ($k in $checks.Keys) {
    if (-not $checks[$k]) { $failures++ }
}

$report = [ordered]@{
    schema       = 'flow-smoke/1'
    platform     = 'windows'
    binary       = $binary
    binary_sha256 = (Get-FileHash $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    started_root = $root
    minimal_env  = [ordered]@{ PATH = $minimalPath; HOME = (Join-Path $root 'home'); TMP = (Join-Path $root 'tmp') }
    results      = $results
    checks       = $checks
    audit        = $audit
    failures     = $failures
}
$jsonText = ($report | ConvertTo-Json -Depth 20) + "`n"
if ($OutJson) {
    $outDir = Split-Path $OutJson -Parent
    if ($outDir) { New-Item -ItemType Directory -Force $outDir | Out-Null }
    [System.IO.File]::WriteAllText($OutJson, $jsonText, (New-Object System.Text.UTF8Encoding($false)))
}

$results.ToArray() | ForEach-Object {
    [pscustomobject]@{ case = $_['case']; exit = $_['exit']; expect_exit = $_['expect_exit']; status = $_['status']; code = $_['code']; pass = $_['pass'] }
} | Format-Table -AutoSize
Write-Host "checks:"
$checks.GetEnumerator() | ForEach-Object { Write-Host ("  {0,-30} {1}" -f $_.Key, $_.Value) }
Write-Host "install dir: $($installFiles -join ', ')"
Write-Host "scratch left: $($scratch.Count)"
Write-Host "failures: $failures"
if ($failures -gt 0) { exit 1 }
