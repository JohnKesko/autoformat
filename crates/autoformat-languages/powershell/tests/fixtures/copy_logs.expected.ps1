<#
.SYNOPSIS
    Copies log files.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Source,
    [int]$Days = 7
)
function Copy-Logs {
    param($From, $To)
    $cutoff = (Get-Date).AddDays(-$Days)
    Get-ChildItem -Path $From -Filter *.log |
        Where-Object { $_.LastWriteTime -gt $cutoff } |
        ForEach-Object {
            Copy-Item $_.FullName -Destination $To -Force  # copy it
        }
    if ($From -eq $To) { Write-Warning "same folder" }
    elseif ($null -eq $To) { throw "no target" }
    else {
        $settings = @{ Name = 'logs'; Count = 3 }
        $list = @(1, 2, 3)
        $text = @"
Copied from $From
  to $To
"@
        Write-Host $text
    }
    switch ($Days) {
        7 { 'week' }
        default { 'other' }
    }
    foreach ($i in $list) { $total += $i }
    Get-Item -Path $From `
        -ErrorAction Stop
    [Math]::Round(3.14159, 2)
}
