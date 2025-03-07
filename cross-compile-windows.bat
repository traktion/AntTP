@echo off
REM Script to cross-compile AntTP on Windows

echo === AntTP Windows Cross-Compilation Helper ===
echo This script will help you cross-compile AntTP on Windows
echo.

REM Check if Rust is installed
where rustc >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo Rust is not installed. Please install Rust first:
    echo https://www.rust-lang.org/tools/install
    exit /b 1
)

REM Fix the crunchy crate
echo Fixing crunchy crate...

REM Create directory structure
if not exist crunchy-fix\src mkdir crunchy-fix\src

REM Create the build.rs file
echo use std::env; > crunchy-fix\build.rs
echo use std::fs::File; >> crunchy-fix\build.rs
echo use std::io::Write; >> crunchy-fix\build.rs
echo use std::path::Path; >> crunchy-fix\build.rs
echo. >> crunchy-fix\build.rs
echo fn main() { >> crunchy-fix\build.rs
echo     let out_dir = env::var("OUT_DIR").unwrap(); >> crunchy-fix\build.rs
echo     let dest_path = Path::new(^&out_dir).join("lib.rs"); >> crunchy-fix\build.rs
echo     let mut f = File::create(^&dest_path).unwrap(); >> crunchy-fix\build.rs
echo. >> crunchy-fix\build.rs
echo     let crunchy_lib = r#" >> crunchy-fix\build.rs
echo /// Unroll the given for loop >> crunchy-fix\build.rs
echo /// >> crunchy-fix\build.rs
echo /// Example: >> crunchy-fix\build.rs
echo /// >> crunchy-fix\build.rs
echo /// ```rust >> crunchy-fix\build.rs
echo /// # #[macro_use] extern crate crunchy; >> crunchy-fix\build.rs
echo /// # fn main() { >> crunchy-fix\build.rs
echo /// let mut x = 0; >> crunchy-fix\build.rs
echo /// unroll! { >> crunchy-fix\build.rs
echo ///     for i in 0..10 { >> crunchy-fix\build.rs
echo ///         x += i; >> crunchy-fix\build.rs
echo ///     } >> crunchy-fix\build.rs
echo /// } >> crunchy-fix\build.rs
echo /// # } >> crunchy-fix\build.rs
echo /// ``` >> crunchy-fix\build.rs
echo #[macro_export] >> crunchy-fix\build.rs
echo macro_rules! unroll { >> crunchy-fix\build.rs
echo     (for $v:ident in 0..$e:expr $c:block) =^> { >> crunchy-fix\build.rs
echo         { >> crunchy-fix\build.rs
echo             let max = $e; >> crunchy-fix\build.rs
echo             assert!(max ^<= 128, "Unroll is not designed for large loops"); >> crunchy-fix\build.rs
echo             #[allow(unused_comparisons)] >> crunchy-fix\build.rs
echo             { >> crunchy-fix\build.rs
echo                 if max ^> 0 { let $v = 0; $c } >> crunchy-fix\build.rs
echo                 if max ^> 1 { let $v = 1; $c } >> crunchy-fix\build.rs
echo                 if max ^> 2 { let $v = 2; $c } >> crunchy-fix\build.rs
echo                 if max ^> 3 { let $v = 3; $c } >> crunchy-fix\build.rs
echo                 if max ^> 4 { let $v = 4; $c } >> crunchy-fix\build.rs
echo                 if max ^> 5 { let $v = 5; $c } >> crunchy-fix\build.rs
echo                 if max ^> 6 { let $v = 6; $c } >> crunchy-fix\build.rs
echo                 if max ^> 7 { let $v = 7; $c } >> crunchy-fix\build.rs
echo                 if max ^> 8 { let $v = 8; $c } >> crunchy-fix\build.rs
echo                 if max ^> 9 { let $v = 9; $c } >> crunchy-fix\build.rs
echo                 if max ^> 10 { let $v = 10; $c } >> crunchy-fix\build.rs
echo                 if max ^> 11 { let $v = 11; $c } >> crunchy-fix\build.rs
echo                 if max ^> 12 { let $v = 12; $c } >> crunchy-fix\build.rs
echo                 if max ^> 13 { let $v = 13; $c } >> crunchy-fix\build.rs
echo                 if max ^> 14 { let $v = 14; $c } >> crunchy-fix\build.rs
echo                 if max ^> 15 { let $v = 15; $c } >> crunchy-fix\build.rs
echo                 if max ^> 16 { let $v = 16; $c } >> crunchy-fix\build.rs
echo                 if max ^> 17 { let $v = 17; $c } >> crunchy-fix\build.rs
echo                 if max ^> 18 { let $v = 18; $c } >> crunchy-fix\build.rs
echo                 if max ^> 19 { let $v = 19; $c } >> crunchy-fix\build.rs
echo                 if max ^> 20 { let $v = 20; $c } >> crunchy-fix\build.rs
echo                 if max ^> 21 { let $v = 21; $c } >> crunchy-fix\build.rs
echo                 if max ^> 22 { let $v = 22; $c } >> crunchy-fix\build.rs
echo                 if max ^> 23 { let $v = 23; $c } >> crunchy-fix\build.rs
echo                 if max ^> 24 { let $v = 24; $c } >> crunchy-fix\build.rs
echo                 if max ^> 25 { let $v = 25; $c } >> crunchy-fix\build.rs
echo                 if max ^> 26 { let $v = 26; $c } >> crunchy-fix\build.rs
echo                 if max ^> 27 { let $v = 27; $c } >> crunchy-fix\build.rs
echo                 if max ^> 28 { let $v = 28; $c } >> crunchy-fix\build.rs
echo                 if max ^> 29 { let $v = 29; $c } >> crunchy-fix\build.rs
echo                 if max ^> 30 { let $v = 30; $c } >> crunchy-fix\build.rs
echo                 if max ^> 31 { let $v = 31; $c } >> crunchy-fix\build.rs
echo                 if max ^> 32 { let $v = 32; $c } >> crunchy-fix\build.rs
echo                 if max ^> 33 { let $v = 33; $c } >> crunchy-fix\build.rs
echo                 if max ^> 34 { let $v = 34; $c } >> crunchy-fix\build.rs
echo                 if max ^> 35 { let $v = 35; $c } >> crunchy-fix\build.rs
echo                 if max ^> 36 { let $v = 36; $c } >> crunchy-fix\build.rs
echo                 if max ^> 37 { let $v = 37; $c } >> crunchy-fix\build.rs
echo                 if max ^> 38 { let $v = 38; $c } >> crunchy-fix\build.rs
echo                 if max ^> 39 { let $v = 39; $c } >> crunchy-fix\build.rs
echo                 if max ^> 40 { let $v = 40; $c } >> crunchy-fix\build.rs
echo                 if max ^> 41 { let $v = 41; $c } >> crunchy-fix\build.rs
echo                 if max ^> 42 { let $v = 42; $c } >> crunchy-fix\build.rs
echo                 if max ^> 43 { let $v = 43; $c } >> crunchy-fix\build.rs
echo                 if max ^> 44 { let $v = 44; $c } >> crunchy-fix\build.rs
echo                 if max ^> 45 { let $v = 45; $c } >> crunchy-fix\build.rs
echo                 if max ^> 46 { let $v = 46; $c } >> crunchy-fix\build.rs
echo                 if max ^> 47 { let $v = 47; $c } >> crunchy-fix\build.rs
echo                 if max ^> 48 { let $v = 48; $c } >> crunchy-fix\build.rs
echo                 if max ^> 49 { let $v = 49; $c } >> crunchy-fix\build.rs
echo                 if max ^> 50 { let $v = 50; $c } >> crunchy-fix\build.rs
echo                 if max ^> 51 { let $v = 51; $c } >> crunchy-fix\build.rs
echo                 if max ^> 52 { let $v = 52; $c } >> crunchy-fix\build.rs
echo                 if max ^> 53 { let $v = 53; $c } >> crunchy-fix\build.rs
echo                 if max ^> 54 { let $v = 54; $c } >> crunchy-fix\build.rs
echo                 if max ^> 55 { let $v = 55; $c } >> crunchy-fix\build.rs
echo                 if max ^> 56 { let $v = 56; $c } >> crunchy-fix\build.rs
echo                 if max ^> 57 { let $v = 57; $c } >> crunchy-fix\build.rs
echo                 if max ^> 58 { let $v = 58; $c } >> crunchy-fix\build.rs
echo                 if max ^> 59 { let $v = 59; $c } >> crunchy-fix\build.rs
echo                 if max ^> 60 { let $v = 60; $c } >> crunchy-fix\build.rs
echo                 if max ^> 61 { let $v = 61; $c } >> crunchy-fix\build.rs
echo                 if max ^> 62 { let $v = 62; $c } >> crunchy-fix\build.rs
echo                 if max ^> 63 { let $v = 63; $c } >> crunchy-fix\build.rs
echo                 if max ^> 64 { let $v = 64; $c } >> crunchy-fix\build.rs
echo                 if max ^> 65 { let $v = 65; $c } >> crunchy-fix\build.rs
echo                 if max ^> 66 { let $v = 66; $c } >> crunchy-fix\build.rs
echo                 if max ^> 67 { let $v = 67; $c } >> crunchy-fix\build.rs
echo                 if max ^> 68 { let $v = 68; $c } >> crunchy-fix\build.rs
echo                 if max ^> 69 { let $v = 69; $c } >> crunchy-fix\build.rs
echo                 if max ^> 70 { let $v = 70; $c } >> crunchy-fix\build.rs
echo                 if max ^> 71 { let $v = 71; $c } >> crunchy-fix\build.rs
echo                 if max ^> 72 { let $v = 72; $c } >> crunchy-fix\build.rs
echo                 if max ^> 73 { let $v = 73; $c } >> crunchy-fix\build.rs
echo                 if max ^> 74 { let $v = 74; $c } >> crunchy-fix\build.rs
echo                 if max ^> 75 { let $v = 75; $c } >> crunchy-fix\build.rs
echo                 if max ^> 76 { let $v = 76; $c } >> crunchy-fix\build.rs
echo                 if max ^> 77 { let $v = 77; $c } >> crunchy-fix\build.rs
echo                 if max ^> 78 { let $v = 78; $c } >> crunchy-fix\build.rs
echo                 if max ^> 79 { let $v = 79; $c } >> crunchy-fix\build.rs
echo                 if max ^> 80 { let $v = 80; $c } >> crunchy-fix\build.rs
echo                 if max ^> 81 { let $v = 81; $c } >> crunchy-fix\build.rs
echo                 if max ^> 82 { let $v = 82; $c } >> crunchy-fix\build.rs
echo                 if max ^> 83 { let $v = 83; $c } >> crunchy-fix\build.rs
echo                 if max ^> 84 { let $v = 84; $c } >> crunchy-fix\build.rs
echo                 if max ^> 85 { let $v = 85; $c } >> crunchy-fix\build.rs
echo                 if max ^> 86 { let $v = 86; $c } >> crunchy-fix\build.rs
echo                 if max ^> 87 { let $v = 87; $c } >> crunchy-fix\build.rs
echo                 if max ^> 88 { let $v = 88; $c } >> crunchy-fix\build.rs
echo                 if max ^> 89 { let $v = 89; $c } >> crunchy-fix\build.rs
echo                 if max ^> 90 { let $v = 90; $c } >> crunchy-fix\build.rs
echo                 if max ^> 91 { let $v = 91; $c } >> crunchy-fix\build.rs
echo                 if max ^> 92 { let $v = 92; $c } >> crunchy-fix\build.rs
echo                 if max ^> 93 { let $v = 93; $c } >> crunchy-fix\build.rs
echo                 if max ^> 94 { let $v = 94; $c } >> crunchy-fix\build.rs
echo                 if max ^> 95 { let $v = 95; $c } >> crunchy-fix\build.rs
echo                 if max ^> 96 { let $v = 96; $c } >> crunchy-fix\build.rs
echo                 if max ^> 97 { let $v = 97; $c } >> crunchy-fix\build.rs
echo                 if max ^> 98 { let $v = 98; $c } >> crunchy-fix\build.rs
echo                 if max ^> 99 { let $v = 99; $c } >> crunchy-fix\build.rs
echo                 if max ^> 100 { let $v = 100; $c } >> crunchy-fix\build.rs
echo                 if max ^> 101 { let $v = 101; $c } >> crunchy-fix\build.rs
echo                 if max ^> 102 { let $v = 102; $c } >> crunchy-fix\build.rs
echo                 if max ^> 103 { let $v = 103; $c } >> crunchy-fix\build.rs
echo                 if max ^> 104 { let $v = 104; $c } >> crunchy-fix\build.rs
echo                 if max ^> 105 { let $v = 105; $c } >> crunchy-fix\build.rs
echo                 if max ^> 106 { let $v = 106; $c } >> crunchy-fix\build.rs
echo                 if max ^> 107 { let $v = 107; $c } >> crunchy-fix\build.rs
echo                 if max ^> 108 { let $v = 108; $c } >> crunchy-fix\build.rs
echo                 if max ^> 109 { let $v = 109; $c } >> crunchy-fix\build.rs
echo                 if max ^> 110 { let $v = 110; $c } >> crunchy-fix\build.rs
echo                 if max ^> 111 { let $v = 111; $c } >> crunchy-fix\build.rs
echo                 if max ^> 112 { let $v = 112; $c } >> crunchy-fix\build.rs
echo                 if max ^> 113 { let $v = 113; $c } >> crunchy-fix\build.rs
echo                 if max ^> 114 { let $v = 114; $c } >> crunchy-fix\build.rs
echo                 if max ^> 115 { let $v = 115; $c } >> crunchy-fix\build.rs
echo                 if max ^> 116 { let $v = 116; $c } >> crunchy-fix\build.rs
echo                 if max ^> 117 { let $v = 117; $c } >> crunchy-fix\build.rs
echo                 if max ^> 118 { let $v = 118; $c } >> crunchy-fix\build.rs
echo                 if max ^> 119 { let $v = 119; $c } >> crunchy-fix\build.rs
echo                 if max ^> 120 { let $v = 120; $c } >> crunchy-fix\build.rs
echo                 if max ^> 121 { let $v = 121; $c } >> crunchy-fix\build.rs
echo                 if max ^> 122 { let $v = 122; $c } >> crunchy-fix\build.rs
echo                 if max ^> 123 { let $v = 123; $c } >> crunchy-fix\build.rs
echo                 if max ^> 124 { let $v = 124; $c } >> crunchy-fix\build.rs
echo                 if max ^> 125 { let $v = 125; $c } >> crunchy-fix\build.rs
echo                 if max ^> 126 { let $v = 126; $c } >> crunchy-fix\build.rs
echo                 if max ^> 127 { let $v = 127; $c } >> crunchy-fix\build.rs
echo             } >> crunchy-fix\build.rs
echo         } >> crunchy-fix\build.rs
echo     } >> crunchy-fix\build.rs
echo } >> crunchy-fix\build.rs
echo "#; >> crunchy-fix\build.rs
echo. >> crunchy-fix\build.rs
echo     f.write_all(crunchy_lib.as_bytes()).unwrap(); >> crunchy-fix\build.rs
echo } >> crunchy-fix\build.rs

REM Create the lib.rs file with no_std support and forward slashes
echo #![cfg_attr(not(feature = "std"), no_std)] > crunchy-fix\src\lib.rs
echo. >> crunchy-fix\src\lib.rs
echo include!(concat!(env!("OUT_DIR"), "/lib.rs")); >> crunchy-fix\src\lib.rs

REM Create the Cargo.toml file with the std feature
echo [package] > crunchy-fix\Cargo.toml
echo name = "crunchy" >> crunchy-fix\Cargo.toml
echo version = "0.2.3" >> crunchy-fix\Cargo.toml
echo authors = ["Parity Technologies <admin@parity.io>"] >> crunchy-fix\Cargo.toml
echo description = "Crunchy unrolled loops" >> crunchy-fix\Cargo.toml
echo license = "MIT" >> crunchy-fix\Cargo.toml
echo repository = "https://github.com/paritytech/crunchy" >> crunchy-fix\Cargo.toml
echo documentation = "https://docs.rs/crunchy" >> crunchy-fix\Cargo.toml
echo. >> crunchy-fix\Cargo.toml
echo [features] >> crunchy-fix\Cargo.toml
echo default = ["std"] >> crunchy-fix\Cargo.toml
echo std = [] >> crunchy-fix\Cargo.toml

REM Update the main Cargo.toml to use our fixed crunchy crate
echo. >> Cargo.toml
echo [patch.crates-io] >> Cargo.toml
echo crunchy = { path = "./crunchy-fix" } >> Cargo.toml

echo Successfully fixed crunchy crate

REM Check if the Windows target is installed
rustup target list | findstr "x86_64-pc-windows-gnu" >nul
if %ERRORLEVEL% neq 0 (
    echo Adding Windows target...
    rustup target add x86_64-pc-windows-gnu
)

REM Clean the target directory to ensure a fresh build
echo Cleaning previous build artifacts...
cargo clean --target x86_64-pc-windows-gnu

REM Build the project
echo Building for Windows...
cargo build --release --target x86_64-pc-windows-gnu

if %ERRORLEVEL% equ 0 (
    echo Cross-compilation completed successfully!
    echo Binary location: %CD%\target\x86_64-pc-windows-gnu\release\anttp.exe
) else (
    echo Cross-compilation failed.
    exit /b 1
)

echo.
echo Press any key to exit...
pause >nul 