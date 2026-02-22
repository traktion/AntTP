### Project Overview
AntTP is an HTTP server for the Autonomi Network, built with Rust and the Actix-web framework. It acts as a gateway/proxy to retrieve and upload data (immutable and mutable) to the Autonomi Network.

### Build and Configuration

```bash
cargo build
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

- If a mock must be created, then define it with 'mock!' macro, with '#[derive(Debug)]', and do not attempt to use '#[automock]' on a struct impl.
- If a mock must be used, use `#[double]` macro to to include the mock or real implementation, depending on context (don't use elaborate #[cfg(test)] combinations).

#[double]

#### Integration & Performance Tests
Located in the `test/` directory:
- **Postman**: `test/postman` contains collections for Newman-based testing for REST HTTP endpoints.
- **Performance**: `test/performance` contains k6 scripts (`.js`) for load testing.

### Development Information

#### Code Style
- Follow standard Rust idioms.
- The project uses `actix-web` for the HTTP layer and `tonic` for gRPC.
- Caching is a critical component; see `src/client/caching_client.rs` and the use of `foyer`.
- Many controllers and services use the `Command` pattern for asynchronous operations and updates (see `src/client/command/`).
- REST API endpoints uses utoipa annotations in /src/controller/*_controller.rs files (and lib.rs) and use the URL prefix `/anttp-0/`
- MCP API endpoints in /src/tool/*_tool.rs files (and lib.rs)
- gRPC API endpoints in /src/grpc/*_handler.rs files (and lib.rs) and protobuffers in /proto/*.proto


#### Key Architectural Components
- **CachingClient**: Wraps the Autonomi network client with a caching layer.
- **AccessChecker**: Manages allow/deny lists for addresses.
- **PointerNameResolver (PNR)**: Experimental service for human-readable names on Autonomi.
- **MCP API**: Allows AI agents to interact with the server as an MCP tool.
- **REST API**: Provides a RESTful HTTP interface for interacting with the server, with controllers in *_controller.rs files.
- **gRPC API**: Provides a gRPC interface for interacting with the server using protobuffers.

#### Debugging
- Use `RUST_LOG=debug` to see detailed logs from Actix and internal services.
- Swagger UI is available at `/swagger-ui/` when the server is running.
