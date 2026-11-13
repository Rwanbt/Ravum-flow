# Wave-6 killchecks: re-test the exact sites the campaign reported, on a tree that
# already contains the assertions that close them, with a wider suite list than the
# campaign used (a survivor is a statement about the selected scope only).
#
# The line numbers are the *current* lines in the killcheck worktree; the campaign
# names are recorded next to them in the wave-6 report. `--bins` is always added by
# killcheck.ps1, so the unit tests that pin the pure helpers run for every module.

$repo = 'D:\App\Ravum-flow-wt\mut-w6-kc'
$script = 'D:\App\Ravum-flow\experiments\quality\mutation-w6\killcheck.ps1'
$out = 'D:\App\Ravum-flow\experiments\quality\mutation-w6'

& $script -Repo $repo -Module state -OutDir $out -Jobs 2 `
    -Lines '445,458,591,661,662,680' -Tests ''
& $script -Repo $repo -Module gitcli -OutDir $out -Jobs 2 `
    -Lines '134,153,177,201,206,331,332,396,425,449,453,523,570,623,640,666,687,758' `
    -Tests 'cli,proposals,operator_surfaces,error_paths,isolation,bounds,git_fidelity,crash_run'
& $script -Repo $repo -Module main -OutDir $out -Jobs 2 `
    -Lines '83,87,281,300,329,343' -Tests 'error_paths,cli,operator_surfaces'
& $script -Repo $repo -Module commands -OutDir $out -Jobs 2 `
    -Lines '217,223,506,539,792,794,863,868,873,942,996,1000,1006,1007,1077,1126,1160,1262,1530,1531,1587,1746,1773,1819,2116,2119,2183,2311' `
    -Tests 'cli,proposals,git_fidelity,operator_surfaces,error_paths'

'KILLCHECKS_DONE' | Set-Content (Join-Path $out 'killchecks-done.txt')
