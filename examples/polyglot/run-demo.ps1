#requires -Version 7.0
<#
.SYNOPSIS
    End-to-end Ravum Flow demo on a disposable copy of a polyglot fixture.

.DESCRIPTION
    Copies one fixture to a fresh temporary directory, turns that copy into a Git
    repository, adopts its policy, verifies the index tree, publishes it, then
    applies the fixture's documented red edit and shows that verification fails
    and publication is refused.

    Nothing is written into this repository: the CLI is built with an external
    --target-dir and the Git work happens only in the temporary directory.

.PARAMETER Fixture
    rust (default), node, python, or all.

.PARAMETER KeepTemp
    Keep the temporary directory (its path is printed) for inspection.

.EXAMPLE
    pwsh -NoProfile -File examples/polyglot/run-demo.ps1
    pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture all
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('rust', 'node', 'python', 'all')]
    [string]$Fixture = 'rust',

    [switch]$KeepTemp
)

$ErrorActionPreference = 'Stop'
# On PowerShell 7.3+ a non-zero native exit code can be turned into a terminating
# error. This script checks exit codes explicitly, so keep the legacy behaviour.
if (Test-Path Variable:\PSNativeCommandUseErrorActionPreference) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptDir '..\..')).Path

function Fail([string]$Message) {
    throw $Message
}

# Fixture-specific one-line red edit. The red path is deliberately available:
# a reader can reproduce the refusal by hand with the same edit.
$specs = @{
    rust = @{
        EditPath  = 'src/lib.rs'
        EditAppend = "`n// demo change: a second, verified tree.`n"
        RedPath   = 'src/lib.rs'
        RedOld    = 'assert_eq!(add(2, 3), 5);'
        RedNew    = 'assert_eq!(add(2, 3), 6);'
        Needs     = @('cargo')
    }
    node = @{
        EditPath  = 'src/math.js'
        EditAppend = "`n// demo change: a second, verified tree.`n"
        RedPath   = 'test/math.test.js'
        RedOld    = 'assert.equal(add(2, 3), 5);'
        RedNew    = 'assert.equal(add(2, 3), 6);'
        Needs     = @('node')
    }
    python = @{
        EditPath  = 'src/math_util.py'
        EditAppend = "`n# demo change: a second, verified tree.`n"
        RedPath   = 'tests/test_math_util.py'
        RedOld    = 'self.assertEqual(add(2, 3), 5)'
        RedNew    = 'self.assertEqual(add(2, 3), 6)'
        Needs     = @('python')
    }
}

function Invoke-FlowRaw {
    param([string[]]$Arguments)
    $output = & $script:flowBin @Arguments
    $code = $LASTEXITCODE
    $text = (@($output) | Out-String).Trim()
    $json = $null
    if ($text) {
        try { $json = $text | ConvertFrom-Json } catch { $json = $null }
    }
    [pscustomobject]@{ Exit = $code; Text = $text; Json = $json }
}

function Invoke-Flow {
    param([string[]]$Arguments, [int[]]$Expect = @(0))
    $result = Invoke-FlowRaw -Arguments $Arguments
    if ($Expect -notcontains $result.Exit) {
        Fail "flow $($Arguments -join ' ') exited $($result.Exit); expected $($Expect -join ', ').`n$($result.Text)"
    }
    return $result
}

function Copy-FixtureTree([string]$Source, [string]$Destination) {
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        if ($_.Name -in @('target', 'node_modules', '__pycache__', '.git')) { return }
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
    # Never inherit Flow state from a previous manual run of the fixture.
    foreach ($stale in @('.flow/receipts', '.flow/logs', '.flow/lock', '.flow/journal.jsonl', '.flow/adoption.json')) {
        $path = Join-Path $Destination $stale
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force }
    }
}

function Edit-Text([string]$Path, [string]$Old, [string]$New, [string]$What) {
    $text = [IO.File]::ReadAllText($Path)
    if (-not $text.Contains($Old)) {
        Fail "${What}: could not find the expected text in $Path. The fixture and the demo script have drifted."
    }
    [IO.File]::WriteAllText($Path, $text.Replace($Old, $New))
}

function Invoke-OneFixture([string]$Name) {
    $spec = $specs[$Name]
    $source = Join-Path $scriptDir $Name
    if (-not (Test-Path -LiteralPath (Join-Path $source '.flow/config.json'))) {
        Fail "fixture $Name has no .flow/config.json under $source"
    }

    foreach ($tool in $spec.Needs) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            Fail "fixture $Name needs '$tool' on PATH; it is missing, so no pass can be claimed for this fixture"
        }
    }

    $temp = Join-Path ([IO.Path]::GetTempPath()) ('flow-demo-polyglot-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $temp | Out-Null
    $work = Join-Path $temp 'work'
    $originalLocation = Get-Location
    try {
        Copy-FixtureTree -Source $source -Destination $work

        Push-Location $work
        try {
            & git init -q -b main
            if ($LASTEXITCODE -ne 0) { Fail 'git init failed' }
            & git config user.name 'Flow Polyglot Demo'
            & git config user.email 'flow-demo@example.invalid'
            & git config commit.gpgsign false
            # Pin the fixture checkout to the bytes in the Git tree, so the demo
            # is identical on Windows and Linux (no smudge/clean conversion).
            & git config core.autocrlf false
            & git config core.safecrlf false
            & git add -A
            if ($LASTEXITCODE -ne 0) { Fail 'git add failed on the baseline commit' }
            & git commit -q -m 'polyglot fixture: baseline'
            if ($LASTEXITCODE -ne 0) { Fail 'git commit failed on the baseline' }
            $baseline = (& git rev-parse HEAD).Trim()

            # 1. Policy adoption. The final CLI gates execution on an explicit
            #    adoption; binaries from before that gate refuse `init` when
            #    .flow/ already exists and use the shipped config implicitly.
            $init = Invoke-FlowRaw -Arguments @('init', '--adopt-policy')
            if ($init.Exit -eq 0) {
                $initMode = 'init --adopt-policy'
            }
            else {
                $help = (Invoke-FlowRaw -Arguments @('help')).Text
                if ($help -match 'adopt-policy') {
                    Fail "flow init --adopt-policy failed unexpectedly:`n$($init.Text)"
                }
                if (-not (Test-Path -LiteralPath (Join-Path $work '.flow/config.json'))) {
                    Fail 'this flow binary has no --adopt-policy and the fixture ships no .flow/config.json'
                }
                $initMode = 'pre-adoption binary: shipped .flow/config.json used as-is (documented fallback)'
            }

            # 2. A real change under verification, so the index tree differs from
            #    the baseline commit; publishing an already-published tree is
            #    refused as ALREADY_PUBLISHED.
            $editFile = Join-Path $work $spec.EditPath
            [IO.File]::AppendAllText($editFile, $spec.EditAppend)
            & git add -A
            if ($LASTEXITCODE -ne 0) { Fail 'git add failed after the demo change' }

            $indexTree = (& git write-tree).Trim()
            $run1 = Invoke-Flow -Arguments @('run', '--json') -Expect @(0)
            $receipt1 = $run1.Json
            if ($receipt1.status -ne 'PASSED') { Fail "expected PASSED, got $($receipt1.status)" }
            if ($receipt1.tree_oid -ne $indexTree) {
                Fail "receipt tree $($receipt1.tree_oid) is not the index tree $indexTree"
            }
            foreach ($ob in $receipt1.obligations) {
                if ($ob.required -and $ob.status -ne 'PASSED') {
                    Fail "obligation $($ob.id) is $($ob.status) in a run that claimed PASSED"
                }
            }

            $publish1 = Invoke-Flow -Arguments @('publish', '--expected-old', $baseline, '--json') -Expect @(0)
            if ($publish1.Json.status -ne 'PUBLISHED') { Fail "expected PUBLISHED, got $($publish1.Json.status)" }
            $published = (& git rev-parse refs/heads/main).Trim()
            if ($published -eq $baseline) { Fail 'publish did not move the branch' }
            if ($published -ne $publish1.Json.commit) { Fail 'published ref does not name the returned commit' }
            $publishedTree = (& git rev-parse 'refs/heads/main^{tree}').Trim()
            if ($publishedTree -ne $receipt1.tree_oid) { Fail 'published commit does not carry the verified tree' }

            # 3. The documented red edit: verification fails, publication refuses,
            #    and the ref does not move.
            Edit-Text -Path (Join-Path $work $spec.RedPath) -Old $spec.RedOld -New $spec.RedNew -What 'red edit'
            & git add -A
            if ($LASTEXITCODE -ne 0) { Fail 'git add failed after the red edit' }
            $redTree = (& git write-tree).Trim()
            if ($redTree -eq $receipt1.tree_oid) { Fail 'the red edit did not change the index tree' }

            $run2 = Invoke-Flow -Arguments @('run', '--json') -Expect @(1)
            $receipt2 = $run2.Json
            if ($receipt2.status -ne 'FAILED') { Fail "expected FAILED on the red edit, got $($receipt2.status)" }
            $failing = @($receipt2.obligations | Where-Object { $_.status -ne 'PASSED' } | ForEach-Object { "$($_.id)=$($_.status)" })
            if ($failing.Count -eq 0) { Fail 'the red run is FAILED but every obligation reports PASSED' }

            $publish2 = Invoke-Flow -Arguments @('publish', '--expected-old', $published, '--json') -Expect @(1)
            if ($publish2.Json.status -ne 'REFUSED') { Fail "expected REFUSED, got $($publish2.Json.status)" }
            if (-not $publish2.Json.code) { Fail 'the refusal carries no machine-readable code' }
            $afterRefusal = (& git rev-parse refs/heads/main).Trim()
            if ($afterRefusal -ne $published) { Fail 'the ref moved on a refused publication' }

            [pscustomobject]@{
                Fixture      = $Name
                InitMode     = $initMode
                Tree         = $receipt1.tree_oid
                Obligations  = (@($receipt1.obligations | ForEach-Object { "$($_.id)=$($_.status)" }) -join ', ')
                Published    = $published
                RedTree      = $redTree
                Failing      = ($failing -join ', ')
                RefusalCode  = $publish2.Json.code
                Run1Exit     = $run1.Exit
                Run2Exit     = $run2.Exit
                Publish1Exit = $publish1.Exit
                Publish2Exit = $publish2.Exit
            }
        }
        finally {
            Pop-Location
        }
    }
    finally {
        Set-Location $originalLocation
        if ($KeepTemp) {
            Write-Host "kept temporary directory: $temp"
        }
        else {
            $resolved = (Resolve-Path -LiteralPath $temp).Path
            $tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
            if ($resolved.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase) -and
                (Split-Path -Leaf $resolved).StartsWith('flow-demo-polyglot-')) {
                Remove-Item -LiteralPath $resolved -Recurse -Force
            }
            else {
                Write-Warning "refusing to remove unexpected temporary path: $resolved"
            }
        }
    }
}

# ------------------------------------------------------------------ main
$names = if ($Fixture -eq 'all') { @('rust', 'node', 'python') } else { @($Fixture) }

if (-not $env:FLOW_BIN) {
    Write-Host 'building flow (target directory kept outside the repository)...'
    $targetDir = Join-Path ([IO.Path]::GetTempPath()) ('flow-demo-target-' + [guid]::NewGuid().ToString('N'))
    & cargo build --offline --manifest-path (Join-Path $repoRoot 'Cargo.toml') --target-dir $targetDir
    if ($LASTEXITCODE -ne 0) { Remove-Item -LiteralPath $targetDir -Recurse -Force -ErrorAction SilentlyContinue; Fail 'cargo build of the flow CLI failed' }
    $script:flowBin = Join-Path $targetDir 'debug/flow.exe'
    if (-not (Test-Path -LiteralPath $script:flowBin)) { $script:flowBin = Join-Path $targetDir 'debug/flow' }
    if (-not (Test-Path -LiteralPath $script:flowBin)) { Fail "flow binary not found under $targetDir/debug" }
    $script:flowTarget = $targetDir
}
else {
    $script:flowBin = (Resolve-Path -LiteralPath $env:FLOW_BIN).Path
    $script:flowTarget = $null
}

$results = @()
$failures = @()
foreach ($name in $names) {
    Write-Host ''
    Write-Host "== fixture: $name"
    try {
        $results += Invoke-OneFixture -Name $name
    }
    catch {
        $failures += "${name}: $($_.Exception.Message)"
        Write-Host "FAIL: $name"
    }
}

Write-Host ''
foreach ($r in $results) {
    Write-Host "Flow polyglot demo — fixture: $($r.Fixture)"
    Write-Host "  flow binary         $script:flowBin"
    Write-Host "  policy adoption     $($r.InitMode)"
    Write-Host "  run (index tree)    PASSED     exit=$($r.Run1Exit)  tree=$($r.Tree)"
    Write-Host "  obligations         $($r.Obligations)"
    Write-Host "  publish             PUBLISHED  exit=$($r.Publish1Exit)  ref=refs/heads/main  commit=$($r.Published)"
    Write-Host "  run (red edit)      FAILED     exit=$($r.Run2Exit)  tree=$($r.RedTree)"
    Write-Host "  failing checks      $($r.Failing)"
    Write-Host "  publish (red edit)  REFUSED    exit=$($r.Publish2Exit)  code=$($r.RefusalCode)"
    Write-Host "  ref after refusal   unchanged"
    Write-Host ''
}

if ($script:flowTarget) {
    $resolved = (Resolve-Path -LiteralPath $script:flowTarget).Path
    $tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if ($resolved.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $resolved).StartsWith('flow-demo-target-')) {
        Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($failures.Count -gt 0) {
    Write-Host ''
    foreach ($failure in $failures) { Write-Host "FAILED: $failure" }
    Fail ("polyglot demo failed for: " + (($failures | ForEach-Object { ($_ -split ':')[0] }) -join ', '))
}

Write-Host "RESULT: PASS — $($results.Count) fixture(s) verified, published, and refused after the documented red edit."
