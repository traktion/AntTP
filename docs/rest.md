# REST API & Swagger UI

AntTP exposes a comprehensive REST API for interacting with the Autonomi Network. This API allows for the creation, retrieval, and management of various data types using standard HTTP methods.

## Swagger UI

The easiest way to explore and test the REST API is through the built-in Swagger UI. It provides an interactive documentation where you can see all available endpoints, their parameters, and even execute requests directly from your browser.

Access the Swagger UI at:
`http://localhost:18888/swagger-ui/`
*(Assuming AntTP is running locally on the default port)*

## Key Features

### Data Types
The REST API supports all primary Autonomi data types:
*   **Chunks:** `/chunk`
*   **Files:** `/file`
*   **Registers:** `/register`
*   **Pointers:** `/pointer`
*   **Archives:** `/archive`

### Custom Storage Header (`x-store-type`)
Developers can use the `x-store-type` header to control where data is stored:
*   `Network`: (Default) Data is uploaded to the Autonomi Network.
*   `Cache`: Data is stored only on the local AntTP instance. This is useful for testing or local-only data without incurring network costs.

### Async Operations
For large uploads or operations that may take time, AntTP provides an async command queue. You can monitor the status of these operations via the API.

## Uploading Data

To upload data via the REST API, ensure that:
1.  Uploads are enabled (default is enabled, unless `--uploads-disabled` is used).
2.  A valid wallet private key is provided via the `--wallet-private-key` argument.

The API supports various upload formats, including `multipart/form-data` for multiple files.
