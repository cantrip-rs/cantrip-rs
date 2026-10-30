@echo off
rem ---------------------------------------------------------------------------
rem The advisor's table: you sit in the AI's
rem seat against a bot (the neural bot or the tactical bot), and beside the
rem table the AI says what it would do at every decision of yours - its
rem choice, its top candidates with the network's prior, the search's visit
rem share and win chance, how much its sampled worlds disagreed, and its
rem belief about the opponent's hand. The play you make is what happens. Every
rem decision is recorded under ..\ai-data\sessions\<date>\<game>\ beside the repository.
rem
rem   advisor.cmd                                  asks for the decks and the bot
rem   advisor.cmd -Mine dimir-midrange -Theirs boros-dragons -Opponent neural
rem   advisor.cmd -Budget 5000 -Seed 7 -NoBuild
rem
rem It starts its own inference server (port 7841), its own table and AI host,
rem and the page (port 5390) in your browser. Ctrl+C in this window stops it
rem all. See tools\scripts\advisor.ps1 for every option.
rem ---------------------------------------------------------------------------

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\advisor.ps1" %*
if errorlevel 1 pause
exit /b %errorlevel%
