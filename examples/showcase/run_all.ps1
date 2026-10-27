# Runner script for all Pilot Concept Showcases
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "         PILOT INTERACTIVE CONCEPT SHOWCASE RUNNER        " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$scripts = Get-ChildItem -Path $PSScriptRoot -Filter "*.pilot" | Sort-Object Name

foreach ($script in $scripts) {
    Write-Host "`n>>> RUNNING: $($script.Name) <<<`n" -ForegroundColor Yellow
    pilot $script.FullName
    Write-Host "----------------------------------------------------------" -ForegroundColor DarkGray
}

Write-Host "`nAll Concept Showcases completed successfully!`n" -ForegroundColor Green
