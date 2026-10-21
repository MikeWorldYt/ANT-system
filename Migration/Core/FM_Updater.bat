@echo off
:: FM_Updater.bat - replaces FolderManager.exe with a downloaded build, then relaunches it.
:: Args: %1 = full path to FolderManager.exe, %2 = download URL of the new exe

setlocal

set "EXE_PATH=%~1"
set "DOWNLOAD_URL=%~2"
set "EXE_NAME=%~nx1"
set "TMP_EXE=%~dp1FolderManager_new.exe"
set "BAK_EXE=%~dp1FolderManager_old.exe"

if "%EXE_PATH%"=="" goto end
if "%DOWNLOAD_URL%"=="" goto end
:: Only an installed exe is replaced; a source checkout (FolderManager.py) must never be overwritten
if /i not "%~x1"==".exe" goto end

echo Waiting for FolderManager to close...
set /a attempts=0
:: ping instead of timeout: timeout fails without a console (CREATE_NO_WINDOW)
:wait_loop
tasklist /fi "imagename eq %EXE_NAME%" 2>nul | find /i "%EXE_NAME%" >nul
if errorlevel 1 goto download
set /a attempts+=1
if %attempts% geq 10 (
    echo Timeout waiting for the app to close. Aborting update.
    goto end
)
ping -n 2 127.0.0.1 >nul
goto wait_loop

:download
echo Downloading update...
if exist "%TMP_EXE%" del /f /q "%TMP_EXE%" >nul 2>&1
powershell -NoProfile -NonInteractive -Command "Invoke-WebRequest -UseBasicParsing -Uri $env:DOWNLOAD_URL -OutFile $env:TMP_EXE" >nul 2>&1
if not exist "%TMP_EXE%" (
    echo Download failed. Keeping the current version.
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

:end
endlocal
:: Deletes this script once it has exited
start "" /b cmd /c "ping -n 3 127.0.0.1 >nul & del /f /q "%~f0""
exit /b
