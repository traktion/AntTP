# Roadmap

AntTP is actively developed, with many features planned to enhance its capabilities as a gateway to the Autonomi Network.

## Documentation
- [x] Basic README
- [x] Improved README
- [x] Refactor documentation into multiple documents
- [ ] Add detailed tutorials
- [ ] Link with IMIM as a sample project

## Core Features

### Files & Directories
- [x] File downloads from XOR addresses
- [x] File downloads from archives with friendly names
- [x] Download files directly from Tarchives
- [x] Directory listing in HTML and JSON
- [x] Multiple file uploads (multipart form data)
- [x] Default to `index.html` via route maps

### Caching & Performance
- [x] Cache immutable archive indexes to disk
- [x] Long-term caching headers for XOR data
- [x] ETag support for all immutable data
- [x] Streaming downloads (Range header support)

### Proxy Server
- [x] Resolve hostnames to XOR addresses (Files/Archives)
- [x] HTTPS proxy support

---

## API Integration

### REST API
- [x] Pointer, Register, Chunk, Public Archive
- [x] Async command/upload queue
- [x] Tarchive & PNR support
- [x] BLS support (encrypt, decrypt, sign, verify)
- [ ] Vault & Wallet management
- [ ] Data upload cost analysis

### gRPC API
- [x] Pointer, Register, Chunk, Public Archive
- [x] Tarchive & PNR support
- [x] BLS support (encrypt, decrypt, sign, verify)
- [ ] Vault & Wallet management
- [ ] Data upload cost analysis

### MCP API
- [x] Pointer, Register, Chunk, Public Archive
- [x] Tarchive & PNR support
- [x] BLS support (encrypt, decrypt, sign, verify)
- [ ] Vault & Wallet management
- [ ] Data upload cost analysis

---

## Advanced Features
- [ ] **Websockets:** Streaming immutable and mutable data changes.
- [ ] **Accounting:** Bandwidth usage tracking and payments for public proxies.
- [x] **Access List:** Allow/deny controls for addresses.
- [x] **Offline Mode:** Support for requests and uploads without an active network connection (with async sync).

---
[<< Previous](mcp.md) | [Up](../README.md)
