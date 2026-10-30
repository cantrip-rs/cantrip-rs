# In-person play (docs/in-person.md): builds the shipping web client when it is out of date - play-release-build.ps1
# -BuildOnly, the same publish - builds the in-person server, starts it in a window of its own and opens its setup page.
#
#   in-person.ps1                       build if anything changed, then set up a table
#   in-person.ps1 -NoBuild              on what was built last
#   in-person.ps1 -Fast                 a Release build without ahead-of-time compiling: quick to build, slow on a phone
#   in-person.ps1 -Event ID             host an event already made again, with fresh codes for every device
#   in-person.ps1 -Address IP           the address the devices reach this computer at
#   in-person.ps1 -Port 5200 -TablePort 7790
#                                       the site (client, relay, setup page) and the table; card art is on TablePort + 1
#   in-person.ps1 -Data PATH            where the event is kept (else CANTRIP_DATA, else .cantrip-data in the checkout)
#
# The setup page is served to this computer alone: its QR codes seat whoever scans them.

[CmdletBinding()]
param(
    [switch] $NoBuild,
    [switch] $Fast,
    [string] $Event,
    [string] $Address,
    [int] $Port = 5200,
    [int] $TablePort = 7790,
    [string] $Data
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$project = Join-Path $root 'dotnet\src\Cantrip.Server.InPerson\Cantrip.Server.InPerson.csproj'
$client = Join-Path $root "dotnet\src\Cantrip.Client.Web\bin\Release\net10.0-browserwasm\$(if ($Fast) { 'publish-fast' } else { 'publish' })\wwwroot"

if (-not $Data) { $Data = if ($env:CANTRIP_DATA) { $env:CANTRIP_DATA } else { Join-Path $root '.cantrip-data' } }
$Data = [IO.Path]::GetFullPath($Data)

function Stop-Listener([int] $port) {
    Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }
}

function Wait-Listening([int] $port, [string] $what) {
    for ($i = 0; $i -lt 240; $i++) {
        try {
            $probe = New-Object Net.Sockets.TcpClient
            $probe.Connect('127.0.0.1', $port)
            $probe.Close()
            return
        }
        catch {
            Start-Sleep -Milliseconds 250
        }
    }

    Write-Host "The $what never started listening on port $port. Check its window."
    exit 1
}

# Whatever a last run left serving holds the ports, and files the build would replace.
Stop-Listener $Port
Stop-Listener $TablePort
Stop-Listener ($TablePort + 1)

if (-not $NoBuild) {
    $publish = @{ BuildOnly = $true }
    if ($Fast) { $publish.Fast = $true }
    & (Join-Path $PSScriptRoot 'play-release-build.ps1') @publish
    if ($LASTEXITCODE -ne 0) { exit 1 }

    dotnet build $project -v q --nologo
    if ($LASTEXITCODE -ne 0) { exit 1 }
}

if (-not (Test-Path (Join-Path $client 'index.html'))) {
    Write-Host "Nothing published at $client yet. Run without -NoBuild."
    exit 1
}

$serverArgs = "run --project `"$project`" --no-build -- --data `"$Data`" --client `"$client`" --port $Port --table-port $TablePort"
if ($Address) { $serverArgs += " --address $Address" }
if ($Event) { $serverArgs += " --event $Event" }

Write-Host "Events are kept in $Data"
Write-Host 'The phones and tablets reach this computer over the network: allow the server through the firewall when Windows asks.'
Start-Process -FilePath 'cmd.exe' -ArgumentList "/k title Cantrip in person & dotnet $serverArgs" -WorkingDirectory $root
Wait-Listening $Port 'in-person server'

$page = "http://localhost:$Port/setup"
Start-Process $page
Write-Host "The setup page is open in your browser: $page"
