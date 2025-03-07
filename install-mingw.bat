@echo off
REM Script to install MinGW-w64 on Windows using MSYS2

echo === MinGW-w64 Installation Helper ===
echo This script will help you install MinGW-w64 for cross-compilation
echo.

REM Check if PowerShell is available
where powershell >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo PowerShell is required but not found.
    echo Please install PowerShell or run this script on a newer version of Windows.
    exit /b 1
)

REM Create a temporary directory for downloads
if not exist temp mkdir temp
cd temp

echo Downloading MSYS2 installer...
powershell -Command "& {Invoke-WebRequest -Uri 'https://github.com/msys2/msys2-installer/releases/download/2023-07-18/msys2-x86_64-20230718.exe' -OutFile 'msys2-installer.exe'}"

if not exist msys2-installer.exe (
    echo Failed to download MSYS2 installer.
    cd ..
    exit /b 1
)

echo.
echo Installing MSYS2...
echo Please follow the installation instructions in the MSYS2 installer.
echo Make sure to use the default installation path (C:\msys64).
echo.
echo Press any key to start the installer...
pause >nul

start /wait msys2-installer.exe

echo.
echo MSYS2 installation completed.
echo.
echo Now installing MinGW-w64 GCC...
echo This will open an MSYS2 terminal. Please wait for the installation to complete.
echo.
echo Press any key to continue...
pause >nul

REM Create a batch file to run in MSYS2
echo @echo off > install-gcc.bat
echo echo Updating MSYS2 packages... >> install-gcc.bat
echo C:\msys64\usr\bin\bash.exe -lc "pacman -Syu --noconfirm" >> install-gcc.bat
echo echo Installing MinGW-w64 GCC... >> install-gcc.bat
echo C:\msys64\usr\bin\bash.exe -lc "pacman -S --noconfirm mingw-w64-x86_64-gcc" >> install-gcc.bat
echo echo Installation completed. >> install-gcc.bat
echo echo. >> install-gcc.bat
echo echo Press any key to exit... >> install-gcc.bat
echo pause ^>nul >> install-gcc.bat

REM Run the batch file
call install-gcc.bat

REM Add MinGW to PATH
echo.
echo Adding MinGW to PATH...
setx PATH "%PATH%;C:\msys64\mingw64\bin" /M

echo.
echo MinGW-w64 installation completed.
echo.
echo You may need to restart your command prompt or computer for the PATH changes to take effect.
echo.
echo After restarting, you can verify the installation by running:
echo x86_64-w64-mingw32-gcc --version
echo.

REM Clean up
cd ..
rmdir /s /q temp

echo Press any key to exit...
pause >nul 