# gRPC API

AntTP provides a high-performance gRPC interface for interacting with the Autonomi Network. This interface is ideal for service-to-service communication and applications requiring low-latency data operations.

## Service Definition

The gRPC API is defined using Protocol Buffers. You can find the `.proto` files in the `proto/` directory of the repository.

By default, the gRPC server listens on `0.0.0.0:18887`. This can be configured using the `--grpc-listen-address` argument.

## Supported Operations

The gRPC API supports CRUD operations for all primary Autonomi data types:

*   **Pointers:** Create, retrieve, and update pointers.
*   **Registers:** Manage mutable registers.
*   **Chunks:** Upload and download data chunks.
*   **Files:** Handle file uploads and downloads.
*   **Archives:** Interact with public archives and tarchives.
*   **Pointer Name Resolver (PNR):** Resolve human-readable names to network addresses.

## Usage

To use the gRPC API, you can generate client libraries in your preferred language using the provided `.proto` files.

For Rust developers, AntTP uses `tonic` for its gRPC implementation.

### Disabling gRPC
If you do not require gRPC functionality, it can be disabled using the `--grpc-disabled` flag when starting AntTP.

### Public Archive & Tarchive
Allows creating and updating archives and tarchives. The `File` message includes:
- `name`: Filename.
- `content`: File bytes.
- `target_path`: (Optional) The relative path in the archive.
---
[<< Previous](rest.md) | [Up](../README.md) | [Next >>](mcp.md)
