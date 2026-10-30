@echo off
rem ---------------------------------------------------------------------------
rem In-person play through tools\scripts\in-person.ps1 (docs/in-person.md): two
rem players at one table, a phone each for hands and prompts, and one shared
rem tablet or a tablet each for the board. Builds the shipping web client if it
rem is out of date, starts the in-person server and opens its setup page, where
rem you name the players, pick the cube and the devices, and start a grid
rem draft; the page then shows a QR code for each device to scan.
rem
rem   in-person.cmd                     build if needed, then set up a table
rem   in-person.cmd -NoBuild            on the last build
rem   in-person.cmd -Fast               a quick build, interpreted to run
rem   in-person.cmd -Event ID           host an event already made again
rem   in-person.cmd -Address 192.168.1.20
rem                                     the address the devices reach this
rem                                     computer at, if the setup page's guess
rem                                     is wrong
rem
rem The devices reach this computer over the local network (or a phone's
rem hotspot): allow the server through the firewall when Windows asks. The
rem event is kept in a data directory (-Data, else CANTRIP_DATA, else
rem .cantrip-data in the checkout).
rem ---------------------------------------------------------------------------

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\in-person.ps1" %*

if errorlevel 1 pause
exit /b %errorlevel%
