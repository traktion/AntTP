# AntTP Postman Collection

This directory contains the Postman collection for testing the AntTP API.

## Contents

The collection `anttp_postman_collection.json` provides a comprehensive set of REST API requests for the following services:

- **Chunk**: Create and retrieve data chunks (JSON and Binary).
- **Register**: Create, update, and retrieve registers and their history.
- **Pointer**: Create, update, and retrieve pointers to chunks or other pointers.
- **Private Scratchpad**: Create, update, and retrieve private (encrypted) scratchpads.
- **Public Scratchpad**: Create, update, and retrieve public scratchpads.
- **Graph**: Create graph entries.
- **Public Archive**: Create and update public archives using multipart/form-data.
- **Public Data**: Create and retrieve public data (binary).
- **Tarchive**: Create and update tarchives (TAR-based archives).
- **Command**: Retrieve the list of background commands.
- **PNR**: Create PNR (Pointer Name Record) zones.

## Prerequisites

To run this collection, you need to have one of the following installed:

- **Postman**: For manual testing and exploration via the GUI.
- **Newman**: For running the collection from the command line.

To install Newman, use npm:

```bash
npm install -g newman
```

## Running the Collection

### With Newman

You can run the collection using the following command from the project root:

```bash
newman run test/postman/anttp_postman_collection.json --env-var base_url=http://localhost:18888
```

The `--env-var base_url` flag allows you to point the requests to your running AntTP instance. The default in the collection is `http://localhost:18888`.

### Environment Variables

The collection uses several variables to chain requests (e.g., capturing the address from a `POST` response and using it in a subsequent `GET` or `PUT`). 

When running with Newman, these are handled automatically in the temporary environment created during the run. In Postman, ensure you have an environment selected or that the variables are initialized.

Key variables used:
- `base_url`: The root URL of the AntTP service.
- `chunk_address`
- `register_address`
- `pointer_address`
- `private_scratchpad_address`
- `public_scratchpad_address`
- `public_data_address`

## Store Type

By default, the requests are configured with `x-cache-only: memory` header and `store_type: memory` in the body (where applicable) to avoid unnecessary persistence during testing.
