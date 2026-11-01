#requires -Version 7
<#
.SYNOPSIS  Hide the real DualSense from games (HidHide) so only the virtual Xbox 360 pad is seen.
.DESCRIPTION
  Hides every Sony DualSense/Edge, whitelists the dotnet host that runs dualsense-emu, turns cloaking on.
  Run from an elevated pwsh. -Undo reverses it. -WhatIf previews without changing anything.
  shortcut: whitelists the whole dotnet.exe (any dotnet app can still see the pad); publish a single exe and whitelist that instead.
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [switch]$Undo,
    [string]$Dotnet = (& mise which dotnet 2>$null)
)
$ErrorActionPreference = 'Stop'
$cli = Join-Path $env:ProgramFiles 'Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe'
if (-not (Test-Path $cli)) { throw "HidHide not installed: winget install Nefarius.HidHide" }
if (-not $Dotnet -or -not (Test-Path $Dotnet)) { throw "dotnet not found; pass -Dotnet <path>" }

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole('Administrator')
if (-not $isAdmin -and -not $WhatIfPreference) { throw "Run this from an elevated pwsh (Run as administrator)." }

# Sony VID 054C; PID 0CE6 = DualSense, 0DF2 = DualSense Edge
$paths = (& $cli --dev-gaming | ConvertFrom-Json).devices.deviceInstancePath |
    Where-Object { $_ -match 'VID[_&](0002)?054C' -and $_ -match 'PID[_&](0CE6|0DF2)' } | Sort-Object -Unique
if (-not $paths) { throw "No DualSense found. Connect it first." }

foreach ($p in $paths) {
    $verb = if ($Undo) { '--dev-unhide' } else { '--dev-hide' }
    if ($PSCmdlet.ShouldProcess($p, $verb)) { & $cli $verb $p }
}
if ($PSCmdlet.ShouldProcess($Dotnet, $(if ($Undo) { '--app-unreg' } else { '--app-reg' }))) {
    & $cli ($Undo ? '--app-unreg' : '--app-reg') $Dotnet
}
if ($PSCmdlet.ShouldProcess('HidHide', $(if ($Undo) { '--cloak-off' } else { '--cloak-on' }))) {
    & $cli ($Undo ? '--cloak-off' : '--cloak-on')
}
if (-not $WhatIfPreference) { Write-Host ($Undo ? 'Undone: real DualSense visible again.' : "Hidden: $($paths -join '; ')") }
