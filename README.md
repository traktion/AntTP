# AntTP

AntTP is an HTTP gateway and proxy for the [Autonomi Network](https://autonomi.com/). It allows users to browse decentralized data using traditional web browsers and enables developers to integrate with Autonomi via familiar REST, gRPC, and MCP interfaces.

## Quick Links

*   **[Background & Overview](docs/background.md)** - What is AntTP and how it works.
*   **[Build & Run](docs/build_run.md)** - How to compile and start the server.
*   **[Configuration](docs/configuration.md)** - CLI arguments and browser proxy setup.
*   **[Roadmap](docs/roadmap.md)** - Current status and future plans.

## User Guides

*   **[Archives & Tarchives](docs/archive.md)** - Hosting collections of files.
*   **[Web App Customisation](docs/web_app.md)** - Routing for SPAs (Angular, React, etc.).
*   **[Publish Your Website](docs/publish_website.md)** - How to get your site onto Autonomi.
*   **[Pointer Name Resolver (PNR)](docs/pnr.md)** - Human-readable names on Autonomi.

## Developer Resources

*   **[REST API & Swagger UI](docs/rest.md)** - Interactive API documentation.
*   **[gRPC API](docs/grpc.md)** - High-performance service interface.
*   **[MCP Tools API](docs/mcp.md)** - Interface for AI agents.
*   **[Data Types](docs/data_types/)** - Detailed technical documentation for network data types.
    *   [Chunks](docs/data_types/chunks.md)
    *   [Files](docs/data_types/files.md)
    *   [Registers](docs/data_types/registers.md)
    *   [Pointers](docs/data_types/pointers.md)
    *   [Archives](docs/data_types/archives.md)
    *   [PNR](docs/data_types/pnr.md)

## Testing

Documentation for testing can be found in the `test/` directory:
*   [Performance Testing](test/performance/README.md) - k6 scripts for load testing.
*   [Integration Testing](test/postman/README.md) - Postman/Newman collections for API verification.

## Contributing

We welcome contributions! Please see the [Roadmap](docs/roadmap.md) for areas where you can help.

---
*AntTP was formerly known as `sn_httpd`.*
