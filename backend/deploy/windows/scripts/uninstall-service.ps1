#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Draox Server — Windows Service Uninstaller

.DESCRIPTION
    Stops and removes the Draox Server Windows Service.
    Optionally removes all data (ProgramData) with -Purge.

.PARAMETER ServiceName
    Windows service name to remove. Default: DraoxServer

.PARAMETER Purge
    Remove all data directories (C:\ProgramData\DraoxServer).

.PARAMETER KeepConfig
    When used with -Purge, keep configuration files.

.PARAMETER KeepData
    When used with -Purge, keep database and state files.

.PARAMETER KeepLogs
    When used with -Purge, keep log files.

.EXAMPLE
    .\uninstall-service.ps1                    # Remove service only
    .\uninstall-service.ps1 -Purge             # Remove service + all data
    .\uninstall-service.ps1 -Purge -KeepConfig # Remove service + data, keep config
#>

[CmdletBinding()]
param(
    [string]$ServiceName = "DraoxServer",
    [switch]$Purge,
    [switch]$KeepConfig,
    [switch]$KeepData,
    [switch]$KeepLogs
)

$ErrorActionPreference = "Stop"

# ── Banner ──
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  Draox Server - Service Uninstaller"     -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# ── Check if service exists ──
$service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $service) {
    Write-Host "[INFO]  Service '$ServiceName' not found. Nothing to do." -ForegroundColor Yellow
    if (-not $Purge) {
        exit 0
    }
    Write-Host "[INFO]  Continuing with data cleanup (-Purge)..." -ForegroundColor Yellow
} else {
    Write-Host "[INFO]  Found service: $ServiceName (Status: $($service.Status))" -ForegroundColor Green

    # ── Stop service if running ──
    if ($service.Status -eq "Running") {
        Write-Host "[STEP]  Stopping service..." -ForegroundColor Cyan
        Stop-Service -Name $ServiceName -Force
        $timeout = 30
        $elapsed = 0
        while ((Get-Service -Name $ServiceName).Status -ne "Stopped" -and $elapsed -lt $timeout) {
            Start-Sleep -Seconds 1
            $elapsed++
        }
        if ((Get-Service -Name $ServiceName).Status -ne "Stopped") {
            Write-Host "[WARN]  Service did not stop within ${timeout}s. Force killing..." -ForegroundColor Yellow
            $proc = Get-WmiObject Win32_Service | Where-Object { $_.Name -eq $ServiceName }
            if ($proc -and $proc.ProcessId -gt 0) {
                Stop-Process -Id $proc.ProcessId -Force -ErrorAction SilentlyContinue
            }
        }
        Write-Host "[INFO]  Service stopped." -ForegroundColor Green
    }

    # ── Remove service ──
    Write-Host "[STEP]  Removing service..." -ForegroundColor Cyan
    sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 1
    Write-Host "[INFO]  Service '$ServiceName' removed." -ForegroundColor Green
}

# ── Remove firewall rules ──
Write-Host "[STEP]  Removing firewall rules..." -ForegroundColor Cyan
$rules = @(
    "Draox Server - TCP (9000)",
    "Draox Server - UDP (9001)",
    "Draox Server - WebSocket (9002)",
    "Draox Server - HTTP (9003)",
    "Draox Server - Metrics (9090)",
    "Draox Server - Admin API (9100)"
)
foreach ($rule in $rules) {
    $existing = Get-NetFirewallRule -DisplayName $rule -ErrorAction SilentlyContinue
    if ($existing) {
        Remove-NetFirewallRule -DisplayName $rule
        Write-Host "         Removed: $rule" -ForegroundColor DarkGray
    }
}
Write-Host "[INFO]  Firewall rules cleaned up." -ForegroundColor Green

# ── Purge data ──
if ($Purge) {
    Write-Host ""
    Write-Host "[WARN]  Purging data directories..." -ForegroundColor Yellow

    $baseDir = "C:\ProgramData\DraoxServer"

    if (-not $KeepData) {
        $dataDir = Join-Path $baseDir "data"
        if (Test-Path $dataDir) {
            Remove-Item -Path $dataDir -Recurse -Force
            Write-Host "         Removed: $dataDir" -ForegroundColor DarkGray
        }
    } else {
        Write-Host "         Keeping: data\" -ForegroundColor DarkGray
    }

    if (-not $KeepLogs) {
        $logDir = Join-Path $baseDir "logs"
        if (Test-Path $logDir) {
            Remove-Item -Path $logDir -Recurse -Force
            Write-Host "         Removed: $logDir" -ForegroundColor DarkGray
        }
    } else {
        Write-Host "         Keeping: logs\" -ForegroundColor DarkGray
    }

    if (-not $KeepConfig) {
        $configDir = Join-Path $baseDir "config"
        $certsDir = Join-Path $baseDir "certs"
        if (Test-Path $configDir) {
            Remove-Item -Path $configDir -Recurse -Force
            Write-Host "         Removed: $configDir" -ForegroundColor DarkGray
        }
        if (Test-Path $certsDir) {
            Remove-Item -Path $certsDir -Recurse -Force
            Write-Host "         Removed: $certsDir" -ForegroundColor DarkGray
        }
    } else {
        Write-Host "         Keeping: config\ and certs\" -ForegroundColor DarkGray
    }

    # Always remove plugins on purge
    $pluginDir = Join-Path $baseDir "plugins"
    if (Test-Path $pluginDir) {
        Remove-Item -Path $pluginDir -Recurse -Force
        Write-Host "         Removed: $pluginDir" -ForegroundColor DarkGray
    }

    # Remove base directory if empty
    if ((Test-Path $baseDir) -and -not (Get-ChildItem $baseDir -Recurse -File)) {
        Remove-Item -Path $baseDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "         Removed: $baseDir" -ForegroundColor DarkGray
    }

    Write-Host "[INFO]  Data purge complete." -ForegroundColor Green
}

# ── Done ──
Write-Host ""
Write-Host "=========================================" -ForegroundColor Green
Write-Host "  Uninstallation Complete!"               -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
if (-not $Purge) {
    Write-Host "  Data directories preserved at: C:\ProgramData\DraoxServer\" -ForegroundColor White
    Write-Host "  To remove all data: .\uninstall-service.ps1 -Purge" -ForegroundColor DarkGray
}
Write-Host ""
