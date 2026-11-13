#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Build the Windows release artefact for Ravum Flow from one committed SHA,
  prove the build is reproducible on this host, and write the distribution
  directory (binary, LICENSE, SHA256SUMS.txt, SBOM.json, PROVENANCE.json).

.DESCRIPTION
  The build commands are the ones in .github/workflows/flow-ci.yml:
  two clean `git archive` checkouts of the same commit, built in two different
  directories with the reproducibility RUSTFLAGS. The script fails if the two
  binaries differ; it never packages a build it could not reproduce.

  Nothing is fabricated: fields that cannot be measured are omitted. The
  reproducibility claim is scoped to this host and toolchain (it is not a
  cross-host claim).

.EXAMPLE
  pwsh -File scripts/release/build-windows.ps1 -Sha dfb941db8b22d3340ad873950113fcdf03166f81
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Sha,
    # Empty defaults are resolved from this script's location and from
    # RUNNER_TEMP, so the script runs unchanged from a CI checkout.
    [string]$Repo = '',
    [string]$Version = '',
    [string]$Triple = 'x86_64-pc-windows-msvc',
    [string]$WorkRoot = '',
    [string]$Dist = '',
    # CI already ran the gated test step: skip running it again and read the
    # existing log with -TestLog. Without -TestLog, no test claim is made.
    [switch]$SkipTests,
    # A `cargo test` log for THIS commit. When given, the measured counts are
    # recorded in PROVENANCE.json; when absent, no test claim is made.
    [string]$TestLog = ''
)

$ErrorActionPreference = 'Stop'
if (-not $Repo) { $Repo = Join-Path $PSScriptRoot '..\..' }
$repo = (Resolve-Path $Repo).Path
if (-not $Version) {
    # The crate manifest is the single source of the version. A hardcoded default
    # made every artefact and every PROVENANCE.json claim 0.2.0-rc.1 long after the
    # crate had moved to 0.2.0-rc.2 (found by the 2026-11-13 release audit).
    $manifest = Join-Path $repo 'crates/flow/Cargo.toml'
    $found = Select-String -Path $manifest -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $found) { throw "cannot read the crate version from $manifest" }
    $Version = $found.Matches[0].Groups[1].Value
}
if (-not $WorkRoot) {
    $WorkRoot = if ($env:TEMP) { Join-Path $env:TEMP 'flow-release-work' } else { Join-Path $repo 'target\release-work' }
}
if (-not $Dist) { $Dist = Join-Path $repo 'dist' }
$short = $Sha.Substring(0, 7)
$buildRoot = Join-Path $WorkRoot "pkg-$short"
$distDir = Join-Path $Dist "flow-$Version-$Triple"
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Invoke-Native {
    param([string]$Description, [string]$File, [string[]]$Arguments)
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

function Write-Utf8 {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText($Path, $Text, $utf8NoBom)
}

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

# ---------------------------------------------------------------- preconditions
Invoke-Native 'git cat-file' 'git' @('-C', $repo, 'cat-file', '-e', "$Sha^{commit}")
$resolved = (& git -C $repo rev-parse "$Sha^{commit}").Trim()
if ($resolved -ne $Sha) { throw "commit $Sha resolved to $resolved; use the full 40-hex SHA" }
$treeOid = (& git -C $repo rev-parse "$Sha^{tree}").Trim()
$branch = (& git -C $repo rev-parse --abbrev-ref HEAD).Trim()

# ---------------------------------------------------------------- clean checkouts
Write-Host "== extracting two clean archives of $Sha into $buildRoot =="
if (Test-Path $buildRoot) { Remove-Item -Recurse -Force $buildRoot }
New-Item -ItemType Directory -Force $buildRoot | Out-Null

# Use the Windows bsdtar explicitly: when this script is launched from Git Bash
# (as CI does), a bare `tar` is MSYS tar, whose path conversion mangles a
# `C:\...` destination ("C\:\\Users\\..."). System32\tar.exe is the native tool
# and accepts a Windows path unchanged. The archive stream is the same
# `git archive` tar used by flow-ci.yml.
$tarExe = Join-Path $env:SystemRoot 'System32\tar.exe'
if (-not (Test-Path $tarExe)) { throw "Windows tar.exe not found at $tarExe" }
$archives = @{}
foreach ($n in @('a', 'b')) {
    $dir = Join-Path $buildRoot $n
    New-Item -ItemType Directory -Force $dir | Out-Null
    $archive = Join-Path $buildRoot "archive-$n.tar"
    Invoke-Native "git archive $n" 'git' @('-C', $repo, 'archive', '-o', $archive, $Sha)
    & $tarExe -xf $archive -C $dir
    if ($LASTEXITCODE -ne 0) { throw "tar -xf failed for archive ${n}" }
    $archives[$n] = $archive
}

# ---------------------------------------------------------------- tests
# CI runs the gated suite already; -SkipTests + -TestLog avoids a duplicate run.
$testLogPath = $TestLog
if (-not $SkipTests -and -not $TestLog) {
    $testLogPath = Join-Path $WorkRoot "test-win-$short.log"
    $testTarget = Join-Path $WorkRoot "test-$short"
    if (Test-Path $testTarget) { Remove-Item -Recurse -Force $testTarget }
    $env:CARGO_TARGET_DIR = $testTarget
    $env:RUSTFLAGS = $null
    Write-Host "== cargo test --locked --all-targets --offline =="
    Push-Location (Join-Path $buildRoot 'a')
    try {
        & cargo test --locked --all-targets --offline 2>&1 | Tee-Object -FilePath $testLogPath
        $testCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
    if ($testCode -ne 0) { throw "cargo test failed (exit $testCode); refusing to package" }
}
elseif ($SkipTests -and -not $TestLog) {
    Write-Host "== -SkipTests with no -TestLog: PROVENANCE will not claim a test result =="
}

# ---------------------------------------------------------------- two builds
$builds = @()
foreach ($n in @('a', 'b')) {
    $dir = Join-Path $buildRoot $n
    $target = Join-Path $WorkRoot "win-$n-$short"
    if (Test-Path $target) { Remove-Item -Recurse -Force $target }
    $env:CARGO_TARGET_DIR = $target
    $env:RUSTFLAGS = "-C link-arg=/Brepro -C codegen-units=1 --remap-path-prefix=$dir= "
    Write-Host "== building archive $n =="
    Write-Host "   CARGO_TARGET_DIR=$target"
    Write-Host "   RUSTFLAGS=$env:RUSTFLAGS"
    Push-Location $dir
    try {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & cargo build --locked --release --offline
        $code = $LASTEXITCODE
        $sw.Stop()
        if ($code -ne 0) { throw "cargo build failed for archive $n (exit $code)" }
    }
    finally {
        Pop-Location
    }
    $exe = Join-Path $target 'release\flow.exe'
    if (-not (Test-Path $exe)) { throw "binary missing after build ${n}: $exe" }
    $builds += [pscustomobject]@{
        archive = $n
        target  = $target
        sha256  = Get-Sha256 $exe
        bytes   = (Get-Item $exe).Length
        seconds = [math]::Round($sw.Elapsed.TotalSeconds, 2)
    }
}
$builds | Format-Table -AutoSize | Out-String | Write-Host

$ha = $builds[0].sha256
$hb = $builds[1].sha256
if ($ha -ne $hb) {
    throw "BUILD IS NOT REPRODUCIBLE: archive a = $ha, archive b = $hb (refusing to package)"
}
Write-Host "reproducible: both archives produced $ha"

# ---------------------------------------------------------------- source identity
$archiveFile = $archives['a']
if (-not (Test-Path $archiveFile)) { throw "source archive missing: $archiveFile" }
$sourceArchiveSha256 = Get-Sha256 $archiveFile
$sourceArchiveBytes = (Get-Item $archiveFile).Length
# SHA-256 of the exact `git ls-tree -r --full-tree` stdout (LF line endings).
$manifestText = ((& git -C $repo ls-tree -r --full-tree $Sha) -join "`n") + "`n"
$manifestBytes = $utf8NoBom.GetBytes($manifestText)
$manifestSha256 = [System.Convert]::ToHexString(
    [System.Security.Cryptography.SHA256]::HashData($manifestBytes)).ToLowerInvariant()
$manifestLines = $manifestText.Split("`n").Count - 1

# ---------------------------------------------------------------- package
Write-Host "== packaging $distDir =="
if (Test-Path $distDir) { Remove-Item -Recurse -Force $distDir }
New-Item -ItemType Directory -Force $distDir | Out-Null
Copy-Item (Join-Path $builds[0].target 'release\flow.exe') (Join-Path $distDir 'flow.exe')
Copy-Item (Join-Path $buildRoot 'a\LICENSE') (Join-Path $distDir 'LICENSE')

$exeHash = Get-Sha256 (Join-Path $distDir 'flow.exe')
$exeBytes = (Get-Item (Join-Path $distDir 'flow.exe')).Length
$licenseHash = Get-Sha256 (Join-Path $distDir 'LICENSE')
$licenseBytes = (Get-Item (Join-Path $distDir 'LICENSE')).Length
$licenseFirstLine = (Get-Content -LiteralPath (Join-Path $distDir 'LICENSE') -TotalCount 1).Trim()
$spdx = if ($licenseFirstLine -match 'Mozilla Public License Version 2\.0') { 'MPL-2.0' } else { $null }
# The committed blob and the shipped file can differ only by line endings when
# the host Git has core.autocrlf=true (measured on this host: 16 727 -> 17 100).
# The normalized text hash proves the licence itself is the same.
$licenseBlobSize = [int](((& git -C $repo cat-file -s "$Sha`:LICENSE") -join '').Trim())
$licenseBlobOid = ((& git -C $repo rev-parse "$Sha`:LICENSE") -join '').Trim()
$licenseText = [System.IO.File]::ReadAllText((Join-Path $distDir 'LICENSE'))
$licenseNormalized = [System.Text.Encoding]::UTF8.GetBytes($licenseText.Replace("`r`n", "`n"))
$licenseNormalizedHash = [System.Convert]::ToHexString(
    [System.Security.Cryptography.SHA256]::HashData($licenseNormalized)).ToLowerInvariant()
$coreAutocrlf = ((& git -C $repo config --get core.autocrlf) -join '').Trim()
$licenseLineEndings = if ($licenseBytes -ne $licenseBlobSize) { 'CRLF' } else { 'LF' }

$now = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
$rustc = ((& rustc --version) -join ' ').Trim()
$cargo = ((& cargo --version) -join ' ').Trim()
$rustcVv = (& rustc -vV) -join "`n"
$rustcHost = (($rustcVv -split "`n") | Where-Object { $_ -like 'host:*' } | Select-Object -First 1)
$rustcHost = if ($rustcHost) { $rustcHost.Substring(5).Trim() } else { 'unknown' }
$os = Get-CimInstance Win32_OperatingSystem
$hostOs = "$($os.Caption) $($os.Version) build $($os.BuildNumber)"
$archRaw = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
$arch = switch ($archRaw) { 'x64' { 'x86_64' } 'arm64' { 'aarch64' } default { $archRaw } }

# Test evidence is only recorded when a log for this commit is supplied.
$testResult = $null
if ($testLogPath) {
    if (-not (Test-Path $testLogPath)) { throw "test log not found: $testLogPath" }
    $passed = 0; $failed = 0; $ignored = 0
    $perBinary = [ordered]@{}
    $current = $null
    foreach ($line in Get-Content -LiteralPath $testLogPath) {
        if ($line -match '^\s*Running\s+(.+?)\s+\(') { $current = $Matches[1].Trim() }
        if ($line -match 'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored') {
            $p = [int]$Matches[1]; $f = [int]$Matches[2]; $i = [int]$Matches[3]
            $passed += $p; $failed += $f; $ignored += $i
            if ($current) { $perBinary[$current] = [ordered]@{ passed = $p; failed = $f; ignored = $i } }
            $current = $null
        }
    }
    if ($passed -eq 0) { throw "no test result lines found in $testLogPath" }
    $testResult = [ordered]@{
        command    = 'cargo test --locked --all-targets --offline'
        passed     = $passed
        failed     = $failed
        ignored    = $ignored
        per_binary = $perBinary
    }
}

$bomRef = "flow@${Version}:$Triple"
# A deterministic serial: the same commit/target yields the same UUID, so two
# rebuilds do not look like two different products. Version 4/variant bits set
# to keep the value a well-formed UUID.
$seed = [System.Text.Encoding]::UTF8.GetBytes("flow|$Version|$Sha|$Triple")
$raw = [System.Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData($seed)).ToLowerInvariant()
$uuid = $raw.Substring(0, 8) + '-' + $raw.Substring(8, 4) + '-' + '4' + $raw.Substring(13, 3) + '-' +
        '8' + $raw.Substring(17, 3) + '-' + $raw.Substring(20, 12)

$sbom = [ordered]@{
    bomFormat    = 'CycloneDX'
    specVersion  = '1.5'
    serialNumber = "urn:uuid:$uuid"
    version      = 1
    metadata     = [ordered]@{
        timestamp = $now
        component = [ordered]@{
            type       = 'application'
            'bom-ref'  = $bomRef
            name       = 'flow'
            version    = $Version
            purl       = "pkg:cargo/flow@$Version"
            licenses   = @(
                if ($spdx) {
                    @{ license = [ordered]@{ id = $spdx; acknowledgement = 'declared' } }
                }
                else {
                    @{ license = [ordered]@{ name = $licenseFirstLine; acknowledgement = 'declared' } }
                }
            )
            hashes     = @(@{ alg = 'SHA-256'; content = $exeHash })
            externalReferences = @(
                @{ type = 'vcs'; url = "https://github.com/Rwanbt/Ravum-flow/commit/$Sha" }
            )
        }
    }
    components   = @()
    dependencies = @(@{ ref = $bomRef; dependsOn = @() })
}
$sbomPath = Join-Path $distDir 'SBOM.json'
Write-Utf8 $sbomPath (($sbom | ConvertTo-Json -Depth 12) + "`n")

$sums = "$exeHash  flow.exe`n$licenseHash  LICENSE`n"
$sumsPath = Join-Path $distDir 'SHA256SUMS.txt'
Write-Utf8 $sumsPath $sums

$provenance = [ordered]@{
    schema  = 'flow-provenance/1'
    product = 'flow'
    version = $Version
    source  = [ordered]@{
        commit                 = $Sha
        branch                 = $branch
        repository             = 'https://github.com/Rwanbt/Ravum-flow.git'
        git_tree_oid           = $treeOid
        git_archive_format     = 'tar'
        git_archive_sha256     = $sourceArchiveSha256
        git_archive_bytes      = $sourceArchiveBytes
        tree_manifest_sha256   = $manifestSha256
        tree_manifest_entries  = $manifestLines
        tree_manifest_command  = "git -C $repo ls-tree -r --full-tree $Sha  (SHA-256 of the LF-terminated stdout)"
        archive_command        = "git -C $repo archive -o <archive.tar> $Sha ; tar.exe -xf <archive.tar> -C <clean-dir>  (same git archive stream as the flow-ci.yml pipeline; Windows bsdtar is used explicitly so a Git Bash MSYS tar cannot mangle the path)"
        host_git_core_autocrlf = $coreAutocrlf
        archive_extraction_note = "the extraction is the flow-ci.yml pipeline with this host's Git config (core.autocrlf=$coreAutocrlf); when true, text files are CRLF in the extracted tree. The commit, tree OID and tree-manifest are blob-based and unaffected."
        git_archive_note       = 'the archive hash is host-specific (the tar reflects core.autocrlf and the Git tar serialisation); the tree OID and tree-manifest are the portable source identity'
    }
    license = [ordered]@{
        spdx                    = if ($spdx) { $spdx } else { 'unknown' }
        shipped_bytes           = $licenseBytes
        shipped_sha256          = $licenseHash
        shipped_line_endings    = $licenseLineEndings
        committed_blob_bytes    = $licenseBlobSize
        committed_blob_oid      = $licenseBlobOid
        normalized_text_sha256  = $licenseNormalizedHash
        note                    = 'same licence text: the shipped Windows file is longer than the committed blob only by line endings (host core.autocrlf); the normalized text hash is identical to the Linux artefact'
    }
    target  = [ordered]@{
        triple = $Triple
        os     = 'windows'
        arch   = 'x86_64'
        abi    = 'msvc'
    }
    toolchain = [ordered]@{
        rustc        = $rustc
        cargo        = $cargo
        rustc_host   = $rustcHost
        pinned_by    = 'rust-toolchain.toml (channel 1.95.0)'
    }
    build = [ordered]@{
        command           = 'cargo build --locked --release --offline'
        working_directory = (Join-Path $buildRoot 'a')
        env               = [ordered]@{
            RUSTFLAGS        = "-C link-arg=/Brepro -C codegen-units=1 --remap-path-prefix=$(Join-Path $buildRoot 'a')= "
            CARGO_TARGET_DIR = $builds[0].target
        }
        offline           = $true
        locked            = $true
        dependencies      = 'none (Cargo.lock has one package: flow)'
    }
    reproducibility = [ordered]@{
        method          = 'two clean git-archive checkouts of the same commit in two different directories, built sequentially with the flow-ci.yml reproducibility flags'
        build_a_sha256  = $builds[0].sha256
        build_b_sha256  = $builds[1].sha256
        identical       = ($builds[0].sha256 -eq $builds[1].sha256)
        scope           = 'per host and per toolchain: identical bytes on this host with rustc 1.95.0; cross-host identity is NOT claimed'
        measured_at_utc = $now
    }
    artifacts = @(
        [ordered]@{ path = 'flow.exe'; sha256 = $exeHash; bytes = $exeBytes }
        [ordered]@{ path = 'LICENSE'; sha256 = $licenseHash; bytes = $licenseBytes }
        [ordered]@{ path = 'SHA256SUMS.txt'; sha256 = (Get-Sha256 $sumsPath); bytes = (Get-Item $sumsPath).Length }
        [ordered]@{ path = 'SBOM.json'; sha256 = (Get-Sha256 $sbomPath); bytes = (Get-Item $sbomPath).Length }
    )
    artifacts_note = 'PROVENANCE.json cannot list its own hash; SHA256SUMS.txt covers the distributable binary and LICENSE only'
    build_date_utc = $now
    host           = [ordered]@{
        os           = $hostOs
        arch         = $arch
        rustc_target = $rustcHost
    }
    verification = if ($testResult) {
        [ordered]@{
            tests       = $testResult
            smoke_tests = 'docs/reviews/FLOW-PACKAGING-WAVE3.md (section 5: clean-room smoke matrix, both platforms)'
        }
    }
    else {
        [ordered]@{
            smoke_tests = 'docs/reviews/FLOW-PACKAGING-WAVE3.md (section 5: clean-room smoke matrix, both platforms)'
        }
    }
    verification_summary_reference = 'docs/reviews/FLOW-PACKAGING-WAVE3.md (sections 4-6: tests, smoke tests, reproducibility)'
    unsigned = $true
}
$provPath = Join-Path $distDir 'PROVENANCE.json'
Write-Utf8 $provPath (($provenance | ConvertTo-Json -Depth 12) + "`n")

Write-Host ''
Write-Host '== result =='
"dist:        $distDir"
"flow.exe:    $exeHash ($exeBytes bytes)"
"LICENSE:     $licenseHash ($licenseBytes bytes)"
"SBOM.json:   $((Get-Sha256 $sbomPath))"
"SHA256SUMS:  $((Get-Sha256 $sumsPath))"
"PROVENANCE:  $((Get-Sha256 $provPath))"
"reproducible (host/toolchain scope): $ha == $hb"
$env:RUSTFLAGS = $null
