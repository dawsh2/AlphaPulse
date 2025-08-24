//! WebSocket client management

use crate::error::{DashboardError, Result};
use futures_util::SinkExt;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};
use uuid::Uuid;

/// WebSocket client representation
pub struct Client {
    pub id: Uuid,
    pub sender: mpsc::UnboundedSender<Value>,
}

impl Client {
    pub fn new(sender: mpsc::UnboundedSender<Value>) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
        }
    }

    /// Send a JSON message to this client
    pub fn send_message(&self, message: Value) -> Result<()> {
        self.sender
            .send(message)
            .map_err(|_| DashboardError::Client {
                message: "Failed to send message to client".to_string(),
            })
    }
}

/// Manages all connected WebSocket clients
pub struct ClientManager {
    clients: Arc<RwLock<HashMap<Uuid, Client>>>,
    max_connections: usize,
}

impl ClientManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
        }
    }

    /// Add a new client
    pub async fn add_client(&self, client: Client) -> Result<()> {
        let mut clients = self.clients.write().await;

        if clients.len() >= self.max_connections {
            return Err(DashboardError::Client {
                message: "Maximum connections reached".to_string(),
            });
        }

        let client_id = client.id;
        clients.insert(client_id, client);

        info!(
            "Added client {}, total connections: {}",
            client_id,
            clients.len()
        );
        Ok(())
    }

    /// Remove a client
    pub async fn remove_client(&self, client_id: Uuid) {
        let mut clients = self.clients.write().await;
        if clients.remove(&client_id).is_some() {
            info!(
                "Removed client {}, total connections: {}",
                client_id,
                clients.len()
            );
        }
    }

    /// Broadcast a message to all connected clients
    pub async fn broadcast(&self, message: Value) {
        let clients = self.clients.read().await;
        let mut failed_clients = Vec::new();

        for (client_id, client) in clients.iter() {
            if let Err(_) = client.send_message(message.clone()) {
                failed_clients.push(*client_id);
            }
        }

        drop(clients); // Release read lock before acquiring write lock

        // Remove failed clients
        if !failed_clients.is_empty() {
            let mut clients = self.clients.write().await;
            for client_id in failed_clients {
                clients.remove(&client_id);
                debug!("Removed disconnected client {}", client_id);
            }
        }
    }

    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_client_manager() {
        let manager = ClientManager::new(10);
        assert_eq!(manager.client_count().await, 0);

        let (tx, _rx) = mpsc::unbounded_channel();
        let client = Client::new(tx);
        let client_id = client.id;

        manager.add_client(client).await.unwrap();
        assert_eq!(manager.client_count().await, 1);

        manager.remove_client(client_id).await;
        assert_eq!(manager.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_client_broadcast() {
        let manager = ClientManager::new(10);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let client = Client::new(tx);

        manager.add_client(client).await.unwrap();

        let test_message = json!({"type": "test", "data": "hello"});
        manager.broadcast(test_message.clone()).await;

        // Should receive the broadcast message
        let received = rx.recv().await.unwrap();
        assert_eq!(received, test_message);
    }
}
