#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Draox Server — Windows Firewall Rule Manager

.DESCRIPTION
    Adds or removes Windows Firewall rules for Draox Server ports.
    By default, the Admin API (9100) is restricted to localhost only.

.PARAMETER Action
    Add or Remove firewall rules. Required.

.PARAMETER AdminRemoteAccess
    Allow remote access to Admin API (port 9100). Default: localhost only.

.EXAMPLE
    .\manage-firewall.ps1 -Action Add                     # Open ports (admin=localhost)
    .\manage-firewall.ps1 -Action Add -AdminRemoteAccess  # Open ports (admin=remote)
    .\manage-firewall.ps1 -Action Remove                  # Remove all rules
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Add", "Remove")]
    [string]$Action,

    [switch]$AdminRemoteAccess
)

$ErrorActionPreference = "Stop"

# ── Rule definitions ──
$rules = @(
    @{
        DisplayName = "Draox Server - TCP (9000)"
        Direction   = "Inbound"
        Protocol    = "TCP"
        LocalPort   = 9000
        Description = "Draox Server TCP protocol"
    },
    @{
        DisplayName = "Draox Server - UDP (9001)"
        Direction   = "Inbound"
        Protocol    = "UDP"
        LocalPort   = 9001
        Description = "Draox Server UDP protocol"
    },
    @{
        DisplayName = "Draox Server - WebSocket (9002)"
        Direction   = "Inbound"
        Protocol    = "TCP"
        LocalPort   = 9002
        Description = "Draox Server WebSocket protocol"
    },
    @{
        DisplayName = "Draox Server - HTTP (9003)"
        Direction   = "Inbound"
        Protocol    = "TCP"
        LocalPort   = 9003
        Description = "Draox Server HTTP/HTTPS protocol"
    },
    @{
        DisplayName = "Draox Server - Metrics (9090)"
        Direction   = "Inbound"
        Protocol    = "TCP"
        LocalPort   = 9090
        Description = "Draox Server Prometheus metrics endpoint"
    }
)

# Admin API rule (conditional scope)
$adminRule = @{
    DisplayName  = "Draox Server - Admin API (9100)"
    Direction    = "Inbound"
    Protocol     = "TCP"
    LocalPort    = 9100
    Description  = "Draox Server Admin REST API"
}

Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  Draox Server - Firewall Manager"        -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

if ($Action -eq "Add") {
    Write-Host "[STEP]  Adding firewall rules..." -ForegroundColor Cyan

    foreach ($rule in $rules) {
        $existing = Get-NetFirewallRule -DisplayName $rule.DisplayName -ErrorAction SilentlyContinue
        if ($existing) {
            Write-Host "         Exists: $($rule.DisplayName)" -ForegroundColor DarkGray
            continue
        }
        New-NetFirewallRule `
            -DisplayName $rule.DisplayName `
            -Direction $rule.Direction `
            -Protocol $rule.Protocol `
            -LocalPort $rule.LocalPort `
            -Action Allow `
            -Description $rule.Description `
            -Profile Any | Out-Null
        Write-Host "         Added:  $($rule.DisplayName)" -ForegroundColor Green
    }

    # Admin API — localhost or remote
    $existingAdmin = Get-NetFirewallRule -DisplayName $adminRule.DisplayName -ErrorAction SilentlyContinue
    if ($existingAdmin) {
        Remove-NetFirewallRule -DisplayName $adminRule.DisplayName
    }

    $adminParams = @{
        DisplayName = $adminRule.DisplayName
        Direction   = $adminRule.Direction
        Protocol    = $adminRule.Protocol
        LocalPort   = $adminRule.LocalPort
        Action      = "Allow"
        Description = $adminRule.Description
        Profile     = "Any"
    }

    if (-not $AdminRemoteAccess) {
        $adminParams["RemoteAddress"] = "127.0.0.1", "::1"
        $scope = "localhost only"
    } else {
        $scope = "all interfaces (REMOTE ACCESS)"
    }

    New-NetFirewallRule @adminParams | Out-Null
    Write-Host "         Added:  $($adminRule.DisplayName) ($scope)" -ForegroundColor Green

    Write-Host ""
    Write-Host "[INFO]  Firewall rules configured:" -ForegroundColor Green
    Write-Host "         TCP  9000  - Socket protocol" -ForegroundColor White
    Write-Host "         UDP  9001  - Datagram protocol" -ForegroundColor White
    Write-Host "         TCP  9002  - WebSocket" -ForegroundColor White
    Write-Host "         TCP  9003  - HTTP/HTTPS" -ForegroundColor White
    Write-Host "         TCP  9090  - Prometheus metrics" -ForegroundColor White
    Write-Host "         TCP  9100  - Admin API ($scope)" -ForegroundColor White

} elseif ($Action -eq "Remove") {
    Write-Host "[STEP]  Removing firewall rules..." -ForegroundColor Cyan

    $allRules = $rules + @($adminRule)
    foreach ($rule in $allRules) {
        $existing = Get-NetFirewallRule -DisplayName $rule.DisplayName -ErrorAction SilentlyContinue
        if ($existing) {
            Remove-NetFirewallRule -DisplayName $rule.DisplayName
            Write-Host "         Removed: $($rule.DisplayName)" -ForegroundColor Green
        } else {
            Write-Host "         Not found: $($rule.DisplayName)" -ForegroundColor DarkGray
        }
    }

    Write-Host ""
    Write-Host "[INFO]  All Draox firewall rules removed." -ForegroundColor Green
}

Write-Host ""
