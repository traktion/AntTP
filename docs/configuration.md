# Configuration

AntTP can be configured using command-line arguments.

## Command-Line Arguments

| Argument | Description | Default Value |
|----------|-------------|---------------|
| `-l, --listen-address` | HTTP listen address and port. | `0.0.0.0:18888` |
| `--https-listen-address` | HTTPS listen address and port. | `0.0.0.0:18889` |
| `--grpc-listen-address` | gRPC listen address and port. | `0.0.0.0:18887` |
| `-s, --static-file-directory` | Local directory for hosting static files. | (empty) |
| `-w, --wallet-private-key` | Hex-encoded secret key for network uploads. | (empty) |
| `-d, --download-threads` | Parallel threads for chunk downloads. | `8` |
| `-a, --app-private-key` | Personal/App private key for mutable data. | (randomly generated if empty) |
| `-b, --bookmarks-address` | XOR address of the bookmarks archive. | (predefined address) |
| `-u, --uploads-disabled` | Disable all upload functionality. | `false` |
| `--mcp-tools-disabled` | Disable the MCP Tools API. | `false` |
| `--grpc-disabled` | Disable the gRPC API server. | `false` |
| `-c, --cached-mutable-ttl` | TTL in seconds for cached mutable data (pointers/registers). | `5` |
| `-p, --peers` | Multiaddresses for initial network peers (comma-separated). | (empty) |
| `-m, --map-cache-directory` | Directory for storing cache files. | `/tmp/anttp/cache/` (or OS temp dir) |
| `-e, --evm-network` | EVM network to use (e.g., `evm-arbitrum-one`). | `evm-arbitrum-one` |
| `--immutable-disk-cache-size` | Size of the immutable disk cache in MB. | `1024` |
| `--immutable-memory-cache-size` | Size of the immutable memory cache in slots. | `32` |
| `-i, --idle-disconnect` | Seconds of inactivity before disconnecting from Autonomi. | `30` |
| `--command-buffer-size` | Size of the async command buffer in slots. | `128` |
| `--access-list-address` | XOR address of the archive containing `access_list.json`. | (empty) |

---

## Browser Proxy Configuration

Using AntTP as an HTTP/SOCKS proxy is highly recommended for the best experience.

### Firefox

1.  Open **Settings** > Search for **Proxy** > **Settings...**.
2.  Select **Manual proxy configuration**.
3.  Set **HTTP Proxy** to `127.0.0.1` and **Port** to `18888`.
4.  Check **Also use this proxy for HTTPS**.
5.  Select **SOCKS v5** and check **Proxy DNS when using SOCKS v5**.
6.  Click **OK**.

### Brave / Chrome

Launch with proxy arguments:
```bash
brave --proxy-server="127.0.0.1:18888" http://[XOR_ADDRESS]/
```
