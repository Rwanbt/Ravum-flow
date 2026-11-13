# Wave 9 — serialised Windows stability campaign.
#
# Wave 8 ran ten full-suite passes while a mutation campaign occupied the same
# 16 GB host; run 7 failed with `ERROR_NO_SYSTEM_RESOURCES` and
# `ERROR_COMMITMENT_LIMIT` from `proposals.rs`, which is a host-resource artefact,
# not a product failure. The item was deliberately left unproven rather than
# relabelled. This campaign runs the same schedule on an otherwise idle host, so
# the "no failure" claim has one cause: the product.
#
# Evidence only: nothing is edited, committed or pushed by this script.
$ErrorActionPreference = 'Stop'
Set-Location 'D:\App\Ravum-flow'
$dir = 'experiments\quality\stability-w9'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$csv = Join-Path $dir 'windows-stability.csv'
"run,threads,exit,passed,failed,ignored,seconds,concurrent_load" | Set-Content $csv
Add-Content $csv -Value "# serialised: no mutation/coverage/build campaign ran while this script ran"
Add-Content $csv -Value "# head=$(git rev-parse HEAD)"

foreach ($i in 1..10) {
    if ($i % 2 -eq 0) { $env:RUST_TEST_THREADS = '1'; $threads = '1' }
    else { Remove-Item Env:RUST_TEST_THREADS -ErrorAction SilentlyContinue; $threads = 'default' }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $text = & cargo test --locked --offline --all-targets 2>&1 | Out-String
    $exit = $LASTEXITCODE
    $sw.Stop()
    $sum = { param($k) ([regex]::Matches($text, "(\d+) $k") | ForEach-Object { [int]::Parse($_.Groups[1].Value) } | Measure-Object -Sum).Sum }
    $p = & $sum 'passed'; $f = & $sum 'failed'; $g = & $sum 'ignored'
    $secs = [math]::Round($sw.Elapsed.TotalSeconds, 1)
    "run=$i threads=$threads exit=$exit passed=$p failed=$f ignored=$g seconds=$secs"
    "$i,$threads,$exit,$p,$f,$g,$secs,none" | Add-Content $csv
    if ($exit -ne 0) {
        $text | Set-Content (Join-Path $dir "failure-run-$i.log")
        break
    }
}
"done" | Add-Content $csv
