//! Collaboration server

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{
    accept_async,
    tungstenite::Message,
};
use uuid::Uuid;

use crate::{
    crdt::ProjectCrdt,
    error::{Result, SyncError},
    lock::LockManager,
    presence::UserPresence,
    protocol::SyncMessage,
};
use futures_util::{SinkExt, StreamExt};

/// Shared state for the sync server
pub type SharedState = Arc<RwLock<ServerState>>;

/// Server state containing all active sessions
#[derive(Debug)]
pub struct ServerState {
    pub sessions: HashMap<String, ProjectSession>,
    pub client_sessions: HashMap<Uuid, String>, // client_id -> project_id
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            client_sessions: HashMap::new(),
        }
    }

    pub fn get_or_create_session(&mut self, project_id: &str) -> &mut ProjectSession {
        self.sessions
            .entry(project_id.to_string())
            .or_insert_with(|| ProjectSession::new(project_id.to_string()))
    }

    pub fn remove_client(&mut self, client_id: Uuid) {
        if let Some(project_id) = self.client_sessions.remove(&client_id) {
            if let Some(session) = self.sessions.get_mut(&project_id) {
                session.remove_client(client_id);
                // Clean up empty sessions
                if session.is_empty() {
                    self.sessions.remove(&project_id);
                }
            }
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Active project session
#[derive(Debug)]
pub struct ProjectSession {
    pub project_id: String,
    pub clients: HashMap<Uuid, ClientConnection>,
    pub crdt_state: ProjectCrdt,
    pub lock_manager: LockManager,
}

impl ProjectSession {
    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            clients: HashMap::new(),
            crdt_state: ProjectCrdt::new(0),
            lock_manager: LockManager::new(),
        }
    }

    pub fn add_client(&mut self, client: ClientConnection) {
        self.clients.insert(client.id, client);
    }

    pub fn remove_client(&mut self, client_id: Uuid) {
        // Release all locks held by this client
        self.lock_manager.release_all_client_locks(client_id);
        self.clients.remove(&client_id);
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    pub fn get_client(&self, client_id: Uuid) -> Option<&ClientConnection> {
        self.clients.get(&client_id)
    }

    pub fn get_client_mut(&mut self, client_id: Uuid) -> Option<&mut ClientConnection> {
        self.clients.get_mut(&client_id)
    }

    /// Broadcast a message to all clients except the sender
    pub async fn broadcast(&self, sender: Uuid, msg: SyncMessage) {
        let msg_text = serde_json::to_string(&msg).unwrap_or_default();
        
        for (client_id, client) in &self.clients {
            if *client_id != sender {
                let _ = client.tx.send(msg_text.clone());
            }
        }
    }

    /// Broadcast a message to all clients including the sender
    pub async fn broadcast_all(&self, msg: SyncMessage) {
        let msg_text = serde_json::to_string(&msg).unwrap_or_default();
        
        for client in self.clients.values() {
            let _ = client.tx.send(msg_text.clone());
        }
    }
}

/// Client connection information
#[derive(Debug)]
pub struct ClientConnection {
    pub id: Uuid,
    pub username: String,
    pub tx: mpsc::UnboundedSender<String>,
    pub presence: UserPresence,
}

/// Sync server for real-time collaboration
pub struct SyncServer {
    /// Shared server state
    state: SharedState,
    /// WebSocket listener
    listener: Option<TcpListener>,
    /// Server bind address
    bind_addr: SocketAddr,
}

impl SyncServer {
    /// Create a new sync server
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::new())),
            listener: None,
            bind_addr,
        }
    }

    /// Start the server
    pub async fn start(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        tracing::info!("Sync server listening on {}", self.bind_addr);
        
        self.listener = Some(listener);
        
        if let Some(listener) = &self.listener {
            loop {
                let (socket, addr) = listener.accept().await?;
                let state = self.state.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_client(socket, addr, state).await {
                        tracing::error!("Client handler error: {}", e);
                    }
                });
            }
        }
        
        Ok(())
    }

    /// Get server state for inspection
    pub fn state(&self) -> SharedState {
        self.state.clone()
    }

    /// Stop the server
    pub async fn stop(&mut self) {
        self.listener = None;
        tracing::info!("Sync server stopped");
    }
}

/// Handle a single client connection
async fn handle_client(
    socket: TcpStream,
    addr: SocketAddr,
    state: SharedState,
) -> Result<()> {
    tracing::info!("New connection from {}", addr);

    // WebSocket handshake
    let ws = accept_async(socket).await?;
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Channel for sending messages to the client
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Send task - handles outgoing messages
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Wait for Hello message
    let hello = match ws_rx.next().await {
        Some(Ok(Message::Text(text))) => {
            match serde_json::from_str::<SyncMessage>(&text) {
                Ok(SyncMessage::Hello { client_id, username, project_id }) => {
                    (client_id, username, project_id)
                }
                _ => {
                    return Err(SyncError::InvalidMessage(
                        "Expected Hello message".to_string(),
                    ));
                }
            }
        }
        _ => {
            return Err(SyncError::Connection(
                "Failed to receive Hello message".to_string(),
            ));
        }
    };

    let (client_id, username, project_id) = hello;
    tracing::info!(
        "Client {} ({}) joined project {}",
        username,
        client_id,
        project_id
    );

    // Create client connection
    let presence = UserPresence::new(client_id, username.clone());
    let client = ClientConnection {
        id: client_id,
        username: username.clone(),
        tx: tx.clone(),
        presence: presence.clone(),
    };

    // Add client to session
    let collaborators = {
        let mut state_guard = state.write().await;
        state_guard.client_sessions.insert(client_id, project_id.clone());
        
        let session = state_guard.get_or_create_session(&project_id);
        
        // Get existing collaborators
        let collaborators: Vec<UserPresence> = session
            .clients
            .values()
            .map(|c| c.presence.clone())
            .collect();
        
        session.add_client(client);
        collaborators
    };

    // Send Welcome message
    let session_id = Uuid::new_v4();
    let welcome = SyncMessage::Welcome {
        session_id,
        collaborators,
    };
    tx.send(serde_json::to_string(&welcome)?)?;

    // Broadcast join to other clients
    {
        let state_guard = state.read().await;
        if let Some(session) = state_guard.sessions.get(&project_id) {
            let join_msg = SyncMessage::ClientJoined { client: presence };
            session.broadcast(client_id, join_msg).await;
        }
    }

    // Message loop
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<SyncMessage>(&text) {
                    Ok(sync_msg) => {
                        handle_message(&state, client_id, &project_id, sync_msg, &tx).await;
                    }
                    Err(e) => {
                        tracing::warn!("Invalid message from {}: {}", client_id, e);
                        let error = SyncMessage::Error {
                            code: crate::protocol::ErrorCode::InvalidMessage,
                            message: format!("Invalid message: {}", e),
                        };
                        let _ = tx.send(serde_json::to_string(&error).unwrap_or_default());
                    }
                }
            }
            Ok(Message::Close(_)) => {
                tracing::info!("Client {} disconnected", client_id);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("WebSocket error from {}: {}", client_id, e);
                break;
            }
        }
    }

    // Cleanup
    send_task.abort();
    
    let mut state_guard = state.write().await;
    if let Some(session) = state_guard.sessions.get(&project_id) {
        let leave_msg = SyncMessage::ClientLeft { client_id };
        session.broadcast(client_id, leave_msg).await;
    }
    state_guard.remove_client(client_id);

    tracing::info!("Client {} disconnected from {}", client_id, addr);
    Ok(())
}

/// Handle a single message from a client
async fn handle_message(
    state: &SharedState,
    client_id: Uuid,
    project_id: &str,
    msg: SyncMessage,
    tx: &mpsc::UnboundedSender<String>,
) {
    let state_guard = state.read().await;
    
    let Some(session) = state_guard.sessions.get(project_id) else {
        return;
    };

    match msg {
        SyncMessage::Operation { op, timestamp, .. } => {
            // Apply to CRDT
            // In a real implementation, you'd deserialize the operation
            // and apply it to the appropriate CRDT
            
            // Broadcast to other clients
            let broadcast_msg = SyncMessage::Operation {
                client_id,
                timestamp,
                op,
            };
            session.broadcast(client_id, broadcast_msg).await;
        }

        SyncMessage::CursorMove { position, .. } => {
            // Update presence
            let mut state_guard = state.write().await;
            if let Some(session) = state_guard.sessions.get_mut(project_id) {
                if let Some(client) = session.get_client_mut(client_id) {
                    client.presence.cursor = position;
                }
            }
            drop(state_guard);

            // Broadcast to others
            let broadcast_msg = SyncMessage::CursorMove {
                client_id,
                position,
            };
            session.broadcast(client_id, broadcast_msg).await;
        }

        SyncMessage::SelectionChange { selected_entities, .. } => {
            // Update presence
            let mut state_guard = state.write().await;
            if let Some(session) = state_guard.sessions.get_mut(project_id) {
                if let Some(client) = session.get_client_mut(client_id) {
                    client.presence.selected_entities = selected_entities.clone();
                }
            }
            drop(state_guard);

            // Broadcast to others
            let broadcast_msg = SyncMessage::SelectionChange {
                client_id,
                selected_entities,
            };
            session.broadcast(client_id, broadcast_msg).await;
        }

        SyncMessage::LockEntity { entity_id, .. } => {
            let (lock_acquired, holder) = {
                let mut state_guard = state.write().await;
                let session = state_guard.sessions.get_mut(project_id).unwrap();
                
                let Some(client) = session.get_client(client_id) else {
                    return;
                };
                
                let acquired = session
                    .lock_manager
                    .try_lock(entity_id, client_id, &client.username);
                
                let holder = if !acquired {
                    session.lock_manager.get_lock_holder(entity_id)
                } else {
                    None
                };
                
                (acquired, holder)
            };
            
            if lock_acquired {
                // Lock granted
                let granted = SyncMessage::LockGranted { entity_id };
                let _ = tx.send(serde_json::to_string(&granted).unwrap_or_default());
                
                // Notify others
                let state_guard = state.read().await;
                if let Some(session) = state_guard.sessions.get(project_id) {
                    session
                        .broadcast(
                            client_id,
                            SyncMessage::LockEntity {
                                client_id,
                                entity_id,
                            },
                        )
                        .await;
                }
            } else {
                // Lock denied
                let denied = SyncMessage::LockDenied {
                    entity_id,
                    locked_by: holder.unwrap_or_default(),
                };
                let _ = tx.send(serde_json::to_string(&denied).unwrap_or_default());
            }
        }

        SyncMessage::UnlockEntity { entity_id, .. } => {
            let unlocked_success = {
                let mut state_guard = state.write().await;
                let session = state_guard.sessions.get_mut(project_id).unwrap();
                session.lock_manager.unlock(entity_id, client_id)
            };
            
            if unlocked_success {
                // Notify others
                let state_guard = state.read().await;
                if let Some(session) = state_guard.sessions.get(project_id) {
                    let unlocked = SyncMessage::EntityUnlocked {
                        entity_id,
                        unlocked_by: client_id,
                    };
                    session.broadcast_all(unlocked).await;
                }
            }
        }

        SyncMessage::ChatMessage { text, timestamp, .. } => {
            let state_guard = state.read().await;
            if let Some(session) = state_guard.sessions.get(project_id) {
                if let Some(client) = session.get_client(client_id) {
                    let chat = SyncMessage::ChatMessage {
                        client_id,
                        username: client.username.clone(),
                        text,
                        timestamp,
                    };
                    session.broadcast_all(chat).await;
                }
            }
        }

        SyncMessage::RequestSync => {
            // Send current project state
            let state_guard = state.read().await;
            if state_guard.sessions.get(project_id).is_some() {
                let sync_state = SyncMessage::SyncState {
                    state: crate::protocol::ProjectState {
                        project_id: project_id.to_string(),
                        entities: Vec::new(), // TODO: serialize from CRDT
                        tile_maps: Vec::new(),
                        timestamp: current_timestamp(),
                    },
                };
                let _ = tx.send(serde_json::to_string(&sync_state).unwrap_or_default());
            }
        }

        SyncMessage::Ping { timestamp } => {
            let pong = SyncMessage::Pong { timestamp };
            let _ = tx.send(serde_json::to_string(&pong).unwrap_or_default());
        }

        _ => {
            // Ignore other messages
        }
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
