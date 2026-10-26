@echo off
REM Local build script for Discord PS5 homebrew package (Windows)
REM Run this in PowerShell or CMD with PS5SDK installed

set TITLE_ID=DISCORDPS5
set TITLE=Discord PS5
set VERSION=0.1.0
set APP_VER=01.00
set CONTENT_ID=DISCORDPS5_001
set CATEGORY=media
set SYSTEM_VER=5.00

set BINARY_PATH=target\aarch64-unknown-freebsd\release\discord-app
set OUTPUT_PKG=discord-ps5.pkg
set WORK_DIR=build\pkg
set PS5SDK_DIR=%PS5_SDK_PATH%

if "%PS5SDK_DIR%"=="" (
    set PS5SDK_DIR=C:\PS5SDK
)

echo ==========================================
echo Discord PS5 Package Builder
echo ==========================================
echo Title ID: %TITLE_ID%
echo Title: %TITLE%
echo Version: %VERSION%
echo Category: %CATEGORY% (appears in Media tab)
echo.

if not exist "%BINARY_PATH%" (
    echo Error: Binary not found at %BINARY_PATH%
    echo Please build the project first with:
    echo   cargo build --release --target aarch64-unknown-freebsd --bin discord-app
    exit /b 1
)

if not exist "%PS5SDK_DIR%" (
    echo Warning: PS5SDK not found at %PS5SDK_DIR%
    echo Packaging tools may not be available.
    echo Set PS5_SDK_PATH environment variable if installed elsewhere.
)

echo Building package...
python packaging\build_pkg.py ^
    "%BINARY_PATH%" ^
    -o "%OUTPUT_PKG%" ^
    --title-id "%TITLE_ID%" ^
    --title "%TITLE%" ^
    --version "%VERSION%" ^
    --app-ver "%APP_VER%" ^
    --content-id "%CONTENT_ID%" ^
    --category "%CATEGORY%" ^
    --system-ver "%SYSTEM_VER%" ^
    --ps5sdk "%PS5SDK_DIR%" ^
    --work-dir "%WORK_DIR%"

if exist "%OUTPUT_PKG%" (
    echo.
    echo Success! Package created: %OUTPUT_PKG%
    echo.
    echo To install on PS5:
    echo   1. Copy %OUTPUT_PKG% to a USB drive
    echo   2. On PS5, go to Settings ^> System ^> Package Installer
    echo   3. Install from USB
    echo   4. The app will appear in the Media tab
) else (
    echo.
    echo Package structure created at %WORK_DIR%\package
    echo Use orbis-pub-gen or pkg-python to create final .pkg
)