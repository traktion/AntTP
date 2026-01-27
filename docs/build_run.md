# Build & Run Instructions

## Build Instructions

AntTP is written in Rust. Ensure you have the Rust toolchain installed.

### Dependencies

On Ubuntu/Debian:
```bash
sudo apt-get update
sudo apt-get install rustup build-essential pkg-config libssl-dev protoc-gen-rust
rustup default stable
```

The project requires `protoc` (Protocol Buffers compiler) for gRPC support.

### Compilation

To build for your current host architecture:
```bash
cargo build --release
```
The binary will be located at `target/release/anttp`.

### Cross-Compilation Targets

#### Linux (MUSL)
Recommended for distributing binaries with minimal runtime dependencies.
```bash
sudo apt-get install musl-tools
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

#### Windows (GNU)
```bash
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

#### ARM (AARCH64 MUSL)
```bash
sudo apt install gcc-aarch64-linux-gnu binutils-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-musl
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc
export CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc
cargo build --release --target aarch64-unknown-linux-musl
```

---

## Run Instructions

### Help Command
To see all available options:
```bash
./anttp --help
```

### Basic Usage
Run with default settings:
```bash
./anttp
```

### Common Arguments
- `-l, --listen-address`: IP and port to listen on (default: `0.0.0.0:18888`).
- `-s, --static-file-directory`: Local directory for hosting static files.
- `-w, --wallet-private-key`: Hex-encoded secret key for the wallet used for network uploads.
- `-d, --download-threads`: Number of parallel threads for chunk downloads (default: 8).
- `-u, --uploads-disabled`: Disable all upload functionality.
- `-p, --peers`: List of multiaddresses for initial network peers.

For more details on configuration options, see [Configuration](configuration.md).
