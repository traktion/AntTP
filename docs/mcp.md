# MCP Tools API [EXPERIMENTAL]

AntTP provides an implementation of the **Model Context Protocol (MCP)**, allowing AI agents (LLMs) to interface directly with the Autonomi Network. Agents can use these tools to create, retrieve, and update both mutable and immutable data.

## Overview

The MCP API exposes the same underlying functionality as the REST and gRPC interfaces, including the caching layer. This allows AI agents to interact with the decentralized network with optimized performance.

## Configuration

To use AntTP as an MCP server, you need to point your MCP-enabled agent (such as Claude Code, Antigravity, or other MCP clients) to the AntTP MCP endpoint.

By default, the MCP endpoint is: `http://localhost:18888/mcp-0`

### Example: Antigravity Configuration
In your `mcp_servers.json` (or equivalent configuration):
```json
{
  "mcpServers": {
    "local-anttp": {
      "serverUrl": "http://localhost:18888/mcp-0",
      "headers": {
        "Authorization": "Bearer unknown",
        "Content-Type": "application/json"
      },
      "disabled": false
    }
  }
}
```

![Antigravity MCP Configuration](../resources/antigravity-mcp-servers.png)

## Available Tools

The MCP server provides tools for interacting with various Autonomi data types:
*   **Chunks:** Create and retrieve immutable data chunks.
*   **Files:** Upload and download files.
*   **Registers:** Manage mutable registers.
*   **Pointers:** Create and resolve pointers.
*   **Archives:** Work with public archives and tarchives.
*   **PNR:** Resolve human-readable names via the Pointer Name Resolver.

For more information on the Model Context Protocol, visit [modelcontextprotocol.io](https://modelcontextprotocol.io/).
