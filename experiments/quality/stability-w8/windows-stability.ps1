# Wave 8 — Windows stability campaign: repeated full-suite runs, mixed thread counts.
# Evidence only; no source change, nothing committed by this script.
$ErrorActionPreference = 'Stop'
Set-Location 'D:\App\Ravum-flow'
$dir = 'experiments\quality\stability-w8'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$csv = Join-Path $dir 'windows-stability.csv'
"run,threads,exit,passed,failed,ignored,seconds" | Set-Content $csv

foreach ($i in 1..10) {
    if ($i % 2 -eq 0) { $env:RUST_TEST_THREADS = '1'; $threads = '1' }
    else { Remove-Item Env:RUST_TEST_THREADS -ErrorAction SilentlyContinue; $threads = 'default' }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $text = & cargo test --locked --offline --all-targets 2>&1 | Out-String
    $exit = $LASTEXITCODE
    $sw.Stop()
    $sum = { param($k) ([regex]::Matches($text, "(\d+) $k") | ForEach-Object { [int]$_.Groups[1].Value } | Measure-Object -Sum).Sum }
    $p = & $sum 'passed'; $f = & $sum 'failed'; $g = & $sum 'ignored'
    $secs = [math]::Round($sw.Elapsed.TotalSeconds, 1)
    "run=$i threads=$threads exit=$exit passed=$p failed=$f ignored=$g seconds=$secs"
    "$i,$threads,$exit,$p,$f,$g,$secs" | Add-Content $csv
    if ($exit -ne 0) {
        $text | Set-Content (Join-Path $dir "failure-run-$i.log")
    }
}
"done" | Add-Content $csv
