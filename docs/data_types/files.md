# Files

Files are high-level abstractions over chunks on the Autonomi Network. They allow for the retrieval of data using XOR addresses or through archives with human-readable names.

## Data Flow

### Retrieve File by XOR Address
When a file is requested directly by its XOR address, AntTP downloads the underlying chunks and streams the data back to the client.

```mermaid
sequenceDiagram
    participant Client
    participant AntTP
    participant Cache
    participant Autonomi

    Client->>AntTP: GET /[XOR_ADDRESS]
    AntTP->>Cache: Check File Cache
    alt Cache Hit
        Cache-->>AntTP: File Data
    else Cache Miss
        AntTP->>Autonomi: Download Chunks
        Autonomi-->>AntTP: Chunks
        AntTP->>Cache: Store File in Cache
    end
    AntTP-->>Client: Stream File Data
```

### Retrieve File from Archive
When a file is requested from an archive, AntTP first resolves the archive index to find the XOR address of the requested file.

```mermaid
sequenceDiagram
    participant Client
    participant AntTP
    participant Cache
    participant Autonomi

    Client->>AntTP: GET /[ARCHIVE_ADDRESS]/[FILENAME]
    AntTP->>Cache: Get Archive Index
    alt Index Not Cached
        AntTP->>Autonomi: Retrieve Archive Index
        Autonomi-->>AntTP: Index Data
        AntTP->>Cache: Cache Index
    end
    AntTP->>AntTP: Lookup XOR for FILENAME
    AntTP->>Cache: Check File Cache (XOR)
    alt XOR Cache Miss
        AntTP->>Autonomi: Download Chunks
        Autonomi-->>AntTP: Chunks
        AntTP->>Cache: Store File
    end
    AntTP-->>Client: Stream File Data
```

## API Endpoints

### REST API
*   `GET /[XOR_ADDRESS]`: Retrieve a file directly.
*   `GET /[ARCHIVE_ADDRESS]/[FILENAME]`: Retrieve a file from an archive.
*   `GET /[POINTER_ADDRESS]/[FILENAME]`: Resolve pointer and retrieve file.

### MCP Tools
*   `upload_file`: Uploads a file to the network.
*   `download_file`: Downloads a file by its address.

### gRPC API
*   `PutFile`: Uploads a file.
*   `GetFile`: Retrieves a file.

## Features
*   **Streaming:** Large files are streamed to the client to reduce memory usage.
*   **Range Requests:** Supports standard HTTP `Range` headers for partial downloads (useful for video/audio seeking).
*   **Content-Type Detection:** Automatically detects and sets the `Content-Type` header based on the file extension or content.
