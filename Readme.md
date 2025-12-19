# P2P Recipe Sharing App

A peer-to-peer distributed recipe-sharing system built in Rust. Nodes
can store, replicate, and retrieve recipes over a lightweight TCP-based
P2P protocol. The project includes full Docker support, multi-node
Compose testing, and a clean multi-stage build.

------------------------------------------------------------------------

## Project Goals

-   Build a minimal but functional P2P protocol in Rust
-   Support recipe storage, retrieval, discovery, and propagation
-   Run multiple nodes locally with Docker Compose
-   Use a production-ready Dockerfile with efficient build caching
-   Provide a scalable architecture suitable for future features

------------------------------------------------------------------------

# Project Plan

## 1. MVP

### **1.1 Core Features**

-   TCP peer listener
-   Connect to initial peers
-   Broadcast new recipes
-   Store recipes in a JSON file
-   Respond to GET/SET requests

### **1.2 Rust Components**

-   `p2p/mod.rs` → message protocol (PING, STORE, GET, RECIPE)
-   `storage.rs` → JSON file read/write
-   `peer.rs` → manage peer list
-   `server.rs` → TCP listener
-   `client.rs` → outgoing messages

------------------------------------------------------------------------

## 2. Multi-Stage Dockerfile

### Requirements:

-   Cache dependencies correctly
-   Avoid missing `src/main.rs` during `cargo fetch`
-   Minimize final image size

### Output:

-   Multi-stage **builder** (Rust)
-   **runtime** (Debian slim) layer containing only the binary
-   Includes SSL + CA certificates

------------------------------------------------------------------------

## 3. Build and Test With Docker Compose

### Compose Goals:

-   Run 3+ nodes
-   Each node has its own data folder
-   Nodes know each other using environment variables
-   Expose ports but run isolated networks

### Example:

    docker compose up --build

Each container will run a P2P node that joins the network automatically.

------------------------------------------------------------------------

# Running the Project

## 1. Build Docker image

    docker build -t p2p-recipe .

## 2. Run a single node

    docker run -p 4001:4001 p2p-recipe

## 3. Run multi-node network

    docker compose up --build

## 4. Check logs

    docker compose logs -f peer1

------------------------------------------------------------------------

# Environment Variables

  Variable              Description
  --------------------- ----------------------------------------
  `P2P_PORT`            Port to listen on
  `STORAGE_FILE_PATH`   Path to recipes.json file
  `PEERS`               Comma-separated list of peer addresses

------------------------------------------------------------------------

# Testing the P2P Network

With Compose running:

### Store a recipe:

    echo '{"title":"Soup"}'   | nc localhost 4001

### Retrieve a recipe:

    echo 'GET Soup' | nc localhost 4002

### Check replication:

    docker exec peer3 cat /data/recipes.json
