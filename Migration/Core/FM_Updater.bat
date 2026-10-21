@echo off
:: FM_Updater.bat - replaces FolderManager.exe with a downloaded build, then relaunches it.
:: Args: %1 = full path to FolderManager.exe, %2 = download URL of the new exe

setlocal

set "EXE_PATH=%~1"
set "DOWNLOAD_URL=%~2"
set "EXE_NAME=%~nx1"
set "TMP_EXE=%~dp1FolderManager_new.exe"
set "BAK_EXE=%~dp1FolderManager_old.exe"

if "%EXE_PATH%"=="" goto keep
if "%DOWNLOAD_URL%"=="" goto keep
:: Only an installed exe is replaced; a source checkout (FolderManager.py) must never be overwritten
if /i not "%~x1"==".exe" goto keep

echo Waiting for FolderManager to close...
set /a attempts=0
:: ping instead of timeout: timeout fails without a console (CREATE_NO_WINDOW)
:wait_loop
tasklist /fi "imagename eq %EXE_NAME%" 2>nul | find /i "%EXE_NAME%" >nul
if errorlevel 1 goto download
set /a attempts+=1
if %attempts% geq 10 goto wait_timeout
ping -n 2 127.0.0.1 >nul
goto wait_loop

:wait_timeout
echo Timeout waiting for the app to close. Aborting update.
:: The script runs hidden, so the user gets a dialog; no relaunch, to avoid a close/reopen loop
powershell -NoProfile -NonInteractive -Command "Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show('FolderManager did not close within 10 seconds, so the update was cancelled. Close every FolderManager window and try again.', 'FolderManager update', 'OK', 'Warning')" >nul 2>&1
goto keep

:download
echo Downloading update...
if exist "%TMP_EXE%" del /f /q "%TMP_EXE%" >nul 2>&1
:: Exit code 0 only when the transfer finished and the file is a Windows executable (MZ header)
powershell -NoProfile -NonInteractive -Command "try { Invoke-WebRequest -UseBasicParsing -Uri $env:DOWNLOAD_URL -OutFile $env:TMP_EXE -ErrorAction Stop; $b = [IO.File]::ReadAllBytes($env:TMP_EXE); if ($b.Length -lt 1024 -or $b[0] -ne 0x4D -or $b[1] -ne 0x5A) { exit 1 }; exit 0 } catch { exit 1 }" >nul 2>&1
if errorlevel 1 (
    echo Download failed or incomplete. Keeping the current version.
    if exist "%TMP_EXE%" del /f /q "%TMP_EXE%" >nul 2>&1
    goto relaunch
)

echo Applying update...
if exist "%BAK_EXE%" del /f /q "%BAK_EXE%" >nul 2>&1
move /y "%EXE_PATH%" "%BAK_EXE%" >nul 2>&1
if errorlevel 1 (
    echo Could not replace the current version. Keeping it.
    del /f /q "%TMP_EXE%" >nul 2>&1
    goto relaunch
)
move /y "%TMP_EXE%" "%EXE_PATH%" >nul 2>&1
if errorlevel 1 (
    echo Replace failed. Restoring the previous version...
    move /y "%BAK_EXE%" "%EXE_PATH%" >nul 2>&1
    goto relaunch
)
del /f /q "%BAK_EXE%" >nul 2>&1
echo Update applied successfully.

:relaunch
echo Relaunching FolderManager...
start "" "%EXE_PATH%"

:: The script stays in 00.Core: FolderManager needs it for every future update
:keep
endlocal
exit /b
