# Pointer Name Resolver (PNR)

The Pointer Name Resolver (PNR) provides human-readable name resolution on the Autonomi Network by utilizing pointer chains.

## Data Flow

### Register/Update Name Flow
Registering a name involves creating or updating a PNR zone (a specialized pointer) using the shared resolver key.

```mermaid
sequenceDiagram
    participant Client
    participant AntTP
    participant Autonomi

    Client->>AntTP: POST/PUT /anttp-0/pnr/{name}
    AntTP->>AntTP: Use Resolver Key
    AntTP->>Autonomi: Create/Update Pointer (Name)
    Autonomi-->>AntTP: Success
    AntTP-->>Client: PNR Zone Info
```

### Resolve Name Flow
When a name is encountered (e.g., in a browser URL), AntTP resolves the pointer chain to find the ultimate target XOR address.

```mermaid
sequenceDiagram
    participant Client
    participant AntTP
    participant Cache
    participant Autonomi

    Client->>AntTP: GET http://[NAME]/
    AntTP->>Cache: Check PNR Cache
    alt Cache Miss
        AntTP->>Autonomi: Follow Pointer Chain
        Autonomi-->>AntTP: Target XOR
        AntTP->>Cache: Cache Result
    end
    AntTP->>AntTP: Serve Archive/File at XOR
    AntTP-->>Client: Content
```

## API Endpoints

### REST API
*   `POST /anttp-0/pnr`: Create a new PNR name registration.
*   `PUT /anttp-0/pnr/{name}`: Update an existing PNR name.
*   `GET /anttp-0/pnr/{name}`: Retrieve the current registration info for a name.

### MCP Tools
*   `create_pnr`: Registers a new name.
*   `update_pnr`: Updates an existing name.
*   `get_pnr`: Retrieves info for a registered name.

### gRPC API
*   `CreatePnr`: Register or update a name.
*   `UpdatePnr`: Updates an existing name.
*   `GetPnr`: Retrieve name registration info.

## Ownership and Chains
PNR names can be transferred between owners. Each transfer adds a link to the pointer chain. AntTP's caching ensures that even long chains resolve quickly after the first lookup.
