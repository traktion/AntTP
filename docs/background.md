# Background & Overview

## Background

Autonomi Network (formerly known as the SAFE Network) is a distributed data network designed for privacy, security, and data permanence. It allows users to store both mutable and immutable data in a decentralized manner, providing a robust web3 experience.

AntTP acts as an HTTP gateway and proxy for the Autonomi Network. It enables traditional web browsers and applications to interact with the network without requiring specialized client libraries or command-line interfaces (CLIs). Users can run AntTP locally or deploy it to a public server, facilitating easy integration with Autonomi.

Originally known as `sn_httpd`, AntTP bridges the gap between the decentralized world of Autonomi and the conventional HTTP-based web.

## Overview

AntTP allows data on the Autonomi Network to be browsed directly via HTTP. It maps network addresses and archives to a URL structure that mimics a traditional file system.

### Core Concepts

*   **Archives:** These act as file containers on the network. They can be public or tarchives (tar-based archives). AntTP can retrieve files from these archives using their network address and the filename within them.
*   **Direct Addressing:** Files can also be accessed directly using their network (XOR) address. An optional friendly filename can be added to the URL to assist browsers in identifying the file type.
*   **Mutable Data (Pointers & Registers):** These allow for dynamic content. A pointer or register can be updated to point to new immutable data, enabling a consistent URL for changing content.
*   **Bookmarks:** Local aliases within an AntTP instance that point to specific network addresses, simplifying access to frequently used resources.

### URL Examples

*   **Archive:** `http://localhost:18888/[ARCHIVE_ADDRESS]/`
*   **File in Archive:** `http://localhost:18888/[ARCHIVE_ADDRESS]/my-image.png`
*   **Direct File:** `http://localhost:18888/[XOR_ADDRESS]/`
*   **Direct File with Name:** `http://localhost:18888/[XOR_ADDRESS]/song.mp3`
*   **Pointer/Register:** `http://localhost:18888/[POINTER_ADDRESS]/`
*   **Bookmark:** `http://localhost:18888/my-blog/`

### Key Features

*   **Data Retrieval:** Browse Autonomi data using human-readable names within archives.
*   **Proxy Support:** Resolve shorter URLs and improve security by routing traffic exclusively through AntTP to Autonomi.
*   **SPA Support:** Host Single Page Applications (e.g., React, Angular) with custom routing via a `routeMap`.
*   **Native Rust Integration:** Built on `actix-web` and the `autonomi` libraries for high performance and reliability.
*   **Caching:** Extensive caching of immutable and mutable data to minimize latency.
