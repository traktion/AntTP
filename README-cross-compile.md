# AntTP Cross-Compilation Guide

This guide explains how to cross-compile AntTP for Windows using the provided scripts.

## Scripts Overview

We've created several scripts to help with cross-compilation:

1. **fix-target-and-compile.bat** - For Windows users to fix the target installation issue and cross-compile
2. **install-mingw.bat** - For Windows users to install MinGW-w64 (required for cross-compilation)
3. **fix-target-and-compile.sh** - For Linux/WSL users to fix the target installation issue and cross-compile
4. **cross-compile-windows.bat** - Original Windows cross-compilation script
5. **cross-compile-docker-fix.sh** - For Linux/WSL users to cross-compile using Docker
6. **cross-compile-all.sh** - For Linux/WSL users with multiple cross-compilation methods

## For Windows Users

### Prerequisites

- Rust installed (https://www.rust-lang.org/tools/install)
- MinGW-w64 installed (use the provided `install-mingw.bat` script if you don't have it)

### Step 1: Install MinGW-w64 (if not already installed)

Run the `install-mingw.bat` script:

```cmd
install-mingw.bat
```

This script will:
1. Download and install MSYS2
2. Install MinGW-w64 GCC
3. Add MinGW to your PATH

After installation, restart your command prompt or computer for the PATH changes to take effect.

### Step 2: Fix Target Installation and Cross-Compile

Run the `fix-target-and-compile.bat` script:

```cmd
fix-target-and-compile.bat
```

This script will:
1. Fix the crunchy crate issue
2. Fix the Rust target installation
3. Cross-compile AntTP for Windows

If you encounter any issues, the script will provide detailed error messages.

## For Linux/WSL Users

### Prerequisites

- Rust installed (https://www.rust-lang.org/tools/install)
- For Docker method: Docker installed

### Option 1: Using the fix-target-and-compile.sh Script

Run the script:

```bash
chmod +x fix-target-and-compile.sh
./fix-target-and-compile.sh
```

This script will:
1. Fix the crunchy crate issue
2. Install MinGW if needed
3. Fix the Rust target installation
4. Cross-compile AntTP for Windows

### Option 2: Using Docker

Run the Docker script:

```bash
chmod +x cross-compile-docker-fix.sh
./cross-compile-docker-fix.sh
```

### Option 3: Multiple Methods

Run the comprehensive script:

```bash
chmod +x cross-compile-all.sh
./cross-compile-all.sh
```

This script offers multiple cross-compilation methods.

## Troubleshooting

### Common Issues

1. **Missing MinGW**: If you see an error about missing `x86_64-w64-mingw32-gcc`, run the `install-mingw.bat` script (Windows) or install MinGW using your package manager (Linux).

2. **Target Installation Issues**: If you see an error like `can't find crate for 'core'`, the script should fix this by reinstalling the target.

3. **Crunchy Crate Issues**: The scripts automatically fix the crunchy crate path issue.

4. **PATH Issues**: After installing MinGW, make sure to restart your command prompt or computer.

### Getting Help

If you encounter any issues not covered here, please open an issue on the AntTP repository with the full error message and details about your environment.

## Binary Location

After successful cross-compilation, the Windows binary will be located at:

```
target/x86_64-pc-windows-gnu/release/anttp.exe
```

You can copy this file to a Windows machine and run it directly. 