@echo off
rem ---------------------------------------------------------------------------
rem Builds the solution, the web client included, and runs the tests: the .NET ones, then the native engine's (cargo
rem test --profile checked over the workspace at the root, debug assertions on, which plays every scenario of
rem scenarios\ whose cards it has).
rem
rem   test.cmd                          the fast tests: everything that does not
rem                                     run on the wall clock
rem   test.cmd /all                     every test, the wall-clock ones included
rem   test.cmd /noweb                   every test but the browser tests
rem   test.cmd /nobuild                 skip the build; test what is already built
rem   test.cmd /engine                  the rules engine's tests alone (cargo test
rem                                     over the workspace, the scenario corpus
rem                                     included): what a change to cards,
rem                                     mechanics or scenarios can break. No .NET
rem                                     build or test; run the whole suite when
rem                                     the host, protocol or client change
rem   test.cmd /filter "FullyQualifiedName~Swiss"
rem                                     only matching .NET tests; the expression
rem                                     is passed to dotnet test --filter as is,
rem                                     and the native engine's tests are left
rem                                     out (one scenario or card is run with
rem                                     cargo run --profile cards -p
rem                                     cantrip-script --bin corpus -- --card
rem                                     "Name"; see scenarios\README.md)
rem   test.cmd /detailed                print each test and full failure
rem                                     messages
rem
rem Wall-clock tests wait on real time: the browser tests (Cantrip.Client.Web.Tests),
rem and the tests marked [Trait("Category", "WallClock")] - real sockets, the
rem Discord bot's tables, whole events played by bots. Run them when the work
rem touches what they cover (/filter "Category=WallClock&FullyQualifiedName~Pod",
rem say, or /all), not as a habit.
rem
rem The browser tests are not a nicety. They run the web client as a player gets
rem it - the WebAssembly build, served, in Chrome, over a real WebSocket - and
rem play one whole game in it and draw every prompt dock. Their pictures go under TestResults\web. They
rem take the installed Chrome or Edge; see dotnet\tests\Cantrip.Client.Web.Tests.
rem
rem Exits non-zero if the build or any test fails, and always ends with one
rem verdict line: "All tests passed (1,262 .NET, 8,849 engine, 3 skipped).",
rem "Tests failed (...)." or "Build failed. No tests ran." Read that line, not
rem the tail of the output - and beware piping this through `tail`, which hands
rem back its own exit code.
rem
rem The output is captured and only what a passing run does not print is shown
rem (failures, errors, panics, anything unrecognised: tools\maintenance\
rem test-report.ps1), so a pass is the verdict line alone. /detailed streams
rem everything as it runs, each test listed.
rem ---------------------------------------------------------------------------

rem No delayed expansion: the /noweb filter carries a "!", which it would eat.
rem Each option line is read again after its goto, so %ARGS% builds up anyway.
setlocal

set "ROOT=%~dp0.."
set "BUILD=1"

rem No MSBuild worker nodes left behind. Kept for reuse, they outlive the run as
rem idle ".NET Host" processes, holding build assemblies, and a play test started
rem afterwards could not build until they were killed by hand.
set "MSBUILDDISABLENODEREUSE=1"
set "ARGS="
set "WEB=1"
set "SLOW="

:args
if "%~1"=="" goto parsed
if /i "%~1"=="/nobuild"  ( set "BUILD=" & shift & goto args )
if /i "%~1"=="/engine"   ( set "ENGINE=1" & shift & goto args )
if /i "%~1"=="/noweb"    ( set "WEB=" & set "SLOW=1" & shift & goto args )
if /i "%~1"=="/all"      ( set "SLOW=1" & shift & goto args )
if /i "%~1"=="/filter"   ( set "ARGS=%ARGS% --filter "%~2"" & set "FILTERED=1" & shift & shift & goto args )
if /i "%~1"=="/detailed" ( set "ARGS=%ARGS% --logger "console;verbosity=detailed"" & set "DETAILED=1" & shift & goto args )
if /i "%~1"=="/?"        goto usage
echo Unknown option: %~1
goto usage

:parsed

rem A filter given is the only filter: dotnet test takes one. Without one, the
rem default leaves out the wall-clock tests, and /noweb only the browser's.
if not defined ARGS if not defined SLOW set "ARGS=--filter "Category!=WallClock^&FullyQualifiedName!~Cantrip.Client.Web.Tests""
if not defined ARGS if not defined WEB set "ARGS=--filter "FullyQualifiedName!~Cantrip.Client.Web.Tests""

rem /noweb builds the solution filter that leaves the web client out: it is the
rem slowest project by far and nothing /noweb runs needs it.
set "SOLUTION=%ROOT%\dotnet\Cantrip.sln"
if not defined WEB set "SOLUTION=%ROOT%\dotnet\Cantrip.NoWeb.slnf"

rem Where each step's output is captured, for :report to sift.
set "LOG=%TEMP%\cantrip-test-%RANDOM%%RANDOM%"

rem /engine: the .NET side is neither built nor tested.
set "RESULT=0"
if defined ENGINE goto engine

rem A build failure ends the run, but it still goes through :report. Exiting here
rem printed compiler errors and no verdict at all, which reads as success to
rem anything that only keeps the tail of the output.
if defined BUILD (
  if defined DETAILED (
    dotnet build "%SOLUTION%" -v q --nologo
  ) else (
    dotnet build "%SOLUTION%" -v q --nologo > "%LOG%-build.txt" 2>&1
  )
  if errorlevel 1 set "BUILDFAILED=1"
)

rem The .NET tests that play a table spawn the cantrip-native-host the cards profile last built, which nothing above
rem builds: a stale one plays without a card added since (a browser scenario's table then publishes nothing).
if defined BUILD if not defined BUILDFAILED (
  pushd "%ROOT%"
  if defined DETAILED (
    cargo build --profile cards -p cantrip-host --quiet
  ) else (
    cargo build --profile cards -p cantrip-host --quiet >> "%LOG%-build.txt" 2>&1
  )
  if errorlevel 1 set "BUILDFAILED=1"
  popd
)

if defined BUILDFAILED (
  set "RESULT=1"
  goto report
)

if defined DETAILED (
  dotnet test "%SOLUTION%" --no-build --nologo %ARGS%
) else (
  dotnet test "%SOLUTION%" --no-build --nologo %ARGS% > "%LOG%-dotnet.txt" 2>&1
)
set "RESULT=%ERRORLEVEL%"

:engine

rem The rules engine (the Rust workspace at the root): its unit tests and the scenario corpus. Run from the workspace's
rem directory so its rust-toolchain.toml chooses the toolchain; the first build takes minutes, later ones seconds.
rem CANTRIP_VERIFY=1 turns on the engine's own consistency checks while it plays (docs\engine-design.md). The profile is
rem `checked`, the cards profile with the debug assertions on (Cargo.toml): the engine's claims about itself
rem (`debug_assert!`) are checked over every test and scenario at release speed, where the cards profile drops them. A
rem /filter names dotnet tests, so it leaves these out.
if "%RESULT%"=="0" if not defined FILTERED (
  pushd "%ROOT%"
  set "CANTRIP_VERIFY=1"
  if defined DETAILED (
    cargo test --profile checked --workspace --quiet --no-fail-fast
  ) else (
    cargo test --profile checked --workspace --quiet --no-fail-fast > "%LOG%-engine.txt" 2>&1
  )
  if errorlevel 1 set "RESULT=1"
  popd
)

:report
if defined DETAILED goto verdict
set "REPORTED="
if defined BUILDFAILED (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\tools\maintenance\test-report.ps1" -Result %RESULT% -BuildFailed "%LOG%-build.txt" && set "REPORTED=1"
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\tools\maintenance\test-report.ps1" -Result %RESULT% "%LOG%-build.txt" "%LOG%-dotnet.txt" "%LOG%-engine.txt" && set "REPORTED=1"
)
rem The report failing to run is no reason to lose the output: it is printed whole.
if not defined REPORTED (
  for %%f in ("%LOG%-build.txt" "%LOG%-dotnet.txt" "%LOG%-engine.txt") do if exist %%f type %%f
)
del /q "%LOG%-build.txt" "%LOG%-dotnet.txt" "%LOG%-engine.txt" 2>nul
if defined REPORTED goto done

:verdict
echo.
if "%RESULT%"=="0" (
  echo All tests passed.
) else (
  if defined BUILDFAILED (echo Build failed. No tests ran.) else (echo Tests failed.)
)

:done
endlocal & exit /b %RESULT%

:usage
echo.
echo Usage: test.cmd [/all ^| /noweb ^| /engine] [/nobuild] [/filter EXPRESSION] [/detailed]
echo.
echo   (default)            the fast tests: none that run on the wall clock
echo   /all                 every test, browser and wall-clock tests included
echo   /noweb               every test but the web client's browser tests
echo   /nobuild             skip the build step
echo   /engine              the rules engine's tests alone: no .NET build or test
echo   /filter EXPRESSION   run only matching .NET tests, e.g. "FullyQualifiedName~Swiss"
echo   /detailed            list each test and print full failure messages
echo.
endlocal
exit /b 1
