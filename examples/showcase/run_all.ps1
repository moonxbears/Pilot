# Runner script for all Pilot Concept Showcases (Windows / PowerShell)
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "         PILOT INTERACTIVE CONCEPT SHOWCASE RUNNER        " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$PilotBin = Join-Path $PSScriptRoot "..\..\target\release\pilot.exe"
if (-not (Test-Path $PilotBin)) {
    $PilotBin = Join-Path $PSScriptRoot "..\..\target\debug\pilot.exe"
}
if (-not (Test-Path $PilotBin)) {
    $PilotBin = "pilot"
}

$scripts = Get-ChildItem -Path $PSScriptRoot -Filter "*.pilot" | Sort-Object Name

foreach ($script in $scripts) {
    Write-Host "`n>>> CHECKING: $($script.Name) <<<" -ForegroundColor Yellow
    $out = & $PilotBin check $script.FullName
    Write-Host $out
    if ($out -notmatch ": 0 diagnostics") {
        throw "ERROR: $($script.Name) reported non-zero diagnostics!"
    }
    Write-Host "----------------------------------------------------------" -ForegroundColor DarkGray
}

Write-Host "`nAll $($scripts.Count) Concept Showcases verified with 0 diagnostics!`n" -ForegroundColor Green
