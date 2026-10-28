#!/usr/bin/env pwsh
param(
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')

Push-Location $repoRoot
try {
    $commitCount = (git rev-list --count HEAD 2>$null).Trim()
    if (-not $commitCount) { $commitCount = "0" }
    $shortSha = (git rev-parse --short=7 HEAD 2>$null).Trim()
    if (-not $shortSha) { $shortSha = "unknown" }
    $branch = (git rev-parse --abbrev-ref HEAD 2>$null).Trim()
    if (-not $branch) { $branch = "unknown" }

    $baseVersion = "1.1"
    $csprojPath = Join-Path $repoRoot "Pilot.csproj"
    if (Test-Path $csprojPath) {
        $csprojText = Get-Content $csprojPath -Raw
        if ($csprojText -match '<PilotBaseVersion>([^<]+)</PilotBaseVersion>') {
            $baseVersion = $Matches[1].Trim()
        }
    }

    $semVer = "$baseVersion.$commitCount"
    $fullVersion = "$semVer+$shortSha"

    # Keep vscode-pilot/package.json synchronized with current commit-bound SemVer
    $vsPkgPath = Join-Path $repoRoot "vscode-pilot\package.json"
    if (Test-Path $vsPkgPath) {
        $pkgRaw = Get-Content $vsPkgPath -Raw
        $updatedPkg = [regex]::Replace($pkgRaw, '("version"\s*:\s*)"[^"]+"', "`${1}`"$semVer`"", 1)
        if ($updatedPkg -ne $pkgRaw) {
            [System.IO.File]::WriteAllText($vsPkgPath, $updatedPkg, (New-Object System.Text.UTF8Encoding($false)))
        }
    }

    if (-not $Quiet) {
        Write-Host "[Pilot Version Sync] Active version is now v$fullVersion (commit #$commitCount on $branch)" -ForegroundColor Cyan
    }
}
finally {
    Pop-Location
}
