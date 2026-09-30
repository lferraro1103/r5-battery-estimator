param([switch]$SkipInstaller)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    cargo build --manifest-path src-tauri/Cargo.toml --release --target x86_64-pc-windows-msvc --bin r5-battery-estimator-v2
    if ($LASTEXITCODE -ne 0) { throw 'V2 build failed.' }
    $portableDir = Join-Path $repoRoot 'outputs/R5BatteryEstimatorV2'
    New-Item -ItemType Directory -Path (Join-Path $portableDir 'Assets') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'src-tauri/target/x86_64-pc-windows-msvc/release/r5-battery-estimator-v2.exe') -Destination (Join-Path $portableDir 'R5BatteryEstimatorV2.exe') -Force
    Copy-Item -LiteralPath (Join-Path $repoRoot 'assets/shark-battery.png') -Destination (Join-Path $portableDir 'Assets/shark-battery.png') -Force
    foreach ($file in @('README.md', 'LICENSE')) {
        Copy-Item -LiteralPath (Join-Path $repoRoot $file) -Destination (Join-Path $portableDir $file) -Force
    }
    Write-Host "Portable: $portableDir"
    if (-not $SkipInstaller) {
        $compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
        $compilerPath = if ($compiler) { $compiler.Source } else {
            @(
                (Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 6/ISCC.exe'),
                (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe'),
                (Join-Path $env:ProgramFiles 'Inno Setup 6/ISCC.exe')
            ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        }
        if (-not $compilerPath) { throw 'Portable built. Install Inno Setup 6 to create the installer, or use -SkipInstaller.' }
        if (-not (Test-Path -LiteralPath $compilerPath)) { throw 'Portable built. Install Inno Setup 6 to create the installer, or use -SkipInstaller.' }
        & $compilerPath (Join-Path $repoRoot 'installer/R5BatteryEstimatorV2.iss')
        if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
    }
} finally {
    Pop-Location
}
