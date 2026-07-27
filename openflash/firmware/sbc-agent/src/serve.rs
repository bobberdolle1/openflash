//! Accepting connections.
//!
//! Both listeners run forever, serving one client at a time: a flash chip has
//! one bus, so two hosts programming it concurrently would interleave commands
//! and corrupt each other's writes.

use std::net::TcpListener;
use std::path::Path;

use log::{error, info, warn};

use crate::agent::Agent;
use crate::bus::SpiBus;

/// Serve on a Unix socket at `path` until the process is stopped.
///
/// Returns only on a fatal error, having logged it.
pub fn listen_unix<B: SpiBus>(path: &str, agent: &mut Agent<B>) {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixListener;

        if Path::new(path).exists() {
            // A socket left behind by a previous run would make bind fail.
            if let Err(error) = std::fs::remove_file(path) {
                error!("Cannot remove the stale socket {path}: {error}");
                return;
            }
        }

        let listener = match UnixListener::bind(path) {
            Ok(listener) => listener,
            Err(error) => {
                error!("Cannot bind {path}: {error}");
                return;
            }
        };
        info!("Listening on {path}");

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    info!("Client connected");
                    agent.serve_client(stream);
                    info!("Client disconnected");
                }
                Err(error) => warn!("Connection failed: {error}"),
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (path, agent);
        error!("Unix sockets are not available on this platform");
    }
}

/// Serve on `address` (`host:port`) until the process is stopped.
pub fn listen_tcp<B: SpiBus>(address: &str, agent: &mut Agent<B>) {
    let listener = match TcpListener::bind(address) {
        Ok(listener) => listener,
        Err(error) => {
            error!("Cannot bind {address}: {error}");
            return;
        }
    };
    info!("Listening on {address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|address| address.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                info!("Client connected from {peer}");
                agent.serve_client(stream);
                info!("Client {peer} disconnected");
            }
            Err(error) => warn!("Connection failed: {error}"),
        }
    }
}
