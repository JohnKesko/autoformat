#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-DeploymentTarget {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [ValidateSet('dev', 'test', 'prod')]
        [string] $Environment
    )

    $targets = @{
        dev = @{ Server = 'dev01'; Port = 8080 }
        test = @{ Server = 'test01'; Port = 8080 }
        prod = @{ Server = 'prod01'; Port = 443 }
    }
    return $targets[$Environment]
}

function Invoke-Deployment {
    param([string]$Environment, [switch]$WhatIf)

    $target = Get-DeploymentTarget -Environment $Environment
    $packages = Get-ChildItem -Path "$PSScriptRoot\packages" -Filter '*.zip' | Sort-Object LastWriteTime -Descending
    if (-not $packages) {
        Write-Error "No packages found"
        return
    }

    $latest = $packages | Select-Object -First 1
    try {
        Write-Verbose "Deploying $($latest.Name) to $($target.Server):$($target.Port)"
        if (-not $WhatIf) {
            Copy-Item -Path $latest.FullName -Destination "\\$($target.Server)\deploy$" -Force
        }
    }
    catch [System.IO.IOException] {
        Write-Warning "Copy failed: $_"
        throw
    }
    finally {
        Write-Verbose 'Done'
    }
}

Invoke-Deployment -Environment dev -WhatIf
