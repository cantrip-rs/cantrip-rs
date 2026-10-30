@echo off
rem ---------------------------------------------------------------------------
rem Plays one way of playing a seat against another, many times, and says which
rem won more: the measure of whether a bot is any good.
rem
rem   bot-duel.cmd                                 the tactical bot against the random one, 400 games of RW aggro
rem   bot-duel.cmd --b auto                        against the one deterministic line instead
rem   bot-duel.cmd --a tactical --b prior          the fitted weights against the hand-written ones
rem   bot-duel.cmd --deck rw-aggro --deck ub-reanimator --games 2000
rem   bot-duel.cmd watch                           one game, told turn by turn
rem   bot-duel.cmd watch --b auto --lost           the first game the tactical bot loses
rem
rem A seat is random, auto, tactical, prior, or a path to a model file. Games
rem are at the session layer - no client and no socket, every core at once -
rem and the seats swap every other game, so nobody is counted as better for
rem having gone first. The tally comes with its margin of error.
rem
rem Every argument is passed through; run with --help for the rest.
rem ---------------------------------------------------------------------------

setlocal

set "ROOT=%~dp0.."
set "COMMAND=duel"
if /i "%~1"=="watch" set "COMMAND="

rem Recording (--ai-record) needs the native host built with its recorder, which the shipping build leaves out.
echo %* | findstr /c:"--ai-record" > nul && (cargo build --manifest-path "%ROOT%\Cargo.toml" --profile cards -p cantrip-host --bin cantrip-native-host --features ai-record || exit /b 2)
rem A neural seat needs the AI host, with ONNX Runtime (a plain cargo test rebuilds it tract-only); --ai-server
rem HOST:PORT has it evaluate on the inference server (python -m cantrip_ai.serve, docs/ai-training.md) instead.
echo %* | findstr /c:"neural" > nul && (cargo build --manifest-path "%ROOT%\Cargo.toml" --profile cards -p cantrip-ai-host --features ort || exit /b 2)
dotnet build "%ROOT%\dotnet\tools\Cantrip.BotLab\Cantrip.BotLab.csproj" -v q --nologo || exit /b 2
dotnet run --project "%ROOT%\dotnet\tools\Cantrip.BotLab\Cantrip.BotLab.csproj" --no-build -v q -- %COMMAND% %*

endlocal & exit /b %ERRORLEVEL%
