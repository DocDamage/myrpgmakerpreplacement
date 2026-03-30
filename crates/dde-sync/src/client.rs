//! Collaboration client

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use dde_core::Entity;
use futures_util::{SinkExt, StreamExt};

use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::{
    error::{Result, SyncError},
    presence::{CursorPosition, UserPresence},
    protocol::{Operation, Rect, SyncMessage},
};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Collaboration client for connecting to a sync server
pub struct SyncClient {
    /// Client configuration
    config: ClientConfig,
    /// Server connection
    connection: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Local client ID
    client_id: Uuid,
    /// Current project session
    _session_id: Option<Uuid>,
    /// Channel for outgoing messages
    tx: Option<mpsc::UnboundedSender<SyncMessage>>,
    /// Collaborator presence
    collaborators: Arc<RwLock<HashMap<Uuid, UserPresence>>>,
    /// Local lock state
    locked_entities: Arc<RwLock<HashSet<Entity>>>,
    /// Pending operations queue
    pending_ops: Arc<Mutex<Vec<SyncMessage>>>,
    /// Message handler callback
    message_handler: Option<Arc<dyn Fn(SyncMessage) + Send + Sync>>,
    /// Background task handles
    _send_task: Option<tokio::task::JoinHandle<()>>,
    _receive_task: Option<tokio::task::JoinHandle<()>>,
    /// Connection error message
    last_error: Arc<RwLock<Option<String>>>,
}

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_url: String,
    pub username: String,
    pub project_id: String,
    pub auto_reconnect: bool,
    pub reconnect_delay_ms: u64,
}

impl ClientConfig {
    pub fn new(server_url: &str, username: &str, project_id: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
            username: username.to_string(),
            project_id: project_id.to_string(),
            auto_reconnect: true,
            reconnect_delay_ms: 5000,
        }
    }
}

impl SyncClient {
    /// Create a new sync client
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            connection: None,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            client_id: Uuid::new_v4(),
            _session_id: None,
            tx: None,
            collaborators: Arc::new(RwLock::new(HashMap::new())),
            locked_entities: Arc::new(RwLock::new(HashSet::new())),
            pending_ops: Arc::new(Mutex::new(Vec::new())),
            message_handler: None,
            _send_task: None,
            _receive_task: None,
            last_error: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the client ID
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }

    /// Get current connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Get connection state synchronously (may block)
    pub fn state_blocking(&self) -> ConnectionState {
        if let Ok(state) = self.state.try_read() {
            *state
        } else {
            ConnectionState::Connecting // Assume connecting if locked
        }
    }

    /// Get collaborators
    pub async fn collaborators(&self) -> HashMap<Uuid, UserPresence> {
        self.collaborators.read().await.clone()
    }

    /// Set message handler callback
    pub fn set_message_handler<F>(&mut self, handler: F)
    where
        F: Fn(SyncMessage) + Send + Sync + 'static,
    {
        self.message_handler = Some(Arc::new(handler));
    }

    /// Get the last error message
    pub async fn last_error(&self) -> Option<String> {
        self.last_error.read().await.clone()
    }

    /// Connect to collaboration server
    pub async fn connect(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        if *state == ConnectionState::Connecting || *state == ConnectionState::Connected {
            return Ok(());
        }
        *state = ConnectionState::Connecting;
        drop(state);

        tracing::info!("Connecting to sync server at {}", self.config.server_url);

        let (ws_stream, _) = connect_async(&self.config.server_url).await?;
        self.connection = Some(ws_stream);

        let (mut write, mut read) = self.connection.take().unwrap().split();

        // Channel for sending messages
        let (tx, mut rx) = mpsc::unbounded_channel::<SyncMessage>();
        self.tx = Some(tx.clone());

        // Store references for tasks
        let state = self.state.clone();
        let collaborators = self.collaborators.clone();
        let locked_entities = self.locked_entities.clone();
        let pending_ops = self.pending_ops.clone();
        let client_id = self.client_id;
        let message_handler = self.message_handler.clone();
        let last_error = self.last_error.clone();

        // Send Hello message
        let hello = SyncMessage::Hello {
            client_id: self.client_id,
            username: self.config.username.clone(),
            project_id: self.config.project_id.clone(),
        };
        let hello_text = serde_json::to_string(&hello)?;
        write.send(Message::Text(hello_text)).await?;

        // Spawn send task
        self._send_task = Some(tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let text = match serde_json::to_string(&msg) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };
                if write.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }));

        // Spawn receive task
        self._receive_task = Some(tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => match serde_json::from_str::<SyncMessage>(&text) {
                        Ok(sync_msg) => {
                            Self::handle_incoming(
                                client_id,
                                sync_msg,
                                &state,
                                &collaborators,
                                &locked_entities,
                                &pending_ops,
                                message_handler.clone(),
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse message: {}", e);
                        }
                    },
                    Ok(Message::Close(_)) => {
                        tracing::info!("Connection closed by server");
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        *last_error.write().await = Some(e.to_string());
                        break;
                    }
                }
            }

            // Connection lost
            *state.write().await = ConnectionState::Disconnected;
        }));

        *self.state.write().await = ConnectionState::Connected;
        tracing::info!("Connected to sync server");

        Ok(())
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) {
        // Abort background tasks
        if let Some(task) = self._send_task.take() {
            task.abort();
        }
        if let Some(task) = self._receive_task.take() {
            task.abort();
        }
        
        // Close WebSocket gracefully
        if let Some(mut conn) = self.connection.take() {
            let _ = conn.close(None).await;
        }
        
        self.tx = None;
        *self.state.write().await = ConnectionState::Disconnected;
        self.collaborators.write().await.clear();
        self.locked_entities.write().await.clear();
        tracing::info!("Disconnected from sync server");
    }

    /// Send operation to server
    pub async fn send_operation(&self, op: Operation) -> Result<()> {
        let msg = SyncMessage::Operation {
            client_id: self.client_id,
            timestamp: current_timestamp(),
            op,
        };
        self.send(msg).await
    }

    /// Try to lock entity for editing
    pub async fn lock_entity(&self, entity: Entity) -> Result<()> {
        let msg = SyncMessage::LockEntity {
            client_id: self.client_id,
            entity_id: entity,
        };
        self.send(msg).await
    }

    /// Unlock an entity
    pub async fn unlock_entity(&self, entity: Entity) -> Result<()> {
        let msg = SyncMessage::UnlockEntity {
            client_id: self.client_id,
            entity_id: entity,
        };
        self.send(msg).await
    }

    /// Update cursor position
    pub async fn move_cursor(&self, pos: CursorPosition) -> Result<()> {
        let msg = SyncMessage::CursorMove {
            client_id: self.client_id,
            position: pos,
        };
        self.send(msg).await
    }

    /// Update selection
    pub async fn set_selection(&self, entities: Vec<Entity>) -> Result<()> {
        let msg = SyncMessage::SelectionChange {
            client_id: self.client_id,
            selected_entities: entities,
        };
        self.send(msg).await
    }

    /// Update viewport
    pub async fn set_viewport(&self, rect: Rect) -> Result<()> {
        let msg = SyncMessage::ViewportChange {
            client_id: self.client_id,
            rect,
        };
        self.send(msg).await
    }

    /// Send chat message
    pub async fn send_chat(&self, text: &str) -> Result<()> {
        let msg = SyncMessage::ChatMessage {
            client_id: self.client_id,
            username: self.config.username.clone(),
            text: text.to_string(),
            timestamp: current_timestamp(),
        };
        self.send(msg).await
    }

    /// Send entity creation
    pub async fn create_entity(
        &self,
        entity: Entity,
        components: Vec<crate::protocol::ComponentData>,
    ) -> Result<()> {
        let msg = SyncMessage::EntityCreate {
            entity_id: entity,
            components,
        };
        self.send(msg).await
    }

    /// Send entity deletion
    pub async fn delete_entity(&self, entity: Entity) -> Result<()> {
        let msg = SyncMessage::EntityDelete { entity_id: entity };
        self.send(msg).await
    }

    /// Send component update
    pub async fn update_component(
        &self,
        entity: Entity,
        component: crate::protocol::ComponentData,
    ) -> Result<()> {
        let msg = SyncMessage::ComponentUpdate {
            entity_id: entity,
            component,
        };
        self.send(msg).await
    }

    /// Request full sync
    pub async fn request_sync(&self) -> Result<()> {
        self.send(SyncMessage::RequestSync).await
    }

    /// Send ping/heartbeat
    pub async fn ping(&self) -> Result<()> {
        let msg = SyncMessage::Ping {
            timestamp: current_timestamp(),
        };
        self.send(msg).await
    }

    /// Send a raw message
    async fn send(&self, msg: SyncMessage) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(msg).map_err(|_| SyncError::NotConnected)?;
            Ok(())
        } else {
            // Queue for later if not connected
            self.pending_ops.lock().await.push(msg);
            Err(SyncError::NotConnected)
        }
    }

    /// Handle incoming messages
    async fn handle_incoming(
        client_id: Uuid,
        msg: SyncMessage,
        _state: &Arc<RwLock<ConnectionState>>,
        collaborators: &Arc<RwLock<HashMap<Uuid, UserPresence>>>,
        locked_entities: &Arc<RwLock<HashSet<Entity>>>,
        pending_ops: &Arc<Mutex<Vec<SyncMessage>>>,
        handler: Option<Arc<dyn Fn(SyncMessage) + Send + Sync>>,
    ) {
        // Call user handler if set
        if let Some(h) = handler {
            h(msg.clone());
        }

        match msg {
            SyncMessage::Welcome {
                session_id,
                collaborators: collab_list,
            } => {
                tracing::info!("Joined session: {}", session_id);
                let mut collab_map = collaborators.write().await;
                for presence in collab_list {
                    collab_map.insert(presence.client_id, presence);
                }
            }

            SyncMessage::ClientJoined { client } => {
                tracing::info!("User joined: {}", client.username);
                collaborators.write().await.insert(client.client_id, client);
            }

            SyncMessage::ClientLeft { client_id: left_id } => {
                if let Some(client) = collaborators.write().await.remove(&left_id) {
                    tracing::info!("User left: {}", client.username);
                }
            }

            SyncMessage::Operation {
                client_id: sender,
                op,
                ..
            } => {
                if sender != client_id {
                    // Apply remote operation
                    tracing::debug!("Received operation from {}: {:?}", sender, op);
                }
            }

            SyncMessage::CursorMove {
                client_id: sender,
                position,
            } => {
                if sender != client_id {
                    if let Some(presence) = collaborators.write().await.get_mut(&sender) {
                        presence.cursor = position;
                    }
                }
            }

            SyncMessage::SelectionChange {
                client_id: sender,
                selected_entities,
            } => {
                if sender != client_id {
                    if let Some(presence) = collaborators.write().await.get_mut(&sender) {
                        presence.selected_entities = selected_entities;
                    }
                }
            }

            SyncMessage::LockGranted { entity_id } => {
                locked_entities.write().await.insert(entity_id);
                tracing::debug!("Lock granted for entity {:?}", entity_id);
            }

            SyncMessage::LockDenied {
                entity_id,
                locked_by,
            } => {
                tracing::warn!(
                    "Lock denied for entity {:?}, already locked by {}",
                    entity_id,
                    locked_by
                );
            }

            SyncMessage::EntityUnlocked { entity_id, .. } => {
                locked_entities.write().await.remove(&entity_id);
            }

            SyncMessage::SyncState {
                state: project_state,
            } => {
                tracing::info!(
                    "Received sync state for project {}",
                    project_state.project_id
                );
                
                // Clear pending operations after successful sync
                pending_ops.lock().await.clear();
            }

            SyncMessage::Error { code, message } => {
                tracing::error!("Server error: {:?} - {}", code, message);
            }

            SyncMessage::Pong { timestamp } => {
                let latency = current_timestamp() - timestamp;
                tracing::debug!("Ping latency: {}ms", latency);
            }

            _ => {}
        }
    }

    /// Check if an entity is locked by this client
    pub async fn is_entity_locked(&self, entity: Entity) -> bool {
        self.locked_entities.read().await.contains(&entity)
    }

    /// Get locked entities
    pub async fn locked_entities(&self) -> HashSet<Entity> {
        self.locked_entities.read().await.clone()
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Get pending operations count
    pub async fn pending_count(&self) -> usize {
        self.pending_ops.lock().await.len()
    }

    /// Flush pending operations (send them if now connected)
    pub async fn flush_pending(&self) -> Result<usize> {
        let mut pending = self.pending_ops.lock().await;
        let count = pending.len();
        
        if count > 0 && self.is_connected().await {
            for msg in pending.drain(..) {
                if let Some(tx) = &self.tx {
                    let _ = tx.send(msg);
                }
            }
        }
        
        Ok(count)
    }

    /// Get username
    pub fn username(&self) -> &str {
        &self.config.username
    }

    /// Get project ID
    pub fn project_id(&self) -> &str {
        &self.config.project_id
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("ws://test.com", "TestUser", "project1");
        assert_eq!(config.server_url, "ws://test.com");
        assert_eq!(config.username, "TestUser");
        assert_eq!(config.project_id, "project1");
        assert!(config.auto_reconnect);
    }

    #[test]
    fn test_client_creation() {
        let config = ClientConfig::new("ws://test.com", "TestUser", "project1");
        let client = SyncClient::new(config);
        
        // Client ID should be unique
        assert_ne!(client.client_id(), Uuid::nil());
    }

    #[test]
    fn test_connection_state() {
        let config = ClientConfig::new("ws://test.com", "TestUser", "project1");
        let client = SyncClient::new(config);
        
        // Initial state should be disconnected
        assert_eq!(client.state_blocking(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_client_getters() {
        let config = ClientConfig::new("ws://test.com", "TestUser", "project1");
        let client = SyncClient::new(config);
        
        assert_eq!(client.username(), "TestUser");
        assert_eq!(client.project_id(), "project1");
    }
}
