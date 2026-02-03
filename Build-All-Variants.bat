@echo off

REM Force a new CMD window that stays open
if "%1" neq "ALREADY_RUNNING" (
    cmd /k "%~f0" ALREADY_RUNNING
    exit
)

setlocal enabledelayedexpansion

REM ====================================================================
REM sysmon - Build All Variants (Optimized)
REM ====================================================================
REM Builds three optimized variants:
REM 1. sysmon-dual.exe   - NVIDIA + AMD support
REM 2. sysmon-nvidia.exe - NVIDIA only
REM 3. sysmon-amd.exe    - AMD only
REM
REM Size optimization: ~400-500KB per binary (LTO + strip enabled)
REM ====================================================================

echo [%TIME%] Starting build process...
echo.

REM Add MinGW to PATH if it exists
if exist "C:\PortableRust\msys64\mingw64\bin" (
    echo Adding MinGW64 to PATH...
    SET "PATH=C:\PortableRust\msys64\mingw64\bin;%PATH%"
    echo.
)

if exist "C:\PortableRust\mingw64\bin" (
    echo Adding MinGW64 to PATH...
    SET "PATH=C:\PortableRust\mingw64\bin;%PATH%"
    echo.
)

REM Try to auto-detect cargo
SET CARGO_PATH=
for %%i in (cargo.exe) do SET CARGO_PATH=%%~$PATH:i

if not defined CARGO_PATH (
    SET CARGO_PATH=C:\Users\%USERNAME%\.cargo\bin\cargo.exe
)

if not exist "%CARGO_PATH%" (
    SET CARGO_PATH=C:\Users\Administrator\.cargo\bin\cargo.exe
)

REM Verify cargo exists
if not exist "%CARGO_PATH%" (
    echo [ERROR] Cargo not found. Install from https://rustup.rs/
    echo.
    goto END
)

REM Set optimization environment variables (native CPU optimizations only)
SET RUSTFLAGS=-C target-cpu=native
SET CARGO_INCREMENTAL=0

echo ========================================
echo Building sysmon - All Variants
echo ========================================
echo.
echo Cargo: %CARGO_PATH%
echo Target: Release (opt-level=z, fat LTO, stripped)
echo RUSTFLAGS: %RUSTFLAGS%
echo.

REM Clean previous builds
if exist target\release\sysmon*.exe (
    echo Cleaning previous builds...
    del /Q target\release\sysmon*.exe >nul 2>&1
    echo.
)

REM Track build status
SET BUILD_ERRORS=0

REM Build 1: Dual GPU support
echo [1/3] Building NVIDIA + AMD version...
"%CARGO_PATH%" build --release
if %ERRORLEVEL% EQU 0 (
    copy /Y target\release\sysmon.exe target\release\sysmon-dual.exe >nul
    del target\release\sysmon.exe
    echo [OK] sysmon-dual.exe
) else (
    echo [FAIL] Dual build failed
    SET /A BUILD_ERRORS=BUILD_ERRORS+1
)

echo.

REM Build 2: NVIDIA only
echo [2/3] Building NVIDIA-only version...
"%CARGO_PATH%" build --release --no-default-features --features nvidia
if %ERRORLEVEL% EQU 0 (
    copy /Y target\release\sysmon.exe target\release\sysmon-nvidia.exe >nul
    del target\release\sysmon.exe
    echo [OK] sysmon-nvidia.exe
) else (
    echo [FAIL] NVIDIA build failed
    SET /A BUILD_ERRORS=BUILD_ERRORS+1
)

echo.

REM Build 3: AMD only
echo [3/3] Building AMD-only version...
"%CARGO_PATH%" build --release --no-default-features --features amd
if %ERRORLEVEL% EQU 0 (
    copy /Y target\release\sysmon.exe target\release\sysmon-amd.exe >nul
    del target\release\sysmon.exe
    echo [OK] sysmon-amd.exe
) else (
    echo [FAIL] AMD build failed
    SET /A BUILD_ERRORS=BUILD_ERRORS+1
)

echo.

REM Summary
echo ========================================
echo Build Summary
echo ========================================
echo.

if %BUILD_ERRORS% EQU 0 (
    echo Status: All builds succeeded ✓
) else (
    echo Status: %BUILD_ERRORS% build^(s^) failed
)

echo.

REM List executables with sizes
if exist target\release\sysmon*.exe (
    echo Available executables:
    for %%F in (target\release\sysmon*.exe) do (
        SET size=%%~zF
        SET /A sizekb=!size!/1024
        echo   %%~nxF ^(!sizekb! KB^)
    )
    echo.
)

echo ========================================

:END
echo.
echo [%TIME%] Build process finished
echo.
echo Window will remain open. Type 'exit' to close.
echo.
