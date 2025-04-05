# glitchy-server-client

A simple client-server application for downloading files over HTTP.

## Project Structure

- **server.py** – A basic HTTP server written in Python, capable of serving files with support for partial content requests (Given)
- **client/** – Rust-based HTTP client that downloads files from the server, verifies data integrity using SHA-256, and supports automatic retries upon network errors.

## Features

- Partial content downloading using HTTP Range headers.
- SHA-256 integrity checks to ensure downloaded file correctness.
- Custom implementation of Hex encoding without external dependencies.
- Command-line argument parsing for convenient client configuration.

## Core Logic (Client)

The client repeatedly sends HTTP requests to the server, requesting specific byte ranges until the entire file is successfully downloaded. It handles network errors gracefully, retrying failed requests automatically. Once the download is complete, the client calculates a SHA-256 hash of the data to ensure its integrity.

## Rust Client Files Explained

- **main.rs** – Entry point for the application, manages high-level logic.
- **http_client.rs** – Contains the HTTP client implementation responsible for fetching file chunks.
- **args.rs** – Handles command-line argument parsing and configuration.
- **config.rs** – Defines configuration defaults and structures.
- **hex.rs** – Custom hex encoding implementation, removing the need for external libraries.

## Running the Project

### Server (Python)
```sh
python server.py
```

### Client (Rust)
```sh
cd client

# tests
cargo test

# run
cargo run -- --hash=<SHA256_HASH> [--host=<HOST>] [--port=<PORT>]
```

## Author's Notes

This was my first experience writing code in Rust. I intentionally kept things straightforward, avoiding unnecessary complexity to express myself clearly through code. I made sure to cover essential functionality with tests. Overall, I enjoyed working with Rust and look forward to diving deeper into it.
