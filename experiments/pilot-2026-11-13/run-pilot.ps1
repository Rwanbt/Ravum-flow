#requires -Version 5
<#
.SYNOPSIS
    Real-project pilot for flow: a disposable clone of this repository plus a
    25 000-file repository that the old envelope refused.

.DESCRIPTION
    Every number this script prints is measured on this host, on the head that is
    checked out, and is written to results.json with the command that produced it.
    It runs only in disposable directories under -Work; it never touches the
    repository it was launched from.

    The two scenarios answer the two questions the readiness audit left open:
      A. a real Rust project (this repository, ~2 700 files) through the whole
         cycle: init, run, publish, two agents on disjoint files, and a collision;
      B. a repository above the old 20 000-file budget, to prove the streaming
         comparison end to end rather than only in unit tests.
#>
param(
    [string]$Flow = "",
    [string]$Work = "",
    [switch]$SkipHeavy
)

$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
if (-not $Flow) { $Flow = Join-Path $repo "target\release\flow.exe" }
if (-not $Work) { $Work = Join-Path ([System.IO.Path]::GetTempPath()) ("flow-pilot-" + [guid]::NewGuid().ToString("N").Substring(0, 8)) }
New-Item -ItemType Directory -Force -Path $Work | Out-Null

$results = [ordered]@{
    schema    = "flow-pilot/1"
    date      = (Get-Date).ToString("o")
    host      = @{ os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription; cpus = [Environment]::ProcessorCount }
    flow      = $null
    scenarios = [ordered]@{}
}
if (-not (Test-Path $Flow)) { throw "flow binary not found: $Flow" }
$results.flow = (& $Flow version | ConvertFrom-Json)

function Invoke-Flow {
    param([string]$Dir, [string[]]$Arguments)
    Push-Location $Dir
    try {
        $start = Get-Date
        $out = & $Flow @Arguments 2>&1 | Out-String
        $ms = [int]((Get-Date) - $start).TotalMilliseconds
        $code = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
    return @{ code = $code; out = $out; ms = $ms; args = ($Arguments -join " ") }
}

function Invoke-Git {
    param([string]$Dir, [string[]]$Arguments)
    $out = & git -C $Dir @Arguments 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed: $out" }
    return $out.Trim()
}

function Get-Json { param([string]$Text) return ($Text | ConvertFrom-Json) }

Write-Host "work directory: $Work"
Write-Host "flow: $Flow"

# ---------------------------------------------------------------- scenario A
Write-Host "`n=== A. this repository (real project) ==="
$a = Join-Path $Work "real"
$t0 = Get-Date
Invoke-Git $Work @("clone", "--quiet", "--no-hardlinks", $repo, $a) | Out-Null
$cloneMs = [int]((Get-Date) - $t0).TotalMilliseconds
$files = (Invoke-Git $a @("ls-files")).Split("`n").Count
$base = Invoke-Git $a @("rev-parse", "HEAD")
Write-Host "cloned $files tracked files at $($base.Substring(0,8)) in $cloneMs ms"

$init = Invoke-Flow $a @("init")
$config = @{
    schema             = "flow-config/1"
    policy_version     = "FLOW-POLICY-1"
    execution_boundary = "local-trusted"
    obligations        = @(
        @{ id = "metadata"; program = "cargo"; args = @("metadata", "--format-version", "1", "--no-deps", "--offline"); required = $true; timeout_s = 120 }
        @{ id = "fmt"; program = "cargo"; args = @("fmt", "--all", "--check"); required = $true; timeout_s = 120 }
    )
    disabled           = $false
} | ConvertTo-Json -Depth 6
Set-Content -LiteralPath (Join-Path $a ".flow\config.json") -Value $config -Encoding utf8
$approve = Invoke-Flow $a @("approve")

# A real change in the index: `flow run` verifies the INDEX tree and `publish`
# moves a ref, so a cycle with nothing staged is `ALREADY_PUBLISHED` by design.
Set-Content -LiteralPath (Join-Path $a "docs\pilot-cycle.md") -Value "# pilot`n`nA first verified change.`n" -Encoding utf8
Invoke-Git $a @("add", "-A") | Out-Null

$run = Invoke-Flow $a @("run")
$runJson = Get-Json $run.out
$publish = Invoke-Flow $a @("publish", "--expected-old", $base)
$publishJson = Get-Json $publish.out

$results.scenarios.real = [ordered]@{
    tracked_files = $files
    clone_ms      = $cloneMs
    init          = @{ code = $init.code; ms = $init.ms }
    approve       = @{ code = $approve.code; ms = $approve.ms }
    run           = @{ code = $run.code; ms = $run.ms; status = $runJson.status; tree = $runJson.tree_oid; obligations = @($runJson.obligations).Count }
    publish       = @{ code = $publish.code; ms = $publish.ms; status = $publishJson.status; code_field = $publishJson.code }
}
Write-Host ("run: {0} in {1} ms, tree {2}; publish: {3} in {4} ms" -f $runJson.status, $run.ms, $runJson.tree_oid, $publishJson.status, $publish.ms)

# Two agents on disjoint files, from the same immutable base.
$newBase = Invoke-Git $a @("rev-parse", "HEAD")
$agentA = Join-Path $Work "agent-a"
$agentB = Join-Path $Work "agent-b"
Invoke-Git $Work @("clone", "--quiet", "--no-hardlinks", $a, $agentA) | Out-Null
Invoke-Git $Work @("clone", "--quiet", "--no-hardlinks", $a, $agentB) | Out-Null
# Two agents, two disjoint new files: a patch that creates a file applies
# whatever the host's line-ending convention does to the existing blobs.
Set-Content -LiteralPath (Join-Path $agentA "docs\pilot-agent-a.md") -Value "# Agent A`n`nDisjoint change.`n" -Encoding utf8
Set-Content -LiteralPath (Join-Path $agentB "docs\pilot-agent-b.md") -Value "# Agent B`n`nDisjoint change.`n" -Encoding utf8
Invoke-Git $agentA @("add", "-A") | Out-Null
Invoke-Git $agentB @("add", "-A") | Out-Null
$patchA = Join-Path $Work "a.patch"
$patchB = Join-Path $Work "b.patch"
[System.IO.File]::WriteAllText($patchA, (Invoke-Git $agentA @("diff", "--binary", $newBase)) + "`n")
[System.IO.File]::WriteAllText($patchB, (Invoke-Git $agentB @("diff", "--binary", $newBase)) + "`n")

$proposeAResult = Invoke-Flow $a @("propose", "--agent", "A", "--task", "T1", "--base", $newBase, "--patch", $patchA, "--key", "a1")
$proposeBResult = Invoke-Flow $a @("propose", "--agent", "B", "--task", "T2", "--base", $newBase, "--patch", $patchB, "--key", "b1")
$proposeA = Get-Json $proposeAResult.out
$proposeB = Get-Json $proposeBResult.out
$assemble = Invoke-Flow $a @("assemble", "--proposal", $proposeA.proposal_id, "--proposal", $proposeB.proposal_id)
$assembleJson = Get-Json $assemble.out
$candidateRun = Invoke-Flow $a @("run", "--candidate", $assembleJson.candidate_id)
$candidateRunJson = Get-Json $candidateRun.out
$candidatePublish = Invoke-Flow $a @("publish", "--candidate", $assembleJson.candidate_id, "--expected-old", $newBase)

$results.scenarios.real.two_agents = [ordered]@{
    base            = $newBase
    propose_a_ms    = $proposeAResult.ms
    propose_b_ms    = $proposeBResult.ms
    assemble_ms     = $assemble.ms
    candidate       = $assembleJson.candidate_id
    candidate_run   = @{ ms = $candidateRun.ms; status = $candidateRunJson.status; tree = $candidateRunJson.tree_oid }
    candidate_publish = @{ code = $candidatePublish.code; status = (Get-Json $candidatePublish.out).status }
}
Write-Host ("two agents: candidate {0} -> run {1} -> publish {2}" -f $assembleJson.candidate_id, $candidateRunJson.status, (Get-Json $candidatePublish.out).status)

# The same path from both agents: the assembly must refuse, not merge silently.
$conflictA = Join-Path $Work "conflict-a"
$conflictB = Join-Path $Work "conflict-b"
Invoke-Git $Work @("clone", "--quiet", "--no-hardlinks", $a, $conflictA) | Out-Null
Invoke-Git $Work @("clone", "--quiet", "--no-hardlinks", $a, $conflictB) | Out-Null
Set-Content -LiteralPath (Join-Path $conflictA "docs\pilot-shared.md") -Value "# version A`n" -Encoding utf8
Set-Content -LiteralPath (Join-Path $conflictB "docs\pilot-shared.md") -Value "# version B`n" -Encoding utf8
Invoke-Git $conflictA @("add", "-A") | Out-Null
Invoke-Git $conflictB @("add", "-A") | Out-Null
$patchCA = Join-Path $Work "ca.patch"
$patchCB = Join-Path $Work "cb.patch"
[System.IO.File]::WriteAllText($patchCA, (Invoke-Git $conflictA @("diff", "--binary", $newBase)) + "`n")
[System.IO.File]::WriteAllText($patchCB, (Invoke-Git $conflictB @("diff", "--binary", $newBase)) + "`n")
$proposeCAResult = Invoke-Flow $a @("propose", "--agent", "A", "--task", "T3", "--base", $newBase, "--patch", $patchCA, "--key", "c1")
$proposeCBResult = Invoke-Flow $a @("propose", "--agent", "B", "--task", "T4", "--base", $newBase, "--patch", $patchCB, "--key", "c2")
$proposeCA = Get-Json $proposeCAResult.out
$proposeCB = Get-Json $proposeCBResult.out
$conflict = Invoke-Flow $a @("assemble", "--proposal", $proposeCA.proposal_id, "--proposal", $proposeCB.proposal_id)
$conflictJson = Get-Json $conflict.out
$results.scenarios.real.collision = [ordered]@{
    code   = $conflict.code
    status = $conflictJson.status
    reason = $conflictJson.reason
    field  = $conflictJson.code
}
Write-Host ("collision: exit {0} status {1} code {2}" -f $conflict.code, $conflictJson.status, $conflictJson.code)

if (-not $SkipHeavy) {
    # A real, slow obligation: the same cycle with the repository's own unit tests.
    $heavyConfig = @{
        schema             = "flow-config/1"
        policy_version     = "FLOW-POLICY-1"
        execution_boundary = "local-trusted"
        obligations        = @(
            @{ id = "unit"; program = "cargo"; args = @("test", "--locked", "--offline", "--bin", "flow"); required = $true; timeout_s = 600 }
            @{ id = "metadata"; program = "cargo"; args = @("metadata", "--format-version", "1", "--no-deps", "--offline"); required = $true; timeout_s = 120 }
        )
        disabled           = $false
    } | ConvertTo-Json -Depth 6
    Set-Content -LiteralPath (Join-Path $a ".flow\config.json") -Value $heavyConfig -Encoding utf8
    Invoke-Flow $a @("approve") | Out-Null
    $head = Invoke-Git $a @("rev-parse", "HEAD")
    $heavy = Invoke-Flow $a @("run")
    $heavyJson = Get-Json $heavy.out
    $results.scenarios.real.heavy_run = @{ ms = $heavy.ms; status = $heavyJson.status; tree = $heavyJson.tree_oid }
    Write-Host ("heavy run (cargo test --bin flow): {0} in {1} ms" -f $heavyJson.status, $heavy.ms)
}

# ---------------------------------------------------------------- scenario B
Write-Host "`n=== B. a repository above the old 20 000-file budget ==="
$b = Join-Path $Work "scale"
New-Item -ItemType Directory -Force -Path $b | Out-Null
Invoke-Git $b @("init", "--quiet") | Out-Null
Invoke-Git $b @("config", "user.email", "pilot@example.invalid") | Out-Null
Invoke-Git $b @("config", "user.name", "Pilot") | Out-Null
$files = 25000
$dirs = 25
$perDir = [int]($files / $dirs)
$t0 = Get-Date
for ($d = 0; $d -lt $dirs; $d++) {
    $dir = Join-Path $b ("src\d{0:d2}" -f $d)
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    for ($f = 0; $f -lt $perDir; $f++) {
        Set-Content -LiteralPath (Join-Path $dir ("f{0:d4}.txt" -f $f)) -Value ("content {0} {1}" -f $d, $f) -NoNewline
    }
}
$genMs = [int]((Get-Date) - $t0).TotalMilliseconds
Invoke-Git $b @("add", "-A") | Out-Null
Invoke-Git $b @("commit", "-q", "-m", "scale fixture") | Out-Null
$bBase = Invoke-Git $b @("rev-parse", "HEAD")
$count = (Invoke-Git $b @("ls-files")).Split("`n").Count
# One staged change, so the cycle has something to publish (an index equal to the
# ref is `ALREADY_PUBLISHED` by design).
Set-Content -LiteralPath (Join-Path $b "pilot-change.txt") -Value "a verified change`n" -Encoding utf8
Invoke-Git $b @("add", "-A") | Out-Null
$initB = Invoke-Flow $b @("init")
$noop = if ($IsWindows -or $env:OS -eq "Windows_NT") { @{ id = "noop"; program = "cmd"; args = @("/C", "exit", "0"); required = $true; timeout_s = 30 } } else { @{ id = "noop"; program = "sh"; args = @("-c", "true"); required = $true; timeout_s = 30 } }
$scaleConfig = @{
    schema             = "flow-config/1"
    policy_version     = "FLOW-POLICY-1"
    execution_boundary = "local-trusted"
    obligations        = @($noop)
    disabled           = $false
} | ConvertTo-Json -Depth 6
Set-Content -LiteralPath (Join-Path $b ".flow\config.json") -Value $scaleConfig -Encoding utf8
Invoke-Flow $b @("approve") | Out-Null
$runB = Invoke-Flow $b @("run")
$runBJson = Get-Json $runB.out
$publishB = Invoke-Flow $b @("publish", "--expected-old", $bBase)
$results.scenarios.scale = [ordered]@{
    files     = $count
    generate_ms = $genMs
    init      = @{ code = $initB.code; ms = $initB.ms }
    run       = @{ code = $runB.code; ms = $runB.ms; status = $runBJson.status; tree = $runBJson.tree_oid }
    publish   = @{ code = $publishB.code; status = (Get-Json $publishB.out).status }
}
Write-Host ("scale: {0} files, run {1} in {2} ms, publish {3}" -f $count, $runBJson.status, $runB.ms, (Get-Json $publishB.out).status)

$outFile = Join-Path $here "results.json"
$results | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $outFile -Encoding utf8
Write-Host "`nresults: $outFile"
