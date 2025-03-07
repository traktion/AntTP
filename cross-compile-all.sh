#!/bin/bash
# Comprehensive script for cross-compiling AntTP to Windows
# This script handles all necessary fixes and checks

set -e  # Exit on error

# Colors for better readability
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== AntTP Cross-Compilation Helper ===${NC}"
echo -e "${BLUE}This script will help you cross-compile AntTP to Windows${NC}"
echo ""

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to fix the crunchy crate
fix_crunchy_crate() {
    echo -e "${YELLOW}Fixing crunchy crate...${NC}"
    
    # Create directory structure
    mkdir -p crunchy-fix/src
    
    # Create the build.rs file
    cat > crunchy-fix/build.rs << 'EOF'
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("lib.rs");
    let mut f = File::create(&dest_path).unwrap();

    let crunchy_lib = r#"
/// Unroll the given for loop
///
/// Example:
///
/// ```rust
/// # #[macro_use] extern crate crunchy;
/// # fn main() {
/// let mut x = 0;
/// unroll! {
///     for i in 0..10 {
///         x += i;
///     }
/// }
/// # }
/// ```
#[macro_export]
macro_rules! unroll {
    (for $v:ident in 0..$e:expr $c:block) => {
        {
            let max = $e;
            assert!(max <= 128, "Unroll is not designed for large loops");
            #[allow(unused_comparisons)]
            {
                if max > 0 { let $v = 0; $c }
                if max > 1 { let $v = 1; $c }
                if max > 2 { let $v = 2; $c }
                if max > 3 { let $v = 3; $c }
                if max > 4 { let $v = 4; $c }
                if max > 5 { let $v = 5; $c }
                if max > 6 { let $v = 6; $c }
                if max > 7 { let $v = 7; $c }
                if max > 8 { let $v = 8; $c }
                if max > 9 { let $v = 9; $c }
                if max > 10 { let $v = 10; $c }
                if max > 11 { let $v = 11; $c }
                if max > 12 { let $v = 12; $c }
                if max > 13 { let $v = 13; $c }
                if max > 14 { let $v = 14; $c }
                if max > 15 { let $v = 15; $c }
                if max > 16 { let $v = 16; $c }
                if max > 17 { let $v = 17; $c }
                if max > 18 { let $v = 18; $c }
                if max > 19 { let $v = 19; $c }
                if max > 20 { let $v = 20; $c }
                if max > 21 { let $v = 21; $c }
                if max > 22 { let $v = 22; $c }
                if max > 23 { let $v = 23; $c }
                if max > 24 { let $v = 24; $c }
                if max > 25 { let $v = 25; $c }
                if max > 26 { let $v = 26; $c }
                if max > 27 { let $v = 27; $c }
                if max > 28 { let $v = 28; $c }
                if max > 29 { let $v = 29; $c }
                if max > 30 { let $v = 30; $c }
                if max > 31 { let $v = 31; $c }
                if max > 32 { let $v = 32; $c }
                if max > 33 { let $v = 33; $c }
                if max > 34 { let $v = 34; $c }
                if max > 35 { let $v = 35; $c }
                if max > 36 { let $v = 36; $c }
                if max > 37 { let $v = 37; $c }
                if max > 38 { let $v = 38; $c }
                if max > 39 { let $v = 39; $c }
                if max > 40 { let $v = 40; $c }
                if max > 41 { let $v = 41; $c }
                if max > 42 { let $v = 42; $c }
                if max > 43 { let $v = 43; $c }
                if max > 44 { let $v = 44; $c }
                if max > 45 { let $v = 45; $c }
                if max > 46 { let $v = 46; $c }
                if max > 47 { let $v = 47; $c }
                if max > 48 { let $v = 48; $c }
                if max > 49 { let $v = 49; $c }
                if max > 50 { let $v = 50; $c }
                if max > 51 { let $v = 51; $c }
                if max > 52 { let $v = 52; $c }
                if max > 53 { let $v = 53; $c }
                if max > 54 { let $v = 54; $c }
                if max > 55 { let $v = 55; $c }
                if max > 56 { let $v = 56; $c }
                if max > 57 { let $v = 57; $c }
                if max > 58 { let $v = 58; $c }
                if max > 59 { let $v = 59; $c }
                if max > 60 { let $v = 60; $c }
                if max > 61 { let $v = 61; $c }
                if max > 62 { let $v = 62; $c }
                if max > 63 { let $v = 63; $c }
                if max > 64 { let $v = 64; $c }
                if max > 65 { let $v = 65; $c }
                if max > 66 { let $v = 66; $c }
                if max > 67 { let $v = 67; $c }
                if max > 68 { let $v = 68; $c }
                if max > 69 { let $v = 69; $c }
                if max > 70 { let $v = 70; $c }
                if max > 71 { let $v = 71; $c }
                if max > 72 { let $v = 72; $c }
                if max > 73 { let $v = 73; $c }
                if max > 74 { let $v = 74; $c }
                if max > 75 { let $v = 75; $c }
                if max > 76 { let $v = 76; $c }
                if max > 77 { let $v = 77; $c }
                if max > 78 { let $v = 78; $c }
                if max > 79 { let $v = 79; $c }
                if max > 80 { let $v = 80; $c }
                if max > 81 { let $v = 81; $c }
                if max > 82 { let $v = 82; $c }
                if max > 83 { let $v = 83; $c }
                if max > 84 { let $v = 84; $c }
                if max > 85 { let $v = 85; $c }
                if max > 86 { let $v = 86; $c }
                if max > 87 { let $v = 87; $c }
                if max > 88 { let $v = 88; $c }
                if max > 89 { let $v = 89; $c }
                if max > 90 { let $v = 90; $c }
                if max > 91 { let $v = 91; $c }
                if max > 92 { let $v = 92; $c }
                if max > 93 { let $v = 93; $c }
                if max > 94 { let $v = 94; $c }
                if max > 95 { let $v = 95; $c }
                if max > 96 { let $v = 96; $c }
                if max > 97 { let $v = 97; $c }
                if max > 98 { let $v = 98; $c }
                if max > 99 { let $v = 99; $c }
                if max > 100 { let $v = 100; $c }
                if max > 101 { let $v = 101; $c }
                if max > 102 { let $v = 102; $c }
                if max > 103 { let $v = 103; $c }
                if max > 104 { let $v = 104; $c }
                if max > 105 { let $v = 105; $c }
                if max > 106 { let $v = 106; $c }
                if max > 107 { let $v = 107; $c }
                if max > 108 { let $v = 108; $c }
                if max > 109 { let $v = 109; $c }
                if max > 110 { let $v = 110; $c }
                if max > 111 { let $v = 111; $c }
                if max > 112 { let $v = 112; $c }
                if max > 113 { let $v = 113; $c }
                if max > 114 { let $v = 114; $c }
                if max > 115 { let $v = 115; $c }
                if max > 116 { let $v = 116; $c }
                if max > 117 { let $v = 117; $c }
                if max > 118 { let $v = 118; $c }
                if max > 119 { let $v = 119; $c }
                if max > 120 { let $v = 120; $c }
                if max > 121 { let $v = 121; $c }
                if max > 122 { let $v = 122; $c }
                if max > 123 { let $v = 123; $c }
                if max > 124 { let $v = 124; $c }
                if max > 125 { let $v = 125; $c }
                if max > 126 { let $v = 126; $c }
                if max > 127 { let $v = 127; $c }
            }
        }
    }
}
"#;

    f.write_all(crunchy_lib.as_bytes()).unwrap();
}
EOF
    
    # Create the lib.rs file with no_std support and forward slashes
    cat > crunchy-fix/src/lib.rs << 'EOF'
#![cfg_attr(not(feature = "std"), no_std)]

include!(concat!(env!("OUT_DIR"), "/lib.rs"));
EOF
    
    # Create the Cargo.toml file with the std feature
    cat > crunchy-fix/Cargo.toml << 'EOF'
[package]
name = "crunchy"
version = "0.2.3"
authors = ["Parity Technologies <admin@parity.io>"]
description = "Crunchy unrolled loops"
license = "MIT"
repository = "https://github.com/paritytech/crunchy"
documentation = "https://docs.rs/crunchy"

[features]
default = ["std"]
std = []
EOF
    
    # Update the main Cargo.toml to use our fixed crunchy crate
    # First, check if there's already a patch section
    if grep -q "\[patch.crates-io\]" Cargo.toml; then
        # Check if there's already a crunchy patch
        if grep -q "crunchy.*=.*{.*path.*=.*\"./crunchy-fix\"" Cargo.toml; then
            echo -e "${GREEN}Crunchy patch already exists in Cargo.toml${NC}"
        else
            # Add our patch to the existing section
            sed -i '/\[patch.crates-io\]/a crunchy = { path = "./crunchy-fix" }' Cargo.toml
        fi
    else
        # Add a new patch section
        cat >> Cargo.toml << 'EOF'

[patch.crates-io]
crunchy = { path = "./crunchy-fix" }
EOF
    fi
    
    # Remove any specific crunchy dependency if it exists
    sed -i '/\[dependencies.crunchy\]/,/version = ".*"/d' Cargo.toml
    
    echo -e "${GREEN}Successfully fixed crunchy crate${NC}"
}

# Function to install MinGW for direct cross-compilation
install_mingw() {
    echo -e "${YELLOW}Installing MinGW for cross-compilation...${NC}"
    sudo apt-get update
    sudo apt-get install -y mingw-w64
    echo -e "${GREEN}MinGW installed successfully${NC}"
}

# Function to install Docker
install_docker() {
    echo -e "${YELLOW}Installing Docker...${NC}"
    
    # Update package information
    sudo apt-get update
    
    # Install prerequisites
    sudo apt-get install -y \
        apt-transport-https \
        ca-certificates \
        curl \
        gnupg \
        lsb-release
    
    # Add Docker's official GPG key
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
    
    # Set up the stable repository
    echo \
      "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu \
      $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
    
    # Update package information with Docker repository
    sudo apt-get update
    
    # Install Docker Engine
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io
    
    # Add current user to the docker group
    sudo usermod -aG docker $USER
    
    # Start Docker service
    sudo service docker start
    
    echo -e "${GREEN}Docker installed successfully${NC}"
    echo -e "${YELLOW}You may need to restart your WSL session to use Docker without sudo${NC}"
    echo -e "${YELLOW}For now, we'll use sudo for Docker commands${NC}"
}

# Function to cross-compile using Docker
cross_compile_with_docker() {
    echo -e "${YELLOW}Cross-compiling with Docker...${NC}"
    
    # Create a Docker image for cross-compilation
    echo -e "${YELLOW}Creating Docker image for cross-compilation...${NC}"
    cat > Dockerfile.cross << 'EOF'
FROM rust:latest

# Install cross-compilation tools
RUN apt-get update && apt-get install -y \
    mingw-w64 \
    && rm -rf /var/lib/apt/lists/*

# Add Windows target
RUN rustup target add x86_64-pc-windows-gnu

# Set environment variables for cross-compilation
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
ENV CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
ENV CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++

WORKDIR /app
EOF
    
    # Build the Docker image
    echo -e "${YELLOW}Building Docker image...${NC}"
    $DOCKER_CMD build -t anttp-cross-compiler -f Dockerfile.cross .
    
    # Run the Docker container to build the project
    echo -e "${YELLOW}Building AntTP for Windows...${NC}"
    $DOCKER_CMD run --rm -v "$(pwd):/app" anttp-cross-compiler cargo build --release --target x86_64-pc-windows-gnu
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Cross-compilation with Docker completed successfully${NC}"
        echo -e "${GREEN}Binary location: $(pwd)/target/x86_64-pc-windows-gnu/release/anttp.exe${NC}"
        return 0
    else
        echo -e "${RED}Cross-compilation with Docker failed${NC}"
        return 1
    fi
}

# Function to cross-compile directly using MinGW
cross_compile_direct() {
    echo -e "${YELLOW}Cross-compiling directly with MinGW...${NC}"
    
    # Set up environment variables for cross-compilation
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
    export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
    export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
    
    # Make sure the Windows target is installed
    rustup target add x86_64-pc-windows-gnu
    
    # Clean the target directory to ensure a fresh build
    echo -e "${YELLOW}Cleaning previous build artifacts...${NC}"
    cargo clean --target x86_64-pc-windows-gnu
    
    # Build the project
    echo -e "${YELLOW}Building for Windows...${NC}"
    cargo build --release --target x86_64-pc-windows-gnu
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Cross-compilation with MinGW completed successfully${NC}"
        echo -e "${GREEN}Binary location: $(pwd)/target/x86_64-pc-windows-gnu/release/anttp.exe${NC}"
        return 0
    else
        echo -e "${RED}Cross-compilation with MinGW failed${NC}"
        return 1
    fi
}

# Function to install cross tool
install_cross() {
    echo -e "${YELLOW}Installing 'cross' tool...${NC}"
    cargo install cross
    echo -e "${GREEN}'cross' tool installed successfully${NC}"
}

# Function to cross-compile using cross tool
cross_compile_with_cross() {
    echo -e "${YELLOW}Cross-compiling with 'cross' tool...${NC}"
    
    # Use cross to build for Windows
    cross build --release --target x86_64-pc-windows-gnu
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Cross-compilation with 'cross' tool completed successfully${NC}"
        echo -e "${GREEN}Binary location: $(pwd)/target/x86_64-pc-windows-gnu/release/anttp.exe${NC}"
        return 0
    else
        echo -e "${RED}Cross-compilation with 'cross' tool failed${NC}"
        return 1
    fi
}

# Main script execution starts here

# First, fix the crunchy crate
fix_crunchy_crate

# Check if rustup is installed
if ! command_exists rustup; then
    echo -e "${RED}Rustup is not installed. Please install Rust first:${NC}"
    echo -e "${YELLOW}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    exit 1
fi

# Check if the Windows target is installed
if ! rustup target list | grep -q "x86_64-pc-windows-gnu"; then
    echo -e "${YELLOW}Adding Windows target...${NC}"
    rustup target add x86_64-pc-windows-gnu
fi

# Determine the best cross-compilation method
echo -e "${BLUE}Determining the best cross-compilation method...${NC}"

# Check if Docker is installed
if command_exists docker; then
    echo -e "${GREEN}Docker is installed${NC}"
    DOCKER_CMD="docker"
    DOCKER_AVAILABLE=true
else
    echo -e "${YELLOW}Docker is not installed${NC}"
    DOCKER_AVAILABLE=false
    
    # Ask if the user wants to install Docker
    read -p "Do you want to install Docker? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        install_docker
        DOCKER_CMD="sudo docker"
        DOCKER_AVAILABLE=true
    fi
fi

# Check if MinGW is installed
if command_exists x86_64-w64-mingw32-gcc; then
    echo -e "${GREEN}MinGW is installed${NC}"
    MINGW_AVAILABLE=true
else
    echo -e "${YELLOW}MinGW is not installed${NC}"
    MINGW_AVAILABLE=false
    
    # Ask if the user wants to install MinGW
    read -p "Do you want to install MinGW? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        install_mingw
        MINGW_AVAILABLE=true
    fi
fi

# Check if cross is installed
if command_exists cross; then
    echo -e "${GREEN}'cross' tool is installed${NC}"
    CROSS_AVAILABLE=true
else
    echo -e "${YELLOW}'cross' tool is not installed${NC}"
    CROSS_AVAILABLE=false
    
    # Ask if the user wants to install cross
    read -p "Do you want to install the 'cross' tool? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        install_cross
        CROSS_AVAILABLE=true
    fi
fi

# Determine which method to use
echo -e "${BLUE}Available cross-compilation methods:${NC}"
METHODS=()

if [ "$DOCKER_AVAILABLE" = true ]; then
    METHODS+=("Docker")
    echo -e "${GREEN}1. Docker${NC}"
fi

if [ "$MINGW_AVAILABLE" = true ]; then
    METHODS+=("MinGW")
    echo -e "${GREEN}2. MinGW (direct)${NC}"
fi

if [ "$CROSS_AVAILABLE" = true ]; then
    METHODS+=("Cross")
    echo -e "${GREEN}3. 'cross' tool${NC}"
fi

if [ ${#METHODS[@]} -eq 0 ]; then
    echo -e "${RED}No cross-compilation methods available. Please install Docker, MinGW, or the 'cross' tool.${NC}"
    exit 1
fi

# Ask the user which method to use
echo -e "${BLUE}Which method would you like to use?${NC}"
select METHOD in "${METHODS[@]}"; do
    if [ -n "$METHOD" ]; then
        break
    fi
    echo -e "${RED}Invalid selection${NC}"
done

# Execute the selected method
case "$METHOD" in
    "Docker")
        cross_compile_with_docker
        ;;
    "MinGW")
        cross_compile_direct
        ;;
    "Cross")
        cross_compile_with_cross
        ;;
    *)
        echo -e "${RED}Invalid method selected${NC}"
        exit 1
        ;;
esac

# Check if the compilation was successful
if [ $? -eq 0 ]; then
    echo -e "${GREEN}Cross-compilation completed successfully!${NC}"
    echo -e "${GREEN}Binary location: $(pwd)/target/x86_64-pc-windows-gnu/release/anttp.exe${NC}"
    exit 0
else
    echo -e "${RED}Cross-compilation failed${NC}"
    exit 1
fi 