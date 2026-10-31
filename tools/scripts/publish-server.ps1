# Publishes Cantrip to the server: builds here what can be built here (the Discord bot for linux-arm64, and the web client,
# compiled ahead of time), sends the game's source and data and builds the Rust engine host there, publishes the art
# (publish-art.ps1), and swaps the new bot and client in with one restart of the bot. If the bot does not come back, the
# old one is put back.
#
#   publish-server.ps1                   everything
#   publish-server.ps1 -Fast             a web client without ahead-of-time compiling: quick to build, slow to run
#   publish-server.ps1 -SkipClient       reuse the web client built last time (.cantrip-run\publish\client)
#   publish-server.ps1 -SkipArt          leave the art alone
#   publish-server.ps1 -Server user@host -Key path    another server (else tools\deploy.local.json; see deploy-common.ps1)
#
# What is sent is this working tree - committed or not - so publish what you have tested. The server's copy of the tree is
# a deployment, not a clone: files that are gone here are gone there (only crates, decks, the cube lists, the leagues and content). The
# engine host (crates\cantrip-host) is built on the server, where it will run, because it is native code for the server's
# processor; cargo there builds only what changed. The bot is down only for the restart.
#
# Left alone on the server: /etc/cantrip/bot.env (the Discord token and the OAuth secret), the systemd units (copies are in
# tools\deploy), the Cloudflare tunnel, and the data directory (drafts, sign-in keys).

[CmdletBinding()]
param(
    [string] $Server = '',
    [string] $Key = '',
    [switch] $Fast,
    [switch] $SkipClient,
    [switch] $SkipArt
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'deploy-common.ps1')

$config = Get-DeployConfig $Server $Key
$root = $script:Root
$stage = Join-Path $root '.cantrip-run\publish'
$botOut = Join-Path $stage 'bot'
$clientOut = Join-Path $stage 'client'
$started = Get-Date

Write-Step 'Checking the server'
Invoke-Remote $config 'echo connected to $(hostname); test -x ~/.cargo/bin/cargo && test -d ~/cantrip/src && echo "bot: $(systemctl is-active cantrip-discord || true)"' | ForEach-Object { Write-Host $_ }

# --- Built here ------------------------------------------------------------------------------------------------------

Write-Step 'Publishing the Discord bot (linux-arm64)'
if (Test-Path $botOut) { Remove-Item -Recurse -Force $botOut }
dotnet publish (Join-Path $root 'dotnet\src\Cantrip.Discord\Cantrip.Discord.csproj') -c Release -r linux-arm64 --self-contained false -o $botOut -v q --nologo
if ($LASTEXITCODE -ne 0) { throw 'The bot failed to build.' }
Get-ChildItem $botOut -Filter *.xml | Remove-Item

if ($SkipClient) {
    Write-Step 'Web client: the one built last time'
    if (-not (Test-Path (Join-Path $clientOut 'wwwroot\index.html'))) { throw "No web client at $clientOut yet. Run without -SkipClient." }
}
else {
    Write-Step "Publishing the web client ($(if ($Fast) { 'interpreted' } else { 'ahead-of-time compiled' }))"
    $mode = if ($Fast) { @('-p:RunAOTCompilation=false', '-p:WasmShellMonoRuntimeExecutionMode=Interpreter') } else { @() }

    # Published afresh each time: a publish adds to what is there, and Uno picks its loader by looking for dotnet.*.js in the
    # folder, so an old one left behind can win and load the libraries it was published with. The ahead-of-time work is kept
    # in obj, not here, so starting this folder over costs nothing.
    if (Test-Path $clientOut) { Remove-Item -Recurse -Force $clientOut }
    dotnet publish (Join-Path $root 'dotnet\src\Cantrip.Client.Web\Cantrip.Client.Web.csproj') -c Release -f net10.0-browserwasm -o $clientOut -v q --nologo '-p:PublishCompressionFormats=gzip' @mode
    if ($LASTEXITCODE -ne 0) { throw 'The web client failed to build.' }
}
if (-not (Test-Path (Join-Path $clientOut 'wwwroot\index.html'))) { throw "Nothing published at $clientOut\wwwroot." }

# --- Art, which needs no downtime ------------------------------------------------------------------------------------

if (-not $SkipArt) {
    Write-Step 'Publishing the art'
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'publish-art.ps1') -Server $config.Server -Key $config.Key
    if ($LASTEXITCODE -ne 0) { throw 'Publishing the art failed.' }
}

# --- The game's source and data, and the engine host built where it runs -----------------------------------------------

Write-Step 'Sending the source and data'
Push-Location $root
try {
    # Tracked and not-yet-tracked files, but not the ignored (the art, build output): what a checkout of this tree would hold.
    $manifest = @(git -c core.quotepath=false ls-files --cached --others --exclude-standard -- crates Cargo.toml Cargo.lock content decks docs/cube-lists leagues |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Sort-Object -Unique)
    $revision = (git rev-parse --short HEAD).Trim()
    $dirty = @(git status --porcelain).Count
}
finally {
    Pop-Location
}

$sendDir = Join-Path $env:TEMP "cantrip-publish-$PID"
New-Item -ItemType Directory -Path $sendDir | Out-Null
try {
    $list = Join-Path $sendDir 'manifest.txt'
    [IO.File]::WriteAllText($list, (($manifest -join "`n") + "`n"))
    Write-Host "$($manifest.Count) files."
    Copy-ToServer $config $root '~/cantrip/src' $list

    # The list goes along (alone, from a folder of its own), to take away what is gone from here.
    $own = Join-Path $sendDir 'own'
    New-Item -ItemType Directory -Path $own | Out-Null
    Move-Item $list (Join-Path $own 'manifest.txt')
    Copy-ToServer $config $own '~/cantrip'
}
finally {
    Remove-Item -Recurse -Force $sendDir -ErrorAction SilentlyContinue
}

Write-Step 'Taking away what is gone, and building the engine host there'

# Files in the folders sent that the list does not name. The engine's build output and the art folders (the server has
# none) are not those folders' to prune; nothing outside crates, decks, the cube lists, the leagues and content is.
$build = @'
set -e
export LC_ALL=C
cd ~/cantrip/src
find crates decks docs/cube-lists leagues content -type f ! -path 'content/*/images/*' ! -path '*/target/*' 2>/dev/null | sort > ~/cantrip/have.txt
sort ~/cantrip/manifest.txt > ~/cantrip/want.txt
comm -23 ~/cantrip/have.txt ~/cantrip/want.txt > ~/cantrip/stale.txt
echo "$(wc -l < ~/cantrip/stale.txt) files to take away"
xargs -r -d '\n' rm -f < ~/cantrip/stale.txt
. ~/.cargo/env
cargo build --release -p cantrip-host 2>&1 | tail -n 4
test "${PIPESTATUS[0]}" -eq 0
'@
if ((Invoke-RemoteScript $config $build) -ne 0) { throw 'The engine host failed to build on the server; the bot was left as it was.' }

# --- Sent, then swapped in ---------------------------------------------------------------------------------------------

Write-Step 'Sending the bot and the web client'
Invoke-Remote $config 'rm -rf ~/cantrip/stage && mkdir -p ~/cantrip/stage/bot ~/cantrip/stage/client' | Out-Null
Copy-ToServer $config $botOut '~/cantrip/stage/bot'
Copy-ToServer $config (Join-Path $clientOut 'wwwroot') '~/cantrip/stage/client'

Write-Step 'Swapping them in and restarting the bot'
$swap = @'
set -e
cd ~/cantrip
BOT=src/.cantrip-run/discord
rm -rf $BOT.old client/wwwroot.old
mkdir -p src/.cantrip-run client
if [ -d $BOT ]; then mv $BOT $BOT.old; fi
if [ -d client/wwwroot ]; then mv client/wwwroot client/wwwroot.old; fi
mv stage/bot $BOT
mv stage/client client/wwwroot
sudo systemctl restart cantrip-discord
healthy=no
for i in $(seq 1 60); do
  if [ "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:7779/status)" = 200 ]; then healthy=yes; break; fi
  sleep 1
done
if [ $healthy = yes ]; then
  sleep 3
  if journalctl -u cantrip-discord --since "-30sec" --no-pager | grep -q 'Gateway.*Ready'; then echo "bot is up and connected to Discord"; else echo "WARNING: the bot is serving but has not said it is connected to Discord yet"; fi
  rm -rf $BOT.old client/wwwroot.old
  exit 0
fi
echo "The bot did not come back. Its log:"
journalctl -u cantrip-discord -n 25 --no-pager | cut -c1-200
echo "Putting the old one back."
sudo systemctl stop cantrip-discord || true
rm -rf $BOT client/wwwroot
if [ -d $BOT.old ]; then mv $BOT.old $BOT; fi
if [ -d client/wwwroot.old ]; then mv client/wwwroot.old client/wwwroot; fi
sudo systemctl start cantrip-discord
exit 1
'@
if ((Invoke-RemoteScript $config $swap) -ne 0) { throw 'The bot did not come back on the new build; the old one was put back.' }

$note = "$revision$(if ($dirty -gt 0) { " + $dirty changed files" }) at $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
Invoke-Remote $config "echo '$note' > ~/cantrip/DEPLOYED" | Out-Null

# --- Seen from outside -------------------------------------------------------------------------------------------------

if ($config.Url) {
    Write-Step "Checking $($config.Url)"
    $slug = Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Cantrip\art-cache\512') -Filter *.webp | Select-Object -First 1
    # /inperson is in-person play's setup page (docs/in-person.md, "Hosted in person"): on whenever the bot has a data directory.
    $paths = @('/', '/status', '/inperson', '/inperson/api/choices', '/art/artists.json')
    if ($slug) { $paths += "/art/512/$($slug.Name)" }
    foreach ($path in $paths) {
        try {
            $code = (Invoke-WebRequest -Uri ($config.Url.TrimEnd('/') + $path) -UseBasicParsing -TimeoutSec 30).StatusCode
        }
        catch {
            $code = if ($_.Exception.Response) { [int] $_.Exception.Response.StatusCode } else { $_.Exception.Message }
        }
        Write-Host "  $code  $path"
    }
}

Write-Host ''
Write-Host ("Published {0}{1} in {2:mm\:ss}." -f $revision, $(if ($dirty -gt 0) { " (+$dirty changed files)" } else { '' }), ((Get-Date) - $started)) -ForegroundColor Green
