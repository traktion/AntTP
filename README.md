# AntTP

## Background

Autonomi Network (a.k.a. Safe Network) is a distributed data network where both mutable and immutable data can be stored. It can
considered as a best of class web3 experience.

AntTP is a HTTP service which serves data from Autonomi over conventional HTTP connections. This allows regular
web browsers (and other apps) to retrieve data from Autonomi without needing any client libraries, CLIs, etc.

Users can either spin up a local AntTP service or deploy one to a public environment. This enables developers to
integrate with Autonomi in a more conventional way and gives end users a conventional browsing experience.

AntTP was formally known as sn_httpd.

## Features

`AntTP` currently provides the following:

- Data retrieval from Autonomic using archives for human readable naming `/[ARCHIVE_XOR_ADDRESS]/[MY_FILE_NAME]`. Enables
  regular static sites to be uploaded as an archive, with files browsed by file name. E.g.
- http://localhost:8080/91d16e58e9164bccd29a8fd8d25218a61d8253b51c26119791b2633ff4f6b309/autonomi/david-irvine-autonomi-founder.jpg
- Proxy server to allow `http://[ARCHIVE_XOR_ADDRESS]/[MY_FILE_NAME]` to be resolved. Allows
  sites to pivot from a 'root' directory and a smoother user experience. E.g.
- - http://91d16e58e9164bccd29a8fd8d25218a61d8253b51c26119791b2633ff4f6b309/autonomi/david-irvine-autonomi-founder.jpg
- Routing from URLs to specific `[XOR_ADDRESS]` or `[FILE_NAME]`. Enables SPA (single page apps) such as Angular or
  React to be hosted (once a routeMap is provided - see [example-config](app-conf.json)
- Native integration of the `autonomi` libraries into Actix web framework. These are both written in Rust to provide
  smooth integration. As Actix is core to `AntTP`, it can be extended for specific use cases easily. 
  
## Roadmap

- [ ] Documentation
  - [x] Basic README
  - [ ] Improved README
  - [ ] Add tutorials / API details
  - [ ] Link with IMIM as sample project
- [x] Files
  - [x] Enable file downloads from XOR addresses
  - [x] Enable file downloads from archives with friendly names
- [x] Directories (archives)
  - [x] Enable directory listing in HTML (default)
  - [x] Enable directory listing with JSON (using `accept` header)
  - [x] Enable multiple file uploads as multipart form data
    - Creates an archive, adds the files, then uploads to Autonomi
    - Async operation, with POST for data and GET for status checks
- [x] Caching
  - [x] Cache immutable archive indexes to disk to reduce lookups to Autonomi
  - [x] Set response headers to encourage long term caching of XOR data
  - [x] Add eTag header support to encourage long term caching of all immutable data (with/without XOR)
- [x] Proxy server
  - [x] Resolve hostnames to XOR addresses for files
  - [x] Resolve hostnames to XOR addresses for archives
- [x] Streaming downloads
  - [x] Add streaming support for data requested with RANGE headers
  - [x] Add streaming support for all other data requested
- [ ] Advanced Autonomi API integration
  - [ ] REST API
    - [ ] Pointer
    - [ ] Scratchpad
    - [ ] Graph
    - [ ] Register
    - [ ] Chunk
        - This is in addition to file support, which is already implemented
    - [ ] BLS support
      - [ ] Create, sign, verify
      - [ ] Derived keys
    - [ ] Analyze address support
    - [ ] Vault support (CRUD, cost)
    - [ ] Data upload cost
    - [ ] Wallet support
      - [ ] get balance
      - [ ] send tokens
      - [ ] get transaction history
  - [ ] gRPC API
    - [ ] Pointer
    - [ ] Scratchpad
    - [ ] Graph
    - [ ] Register
    - [ ] Chunk
      - This is in addition to file support, which is already implemented
    - [ ] BLS support
      - [ ] Create, sign, verify
      - [ ] Derived keys
    - [ ] Analyze address support
    - [ ] Vault support (CRUD, cost)
    - [ ] Data upload cost
    - [ ] Wallet support
      - [ ] get balance
      - [ ] send tokens
      - [ ] get transaction history
  - [ ] Websockets
    - [ ] Stream immutable data types
    - [ ] Stream changes to mutable data types
- [ ] Testing
  - [ ] Core unit test coverage
  - [ ] Full unit test coverage
  - [ ] Performance testing
- [ ] CLI and add config files
- [ ] Offline mode
  - Requests without connected client library dependency
- [ ] Accounting features
  - [ ] Bandwidth usage/tracking
  - [ ] Payments for data uploads (i.e. for public proxies)

- Built-in accounting features to allow hosts fund bandwidth usage via Autonomi Network Tokens. While Autonomi doesn't
  have any bandwidth usage fees, traffic too/from `AntTP` may be subject to charges by your hosting company. This
  will allow self-service for site authors to publish their site on your `AntTP` instance - the backend data is
  always on Autonomi, irrespective of where `AntTP` is hosted!
- Refactoring, performance, stability - `AntTP` is highly experimental and should only be used by the adventurous!
- Unit testing

## Build Instructions

### Dependencies
On Ubuntu:

Install Rust

`sudo apt-get install rustup`

Download latest stable release:

`rustup default stable`

### Linux Target

It is recommended that the MUSL target is used to prevent runtime dependency issues.

On Ubuntu:

`sudo apt-get install musl-tools`

Then add target:

`rustup target add x86_64-unknown-linux-musl`

Then build release:

`cargo build --release --target x86_64-unknown-linux-musl`

### Windows Target

On Ubuntu:

`sudo apt-get install mingw-w64`

Then add target:

`rustup target add x86_64-pc-windows-gnu`

Then build release:

`cargo build --release --target x86_64-pc-windows-gnu`

### ARM Target

On Ubuntu:

`sudo apt install gcc make gcc-arm* gcc-aarch64* binutils-arm* binutils-aarch64* pkg-config libssl-dev`

Then add target:

`rustup target add arm-unknown-linux-musleabi`
`rustup target add gcc-arm-linux-gnueabi`

Then update the environment:

`export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc`
`export CC=aarch64-linux-gnu-gcc`

Then build release:

`cargo build --release --target aarch64-unknown-linux-musl`

### Run instructions

From source code with defaults:

`cargo run`

From source code with arguments:

`cargo run 0.0.0.0:8080 static secret_key chunk_download_threads`

From binary with defaults

`anttp`

From binary with arguments:

`anttp 0.0.0.0:8080 static secret_key chunk_download_threads`

Where:

- `0.0.0.0:8080` (optional, default = `0.0.0.0:8080`) is the IP address and port to listen on.
- `static` (optional, default = `static`) is a directory to host local/static files in.
- `secret_key` (optional, default = ``) is a secret key for a wallet used for uploads.
- `chunk_download_threads` (optional, default = `32`) is the number of parallel threads used for chunk downloads.

### Archive Upload

To upload a directory to Autonomi as an archive, do the following:

- `cd your/directory`
- `ant file upload -p -x <directory>`

This command will return information about the uploads and summarise with something like:

`Uploading file: "./1_bYTCL7G4KbcR_Y4rd78OhA.png"
Upload completed in 5.57326318s
Successfully uploaded: ./
At address: 387f61da64d2a4c5d2e02ca34660fa2ac4fa6b3604ed8b67a58a3cba6e8ae787`

The 'At address' is the archive address, which you can now reference the uploaded files like:

Via a proxy (to localhost:8080):
`http://a0f6fa2b08e868060fe6e57018e3f73294821feaf3fdcf9cd636ac3d11e7e2ac/BegBlag.mp3` 

Or via direct request:
`http://localhost:8080/a0f6fa2b08e868060fe6e57018e3f73294821feaf3fdcf9cd636ac3d11e7e2ac/BegBlag.mp3`

### App Configuration

See [example-config](app-conf.json) for customising how your website/app behaves on `AntTP`:

```
{
  "routeMap": {
    "": "index.html",
    "blog/*": "index.html",
    "blog/*/article/*": "index.html"
  }
}
```

- Create an app-config.json file in the directory you intend to upload/publish to Autonomi
- Add the `routeMap` key
- Add any routes that should be mapped to a file
  - Use "" as a key to serve the target file for the root URL, e.g. index.html
  - Use "/blog/*" as a key to serve the target file for any URL with blog followed by a filename
  - Use "/blog/*/article/*" as a key to serve the target file for any URL with a blog and article specified
  - The blog/article above are not keywords. Any names can be used to suit the routing approach needed
- Upload the directory as an archive to Autonomi (see above for more details)
  - `ant file upload -p -x <directory>`
- Browse to the archive XOR address with your browser and confirm the routing is correct
- Why add routes?
  - Many modern frameworks expect all requests to be routed through a single HTML file, which then pull in Javascript
    dependencies, which then handles the routing of your app components. Angular, for example, requires this sort of routing.
  - If you just want an index instead of a file listing being rendered, providing a `routeMap` will also enable this. 
    This is handy when you want a default page/app/script to load for a URL, without needing to specify the filename too.

### Example site - IMIM!

A sister application for AntTP is the IMIM blog. The source code is located at [IMIM](https://github.com/traktion/i-am-immutable-client), and enabled authors 
to write Markup text files and publish them on Autonomi. Using `AntTP`, these blogs can be viewed anywhere that an
instance is running.

IMIM includes examples of route maps and how Angular apps can be integrated with AntTP. It also gives a realistic example
of performance and how immutable file caching can effectively reduce latency to near zero in most cases. IMIM is also
a great place to create a blog.

Why not take a look and start your own immutable blog today?
