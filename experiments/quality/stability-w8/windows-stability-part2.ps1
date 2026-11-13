# Wave 8 — Windows stability campaign, part 2: six more full-suite runs at the
# default (parallel) thread count, which is what CI runs. Appends to the CSV.
$ErrorActionPreference = 'Stop'
Set-Location 'D:\App\Ravum-flow'
$dir = 'experiments\quality\stability-w8'
$csv = Join-Path $dir 'windows-stability.csv'
foreach ($i in 3..8) {
    Remove-Item Env:RUST_TEST_THREADS -ErrorAction SilentlyContinue
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $text = & cargo test --locked --offline --all-targets 2>&1 | Out-String
    $exit = $LASTEXITCODE
    $sw.Stop()
    $sum = { param($k) ([regex]::Matches($text, "(\d+) $k") | ForEach-Object { [int]$_.Groups[1].Value } | Measure-Object -Sum).Sum }
    $p = & $sum 'passed'; $f = & $sum 'failed'; $g = & $sum 'ignored'
    $secs = [math]::Round($sw.Elapsed.TotalSeconds, 1)
    "run=$i threads=default exit=$exit passed=$p failed=$f ignored=$g seconds=$secs"
    "$i,default,$exit,$p,$f,$g,$secs" | Add-Content $csv
    if ($exit -ne 0) { $text | Set-Content (Join-Path $dir "failure-run-$i.log") }
}
"done" | Add-Content $csv
