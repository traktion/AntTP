### Project Overview
AntTP is an HTTP server for the Autonomi Network, built with Rust and the Actix-web framework. It acts as a gateway/proxy to retrieve and upload data (immutable and mutable) to the Autonomi Network.

### Build and Configuration

#### Standard Build
```bash
cargo build
```

#### Multi-Target Compilation
The project is frequently built for multiple targets. Ensure you have the necessary toolchains:

- **Linux (MUSL)**: Recommended for portability.
  ```bash
  rustup target add x86_64-unknown-linux-musl
  cargo build --release --target x86_64-unknown-linux-musl
  ```
- **Windows**:
  ```bash
  rustup target add x86_64-pc-windows-gnu
  cargo build --release --target x86_64-pc-windows-gnu
  ```
- **ARM (AARCH64)**:
  ```bash
  rustup target add aarch64-unknown-linux-musl
  export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc
  export CC=aarch64-linux-gnu-gcc
  cargo build --release --target aarch64-unknown-linux-musl
  ```

#### gRPC Support
The project uses `tonic` for gRPC. Ensure `protoc` is available in your environment as `build.rs` invokes `tonic-build`.

### Testing Guidelines

#### Unit Tests
Unit tests are located within the `src` directory, inlined in modules (using `#[cfg(test)]`).
- **Run all unit tests**: `cargo test`
- **Run a specific test**: `cargo test <test_name>`

**Adding a new test example**:
To add a test in a module (e.g., `src/service/my_service.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_feature() {
        // test logic here
        assert!(true);
    }
}
```

#### Integration & Performance Tests
Located in the `test/` directory:
- **Postman**: `test/postman` contains collections for Newman-based testing for HTTP endpoints.
- **Performance**: `test/performance` contains k6 scripts (`.js`) for load testing.

### Development Information

#### Code Style
- Follow standard Rust idioms.
- The project uses `actix-web` for the HTTP layer and `tonic` for gRPC.
- Caching is a critical component; see `src/client/caching_client.rs` and the use of `foyer`.
- Many controllers and services use the `Command` pattern for asynchronous operations and updates (see `src/client/command/`).

#### Key Architectural Components
- **CachingClient**: Wraps the Autonomi network client with a caching layer.
- **AccessChecker**: Manages allow/deny lists for addresses.
- **PointerNameResolver (PNR)**: Experimental service for human-readable names on Autonomi.
- **MCP API**: Allows AI agents to interact with the server as an MCP tool.

#### Debugging
- Use `RUST_LOG=debug` to see detailed logs from Actix and internal services.
- Swagger UI is available at `/swagger-ui/` when the server is running.
