# Checks every showcase with `pilot check` (see run_all.sh for the rules).
$dir = Split-Path -Parent $MyInvocation.MyCommand.Path
$pilot = if ($env:PILOT_BIN) { $env:PILOT_BIN } else { Join-Path $dir "..\..\target\debug\pilot" }
if (-not (Test-Path $pilot)) { $pilot = "pilot" }
$passed = 0; $seams = 0; $failed = 0
foreach ($script in Get-ChildItem -Path $dir -Filter *.pilot | Sort-Object Name) {
    $out = & $pilot check $script.FullName 2>&1 | Out-String
    $errors = ($out -split "`n") | Where-Object { $_ -match 'error\[' }
    if ($LASTEXITCODE -eq 0) { Write-Host "ok      $($script.Name)"; $passed++ }
    elseif ($errors -and -not ($errors | Where-Object { $_ -notmatch 'SEAM001' })) { Write-Host "seam    $($script.Name)"; $seams++ }
    else { Write-Host "FAIL    $($script.Name)"; $failed++ }
}
Write-Host "$passed passed, $seams stopped at a seam, $failed failed"
if ($failed -gt 0) { exit 1 }
