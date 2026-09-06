@echo off
setlocal
set "HERE=%~dp0"
if not exist "%HERE%codex-pencil.exe" (
  echo Missing codex-pencil.exe next to Launch-Grammar.cmd
  exit /b 1
)
start "" /D "%HERE%" "%HERE%codex-pencil.exe"
exit /b %ERRORLEVEL%
