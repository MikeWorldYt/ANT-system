@echo off
 ─────────────────────────────────────────────────────────────
  FM_Updater.bat — FolderManager auto-updater
  Args %1 = full path to FolderManager.exe
        %2 = download URL for new .exe
 ─────────────────────────────────────────────────────────────

setlocal

set EXE_PATH=%~1
set DOWNLOAD_URL=%~2
set EXE_DIR=%~dp1
set TMP_EXE=%EXE_DIR%FolderManager_new.exe

 Wait for FolderManager.exe to fully close (up to 10s)
echo Waiting for FolderManager to close...
set a attempts=0
wait_loop
    timeout t 1 nobreak nul
    tasklist fi imagename eq FolderManager.exe 2nul  find i FolderManager.exe nul
    if not errorlevel 1 (
        set a attempts+=1
        if %attempts% lss 10 goto wait_loop
        echo Timeout waiting for app to close. Aborting update.
        goto end
    )

 Download new version
echo Downloading update...
powershell -NoProfile -NonInteractive -Command ^
    Invoke-WebRequest -Uri '%DOWNLOAD_URL%' -OutFile '%TMP_EXE%' ^
    nul 2&1

if not exist %TMP_EXE% (
    echo Download failed. Aborting update.
    goto relaunch
)

 Replace old exe with new one
echo Applying update...
del f q %EXE_PATH% nul 2&1
move y %TMP_EXE% %EXE_PATH% nul 2&1

if not exist %EXE_PATH% (
    echo Replace failed. Restoring...
    move y %TMP_EXE% %EXE_PATH% nul 2&1
    goto relaunch
)

echo Update applied successfully.

relaunch
 Relaunch FolderManager
echo Relaunching FolderManager...
start  %EXE_PATH%

end
 Self-delete this script after a short delay
start b cmd c timeout t 2 nobreak nul & del f q %~f0
exit