@echo off
rem Roda o SimAnt original no DOSBox-X (ambiente de referência). Ver docs/decisoes.md.
cd /d "%~dp0.."
start "" "referencia\dosbox-x\bin\x64\Release\dosbox-x.exe" -conf "referencia\dosbox-x.conf" -nopromptfolder %*
