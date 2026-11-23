@echo off
rem ---------------------------------------------------------------------------
rem Builds the rules engine for the browser (crates/cantrip-wasm, the wasm profile, docs/engine-in-the-browser.md) and
rem puts it where the web client serves it: dotnet\src\Cantrip.Client.Web\Platforms\WebAssembly\wwwroot\engine\, which
rem is not tracked. Run it before publishing or serving a client that plays ?local.
rem
rem   build-engine-wasm.cmd
rem ---------------------------------------------------------------------------
setlocal
cd /d "%~dp0.."

rustup target list --installed | findstr /x "wasm32-unknown-unknown" >nul || rustup target add wasm32-unknown-unknown || exit /b 1
cargo build --profile wasm --target wasm32-unknown-unknown -p cantrip-wasm || exit /b 1

set OUT=dotnet\src\Cantrip.Client.Web\Platforms\WebAssembly\wwwroot\engine
if not exist "%OUT%" mkdir "%OUT%"
copy /y target\wasm32-unknown-unknown\wasm\cantrip_wasm.wasm "%OUT%\cantrip_wasm.wasm" >nul || exit /b 1
echo The engine is in %OUT%\cantrip_wasm.wasm
