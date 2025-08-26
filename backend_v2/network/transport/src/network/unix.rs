//! Unix Domain Socket Transport
//!
//! High-performance local IPC transport using Unix domain sockets for
//! ultra-low latency communication between processes on the same machine.

use crate::{Result, TransportError};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Unix socket transport for local IPC
pub struct UnixSocketTransport {
    config: UnixSocketConfig,
    listener: Option<UnixListener>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

/// Unix socket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnixSocketConfig {
    /// Socket path
    pub path: PathBuf,
    /// Buffer size for reading
    pub buffer_size: usize,
    /// Maximum message size
    pub max_message_size: usize,
    /// Clean up socket file on drop
    pub cleanup_on_drop: bool,
}

impl Default for UnixSocketConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("/tmp/alphapulse.sock"),
            buffer_size: 64 * 1024,             // 64KB
            max_message_size: 16 * 1024 * 1024, // 16MB
            cleanup_on_drop: true,
        }
    }
}

impl UnixSocketTransport {
    /// Create new Unix socket transport
    pub fn new(config: UnixSocketConfig) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Ok(Self {
            config,
            listener: None,
            shutdown_tx: Some(shutdown_tx),
            shutdown_rx: Some(shutdown_rx),
        })
    }

    /// Bind to Unix socket and start listening
    pub async fn bind(&mut self) -> Result<()> {
        // Remove existing socket file if it exists
        if self.config.path.exists() {
            std::fs::remove_file(&self.config.path).map_err(|e| {
                TransportError::network_with_source("Failed to remove existing socket", e)
            })?;
        }

        // Create parent directory if needed
        if let Some(parent) = self.config.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TransportError::network_with_source("Failed to create socket directory", e)
            })?;
        }

        // Bind to socket
        let listener = UnixListener::bind(&self.config.path)
            .map_err(|e| TransportError::network_with_source("Failed to bind Unix socket", e))?;

        info!("Unix socket listening on: {:?}", self.config.path);
        self.listener = Some(listener);

        Ok(())
    }

    /// Accept incoming connections
    pub async fn accept(&mut self) -> Result<UnixSocketConnection> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| TransportError::connection("Socket not bound", None))?;

        let (stream, _) = listener
            .accept()
            .await
            .map_err(|e| TransportError::network_with_source("Failed to accept connection", e))?;

        debug!("Accepted Unix socket connection");

        Ok(UnixSocketConnection::new(stream, self.config.clone()))
    }

    /// Connect to a Unix socket server
    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<UnixSocketConnection> {
        let stream = UnixStream::connect(path.as_ref()).await.map_err(|e| {
            TransportError::network_with_source("Failed to connect to Unix socket", e)
        })?;

        let config = UnixSocketConfig {
            path: path.as_ref().to_path_buf(),
            ..Default::default()
        };

        debug!("Connected to Unix socket: {:?}", path.as_ref());

        Ok(UnixSocketConnection::new(stream, config))
    }

    /// Shutdown the transport
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Clean up socket file
        if self.config.cleanup_on_drop && self.config.path.exists() {
            std::fs::remove_file(&self.config.path)
                .map_err(|e| TransportError::network_with_source("Failed to remove socket file", e))?;
        }

        info!("Unix socket transport shut down");
        Ok(())
    }
}

impl Drop for UnixSocketTransport {
    fn drop(&mut self) {
        if self.config.cleanup_on_drop && self.config.path.exists() {
            let _ = std::fs::remove_file(&self.config.path);
        }
    }
}

/// Unix socket connection
pub struct UnixSocketConnection {
    stream: UnixStream,
    config: UnixSocketConfig,
    read_buffer: BytesMut,
}

impl UnixSocketConnection {
    /// Create new connection from stream
    pub fn new(stream: UnixStream, config: UnixSocketConfig) -> Self {
        let buffer_size = config.buffer_size;
        Self {
            stream,
            config,
            read_buffer: BytesMut::with_capacity(buffer_size),
        }
    }

    /// Send data over the connection
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.config.max_message_size {
            return Err(TransportError::protocol(format!(
                "Message size {} exceeds maximum {}",
                data.len(),
                self.config.max_message_size
            )));
        }

        // Write message length prefix (4 bytes)
        let len_bytes = (data.len() as u32).to_be_bytes();
        self.stream
            .write_all(&len_bytes)
            .await
            .map_err(|e| TransportError::network_with_source("Failed to write length prefix", e))?;

        // Write message data
        self.stream
            .write_all(data)
            .await
            .map_err(|e| TransportError::network_with_source("Failed to write data", e))?;

        self.stream
            .flush()
            .await
            .map_err(|e| TransportError::network_with_source("Failed to flush", e))?;

        Ok(())
    }

    /// Receive data from the connection
    pub async fn receive(&mut self) -> Result<Bytes> {
        // Read message length prefix
        let mut len_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| TransportError::network_with_source("Failed to read length prefix", e))?;

        let message_len = u32::from_be_bytes(len_bytes) as usize;

        if message_len > self.config.max_message_size {
            return Err(TransportError::protocol(format!(
                "Message size {} exceeds maximum {}",
                message_len, self.config.max_message_size
            )));
        }

        // Ensure buffer has enough capacity
        if self.read_buffer.capacity() < message_len {
            self.read_buffer
                .reserve(message_len - self.read_buffer.capacity());
        }

        // Read message data
        self.read_buffer.resize(message_len, 0);
        self.stream
            .read_exact(&mut self.read_buffer)
            .await
            .map_err(|e| TransportError::network_with_source("Failed to read data", e))?;

        Ok(self.read_buffer.clone().freeze())
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<()> {
        self.stream
            .shutdown()
            .await
            .map_err(|e| TransportError::network_with_source("Failed to shutdown stream", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_unix_socket_transport() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        let config = UnixSocketConfig {
            path: socket_path.clone(),
            ..Default::default()
        };

        // Create and bind server
        let mut server = UnixSocketTransport::new(config.clone()).unwrap();
        server.bind().await.unwrap();

        // Connect client
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let mut client = UnixSocketTransport::connect(&socket_path).await.unwrap();
            client.send(b"Hello, server!").await.unwrap();
        });

        // Accept connection and receive message
        let mut conn = server.accept().await.unwrap();
        let data = conn.receive().await.unwrap();
        assert_eq!(&data[..], b"Hello, server!");

        // Send response
        conn.send(b"Hello, client!").await.unwrap();

        server.shutdown().await.unwrap();
    }
}
