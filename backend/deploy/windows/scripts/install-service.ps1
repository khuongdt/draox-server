#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Draox Server — Windows Service Installer

.DESCRIPTION
    Registers draox-server.exe as a Windows Service with automatic startup,
    failure recovery, and environment variable configuration.

.PARAMETER BinaryPath
    Path to draox-server.exe. Default: auto-detect from Program Files.

.PARAMETER ConfigPath
    Path to config.toml. Default: C:\ProgramData\DraoxServer\config\default.toml

.PARAMETER ServiceName
    Windows service name. Default: DraoxServer

.PARAMETER DisplayName
    Service display name. Default: Draox Server

.PARAMETER StartType
    Service start type: Automatic, Manual, Disabled. Default: Automatic

.PARAMETER ServiceAccount
    Account to run the service under. Default: LocalService

.EXAMPLE
    .\install-service.ps1
    .\install-service.ps1 -BinaryPath "D:\custom\draox-server.exe" -StartType Manual
#>

[CmdletBinding()]
param(
    [string]$BinaryPath = "",
    [string]$ConfigPath = "C:\ProgramData\DraoxServer\config\default.toml",
    [string]$ServiceName = "DraoxServer",
    [string]$DisplayName = "Draox Server",
    [ValidateSet("Automatic", "Manual", "Disabled")]
    [string]$StartType = "Automatic",
    [string]$ServiceAccount = "NT AUTHORITY\LocalService"
)

$ErrorActionPreference = "Stop"

# ── Banner ──
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  Draox Server - Windows Service Setup"   -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# ── Auto-detect binary ──
if (-not $BinaryPath) {
    $candidates = @(
        "${env:ProgramFiles}\DraoxServer\bin\draox-server.exe",
        "${env:ProgramFiles(x86)}\DraoxServer\bin\draox-server.exe",
        ".\target\release\draox-server.exe",
        ".\draox-server.exe"
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) {
            $BinaryPath = (Resolve-Path $c).Path
            break
        }
    }
    if (-not $BinaryPath) {
        Write-Host "[ERROR] draox-server.exe not found. Specify -BinaryPath." -ForegroundColor Red
        exit 1
    }
}

if (-not (Test-Path $BinaryPath)) {
    Write-Host "[ERROR] Binary not found: $BinaryPath" -ForegroundColor Red
    exit 1
}

Write-Host "[INFO]  Binary:         $BinaryPath" -ForegroundColor Green
Write-Host "[INFO]  Config:         $ConfigPath" -ForegroundColor Green
Write-Host "[INFO]  Service Name:   $ServiceName" -ForegroundColor Green
Write-Host "[INFO]  Display Name:   $DisplayName" -ForegroundColor Green
Write-Host "[INFO]  Start Type:     $StartType" -ForegroundColor Green
Write-Host "[INFO]  Service Account: $ServiceAccount" -ForegroundColor Green
Write-Host ""

# ── Check if service already exists ──
$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "[WARN]  Service '$ServiceName' already exists (Status: $($existing.Status))." -ForegroundColor Yellow
    $confirm = Read-Host "Reinstall? [Y/n]"
    if ($confirm -match "^[Nn]") {
        Write-Host "[INFO]  Cancelled." -ForegroundColor Cyan
        exit 0
    }
    # Stop and remove existing service
    if ($existing.Status -eq "Running") {
        Write-Host "[STEP]  Stopping existing service..." -ForegroundColor Cyan
        Stop-Service -Name $ServiceName -Force
        Start-Sleep -Seconds 2
    }
    Write-Host "[STEP]  Removing existing service..." -ForegroundColor Cyan
    sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 1
}

# ── Create directories ──
Write-Host "[STEP]  Creating data directories..." -ForegroundColor Cyan
$dirs = @(
    "C:\ProgramData\DraoxServer\data",
    "C:\ProgramData\DraoxServer\logs",
    "C:\ProgramData\DraoxServer\plugins",
    "C:\ProgramData\DraoxServer\certs",
    "C:\ProgramData\DraoxServer\config"
)
foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Write-Host "         Created: $dir" -ForegroundColor DarkGray
    }
}

# ── Copy config if not present ──
if (-not (Test-Path $ConfigPath)) {
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $defaultConfig = Join-Path (Split-Path -Parent $scriptDir) "config\default.toml"
    if (Test-Path $defaultConfig) {
        Copy-Item $defaultConfig $ConfigPath
        Write-Host "[INFO]  Copied default config to: $ConfigPath" -ForegroundColor Green
    } else {
        Write-Host "[WARN]  No default config found. Create $ConfigPath manually." -ForegroundColor Yellow
    }
}

# ── Create the service ──
Write-Host "[STEP]  Creating Windows Service..." -ForegroundColor Cyan

$binPathArg = "`"$BinaryPath`" --config `"$ConfigPath`""

New-Service `
    -Name $ServiceName `
    -BinaryPathName $binPathArg `
    -DisplayName $DisplayName `
    -Description "Draox Server - Plugin-powered multi-protocol socket server (TCP, UDP, WebSocket, HTTP)" `
    -StartupType $StartType | Out-Null

Write-Host "[INFO]  Service created: $ServiceName" -ForegroundColor Green

# ── Configure failure recovery ──
Write-Host "[STEP]  Configuring failure recovery..." -ForegroundColor Cyan
# Reset failure count after 86400 seconds (1 day)
# 1st failure: restart after 5 seconds
# 2nd failure: restart after 30 seconds
# 3rd+ failure: restart after 60 seconds
sc.exe failure $ServiceName reset= 86400 actions= restart/5000/restart/30000/restart/60000 | Out-Null
Write-Host "[INFO]  Recovery: restart on failure (5s / 30s / 60s)" -ForegroundColor Green

# ── Set environment variables in registry ──
Write-Host "[STEP]  Setting environment variables..." -ForegroundColor Cyan
$regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$ServiceName"
$envVars = @(
    "RUST_LOG=info,draox_server=info",
    "RUST_BACKTRACE=0"
)
Set-ItemProperty -Path $regPath -Name "Environment" -Value $envVars -Type MultiString
Write-Host "[INFO]  Environment variables configured" -ForegroundColor Green
Write-Host "[WARN]  Set DRAOX_ADMIN_JWT_SECRET in the service environment!" -ForegroundColor Yellow

# ── Set service account ──
if ($ServiceAccount -ne "LocalSystem") {
    Write-Host "[STEP]  Setting service account: $ServiceAccount" -ForegroundColor Cyan
    sc.exe config $ServiceName obj= "$ServiceAccount" | Out-Null
}

# ── Done ──
Write-Host ""
Write-Host "=========================================" -ForegroundColor Green
Write-Host "  Service Installation Complete!"         -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
Write-Host "  Service:    $ServiceName" -ForegroundColor White
Write-Host "  Binary:     $BinaryPath" -ForegroundColor White
Write-Host "  Config:     $ConfigPath" -ForegroundColor White
Write-Host "  Data:       C:\ProgramData\DraoxServer\data\" -ForegroundColor White
Write-Host "  Logs:       C:\ProgramData\DraoxServer\logs\" -ForegroundColor White
Write-Host "  Plugins:    C:\ProgramData\DraoxServer\plugins\" -ForegroundColor White
Write-Host ""
Write-Host "  IMPORTANT:" -ForegroundColor Yellow
Write-Host "    1. Set DRAOX_ADMIN_JWT_SECRET in the service environment" -ForegroundColor Yellow
Write-Host "    2. Review config: $ConfigPath" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Commands:" -ForegroundColor White
Write-Host "    Start-Service $ServiceName           # Start server" -ForegroundColor DarkGray
Write-Host "    Stop-Service $ServiceName            # Stop server" -ForegroundColor DarkGray
Write-Host "    Restart-Service $ServiceName         # Restart server" -ForegroundColor DarkGray
Write-Host "    Get-Service $ServiceName             # Check status" -ForegroundColor DarkGray
Write-Host "    Get-EventLog -LogName Application -Source $ServiceName  # View logs" -ForegroundColor DarkGray
Write-Host ""
