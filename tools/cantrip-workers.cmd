@echo off
rem The worker manager: leases on compute machines, and every call made on one. See docs\workers.md.
python "%~dp0workers\cantrip-workers.py" %*
