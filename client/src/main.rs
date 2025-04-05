mod args;
mod config;
mod hex;
mod http_client;

use sha2::{Digest, Sha256};
use std::error::Error;
use std::io::{self, ErrorKind};
use std::process;

use crate::args::parse_args;
use crate::config::Config;
use crate::http_client::HttpClient;

fn download_file(config: &Config) -> Result<Vec<u8>, Box<dyn Error>> {
    let client = HttpClient::new(
        config.host.clone(),
        config.port,
        config.connect_timeout,
        config.read_write_timeout,
    );

    let mut data: Vec<u8> = Vec::new();
    let server_address = format!("{}:{}", config.host, config.port);

    println!(
        "Starting download from {} using std::net HttpClient...",
        server_address
    );

    loop {
        let start_byte = data.len();
        let range_header_info = format!("bytes={}-", start_byte);

        print!("Requesting range: {} -> ", range_header_info);
        match client.fetch_range(start_byte) {
            Ok((status, received_chunk)) => {
                println!(
                    "Status: {}, Received: {} bytes",
                    status,
                    received_chunk.len()
                );

                if status == 200 || status == 206 {
                    data.extend_from_slice(&received_chunk);

                    if status == 206 && received_chunk.is_empty() && start_byte > 0 {
                        println!("Received status 206 and 0 bytes for range starting at {}, assuming download complete.", start_byte);
                        return Ok(data);
                    }
                } else {
                    return Err(format!("Server returned non-successful status: {}", status).into());
                }
            }

            Err(e) => {
                let error_string = e.to_string();
                let io_error_kind = e.downcast_ref::<io::Error>().map(|io_err| io_err.kind());

                let is_retryable = match io_error_kind {
                    Some(ErrorKind::ConnectionRefused)
                    | Some(ErrorKind::TimedOut)
                    | Some(ErrorKind::ConnectionReset)
                    | Some(ErrorKind::ConnectionAborted)
                    | Some(ErrorKind::NotConnected)
                    | Some(ErrorKind::BrokenPipe) => true,
                    _ => {
                        error_string.contains("Failed to resolve address")
                            || error_string.contains("Connection closed before status line")
                            || error_string.contains("Connection closed during header reading")
                    }
                };

                if is_retryable {
                    eprintln!(
                        "\nNetwork/Connection Error: {}. Retrying range {}...",
                        e, range_header_info
                    );
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                } else {
                    return Err(format!("Fatal download error: {}", e).into());
                }
            }
        }
    }
}

fn main() {
    let config = match parse_args() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            eprintln!("Usage: --hash=<HASH> [--host=<HOST>] [--port=<PORT>] [--connect-timeout=<SECONDS>] [--read-write-timeout=<SECONDS>]");
            process::exit(1);
        }
    };

    match download_file(&config) {
        Ok(downloaded_data) => {
            println!("\n--------------------");
            println!("Download finished.");
            println!("Downloaded data length: {}", downloaded_data.len());

            let mut hasher = Sha256::new();
            hasher.update(&downloaded_data);
            let hash_result = hasher.finalize();
            let hash_hex = hex::encode(&hash_result);

            println!("Downloaded data SHA-256: {}", hash_hex);
            println!("Expected data SHA-256:   {}", config.expected_hash);
            println!("--------------------");

            if hash_hex == config.expected_hash {
                println!("Success: Data downloaded correctly! Hashes match.");
            } else {
                eprintln!("Failure: Data corruption detected! Hashes DO NOT match.");
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("\n--------------------");
            eprintln!("Failed to download the data: {}", e);
            eprintln!("--------------------");
            process::exit(1);
        }
    }
}
