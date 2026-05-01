param(
    [string]$SmokeRoot = (Join-Path $env:TEMP "me-inventory-sync-smoke")
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repoRoot "backend\Cargo.toml"
$tempRoot = [System.IO.Path]::GetFullPath($env:TEMP)
$resolvedSmokeRoot = [System.IO.Path]::GetFullPath($SmokeRoot)

if (-not $resolvedSmokeRoot.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "SmokeRoot must stay under the current user's TEMP folder. Received: $resolvedSmokeRoot"
}

if (Test-Path -LiteralPath $resolvedSmokeRoot) {
    Remove-Item -LiteralPath $resolvedSmokeRoot -Recurse -Force
}

New-Item -ItemType Directory -Path $resolvedSmokeRoot | Out-Null
$env:ME_SYNC_SMOKE_ROOT = $resolvedSmokeRoot

Write-Host "ME Inventory one-machine sync smoke"
Write-Host "Smoke root: $resolvedSmokeRoot"

try {
    cargo test --manifest-path $manifestPath --test shared_sync_flow scripted_one_machine_smoke_uses_env_shared_root -- --nocapture
    $exitCode = $LASTEXITCODE
} finally {
    Remove-Item Env:\ME_SYNC_SMOKE_ROOT -ErrorAction SilentlyContinue
}

$opsDir = Join-Path $resolvedSmokeRoot "shared-root\shared\inventory\ops"
Write-Host ""
Write-Host "Shared ops folder: $opsDir"
if (Test-Path -LiteralPath $opsDir) {
    Get-ChildItem -LiteralPath $opsDir -Recurse -Filter "*.op.json" |
        Sort-Object FullName |
        ForEach-Object { Write-Host ("  " + $_.FullName) }
} else {
    Write-Host "  No operation files were created."
}

if ($exitCode -ne 0) {
    throw "One-machine sync smoke failed with exit code $exitCode."
}

Write-Host ""
Write-Host "PASS: clients converged, stale update was logged as a conflict, delete and newer restore succeeded."
