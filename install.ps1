$ErrorActionPreference = 'Stop'

$Version = "v1.1.0"
$BinaryName = "tensara-windows-amd64.exe"
$DownloadUrl = "https://github.com/tensara/tensara-cli/releases/download/$Version/$BinaryName"
$InstallDir = "$env:LOCALAPPDATA\Tensara"

Write-Host "Tensara CLI installer" -ForegroundColor Cyan
Write-Host "=====================" -ForegroundColor Cyan

if (-not (Test-Path $InstallDir)) {
    Write-Host "Creating installation directory: $InstallDir"
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

try {
    Write-Host "Downloading Tensara CLI from $DownloadUrl"
    Invoke-WebRequest -Uri $DownloadUrl -OutFile "$InstallDir\tensara.exe"
    Write-Host "Download completed successfully" -ForegroundColor Green
} catch {
    Write-Host "Failed to download Tensara CLI: $_" -ForegroundColor Red
    exit 1
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "Adding Tensara to your PATH"
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "Added to PATH successfully" -ForegroundColor Green
} else {
    Write-Host "Tensara is already in your PATH" -ForegroundColor Green
}

try {
    $tensaraVersion = & "$InstallDir\tensara.exe" --version 2>&1
    Write-Host "✅ Installation successful!" -ForegroundColor Green
    Write-Host "You can now use 'tensara' from your terminal."
    Write-Host "NOTE: You may need to open a new terminal window for the PATH changes to take effect."
} catch {
    Write-Host "Installation completed, but there was an issue running tensara." -ForegroundColor Yellow
    Write-Host "You may need to open a new terminal window or manually run: $InstallDir\tensara.exe"
}

Write-Host ""
Write-Host "Thank you for installing Tensara CLI!" -ForegroundColor Cyan
Write-Host "Run 'tensara --help' to get started." -ForegroundColor Cyan
