# The advisor's table: you in the AI's seat against a bot, the AI's advice beside the table at every decision, every
# decision recorded under ..\ai-data\sessions\<date>\<game>\ beside the repository.
#
# Starts everything: the inference server (python -m cantrip_ai.serve on its own port, on the GPU), the advisor
# (dotnet\tools\Cantrip.Advisor: its own table, its own AI host, the page), and the page in your browser.
#
#   advisor.ps1                                         asks for both decks and the opponent
#   advisor.ps1 -Mine dimir-midrange -Theirs boros-dragons -Opponent neural
#
#   -Mine, -Theirs   decklists by name under -Decks, or paths
#   -Opponent        neural (the network's search, as the AI plays) or tactical
#   -Model DIR       the AI's network (default: the champion in force, ..\ai-data\champions\current)
#   -Decks DIR       the decklists (default ..\ai-data\decks)
#   -Python EXE      the Python that runs the inference server (default: $env:CANTRIP_PYTHON, else python)
#   -Budget MS       the AI's search time per decision, for the advice and a neural opponent (default 3000)
#   -Worlds N        worlds a Decklist search deals (default 8)
#   -Seed N          the shuffle (default: a new one, printed)
#   -Port N          the page's port (default 5390); -ServerPort N the inference server's (default 7841)
#   -Cpu             no inference server: the AI evaluates on the CPU (slower)
#   -NoBuild         skip the builds
#   -NoBrowser       print the page's address rather than opening it

[CmdletBinding()]
param(
    [string] $Mine,
    [string] $Theirs,
    [ValidateSet('', 'neural', 'tactical')][string] $Opponent = '',
    [string] $Model = '',
    [int] $Budget = 3000,
    [int] $Worlds = 8,
    [uint64] $Seed = 0,
    [int] $Port = 5390,
    [int] $ServerPort = 7841,
    [string] $Decks = '',
    [string] $Python = '',
    [switch] $Cpu,
    [switch] $NoBrowser,
    [switch] $NoBuild
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$data = Join-Path (Split-Path -Parent $root) 'ai-data'
if (-not $Model) { $Model = Join-Path $data 'champions\current' }
if (-not $Decks) { $Decks = Join-Path $data 'decks' }
if (-not $Python) { $Python = if ($env:CANTRIP_PYTHON) { $env:CANTRIP_PYTHON } else { 'python' } }

$available = @(Get-ChildItem -Path $Decks -Filter *.txt | ForEach-Object { $_.BaseName } | Sort-Object)
function Ask-Deck([string] $who, [string] $default) {
    for ($i = 0; $i -lt $available.Count; $i++) { Write-Host ('  {0}. {1}' -f ($i + 1), $available[$i]) }
    while ($true) {
        $answer = "$(Read-Host "Deck for $who, by number or name [$default]")".Trim()
        if ($answer -eq '') { return $default }
        $n = 0
        if ([int]::TryParse($answer, [ref] $n) -and $n -ge 1 -and $n -le $available.Count) { return $available[$n - 1] }
        if ($available -contains $answer -or (Test-Path -LiteralPath $answer)) { return $answer }
        Write-Host "No deck '$answer'."
    }
}

if (-not $Mine) { $Mine = Ask-Deck 'you (the AI''s seat)' 'dimir-midrange' }
if (-not $Theirs) { $Theirs = Ask-Deck 'the bot' 'boros-dragons' }
if (-not $Opponent) {
    $answer = "$(Read-Host 'The bot: (n)eural or (t)actical [n]')".Trim().ToLowerInvariant()
    $Opponent = if ($answer -like 't*') { 'tactical' } else { 'neural' }
}

if (-not $NoBuild) {
    Write-Host 'Building...'
    Push-Location $root
    try {
        cargo build --profile cards -j 8 -p cantrip-host --bin cantrip-native-host -p cantrip-ai-host --bin cantrip-advisor-host
        if ($LASTEXITCODE -ne 0) { exit 1 }
    }
    finally { Pop-Location }
    dotnet build (Join-Path $root 'dotnet\tools\Cantrip.Advisor\Cantrip.Advisor.csproj') -v q --nologo
    if ($LASTEXITCODE -ne 0) { exit 1 }
    dotnet build (Join-Path $root 'dotnet\src\Cantrip.Client.Web\Cantrip.Client.Web.csproj') -f net10.0-browserwasm -v q --nologo
    if ($LASTEXITCODE -ne 0) { exit 1 }
}

$server = $null
$serverArgs = @()
try {
    if (-not $Cpu) {
        $ready = Join-Path $env:TEMP "cantrip-advisor-serve-$ServerPort.ready"
        Remove-Item -LiteralPath $ready -ErrorAction SilentlyContinue
        $log = Join-Path $env:TEMP "cantrip-advisor-serve-$ServerPort.log"
        Write-Host "Starting the inference server on port $ServerPort (log: $log)..."
        $server = Start-Process -FilePath $Python -WorkingDirectory (Join-Path $root 'ai') -WindowStyle Hidden -PassThru `
            -RedirectStandardError $log `
            -ArgumentList @('-m', 'cantrip_ai.serve', '--model', "`"$Model`"", '--port', $ServerPort, '--ready-file', "`"$ready`"",
                '--device', 'auto', '--batch', '32:32,64:32,128:32', '--priority', 'normal')
        for ($i = 0; $i -lt 240 -and -not (Test-Path -LiteralPath $ready); $i++) {
            if ($server.HasExited) { Write-Host "The inference server stopped; see $log."; exit 1 }
            Start-Sleep -Milliseconds 500
        }
        if (-not (Test-Path -LiteralPath $ready)) { Write-Host "The inference server never got ready; see $log."; exit 1 }
        $serverArgs = @('--server', "127.0.0.1:$ServerPort")
    }

    $arguments = @('--mine', $Mine, '--theirs', $Theirs, '--decks', $Decks, '--model', $Model, '--opponent', $Opponent,
        '--budget', $Budget, '--worlds', $Worlds, '--port', $Port) + $serverArgs
    if ($Seed -gt 0) { $arguments += @('--seed', $Seed) }
    if ($NoBrowser) { $arguments += @('--no-browser') }
    $exe = Join-Path $root 'dotnet\tools\Cantrip.Advisor\bin\Debug\net8.0\cantrip-advisor.exe'
    & $exe @arguments
    exit $LASTEXITCODE
}
finally {
    if ($server -and -not $server.HasExited) { Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue }
}
