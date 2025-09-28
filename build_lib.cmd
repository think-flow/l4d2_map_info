@echo off
chcp 65001 >nul

rem 调用 PowerShell 发布和复制脚本
pwsh -NoProfile -ExecutionPolicy Bypass -File "%CD%\build_lib.ps1"
if errorlevel 1 (
    echo PowerShell 脚本执行失败！
    pause
    exit /b 1
)

echo. 
pause