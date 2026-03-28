use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};

// 客户端类型
#[derive(Debug, Clone, PartialEq)]
enum ClientRole {
    Host,    // 被控端
    Control, // 控制端
}

// 客户端连接信息
struct Client {
    id: String,
    role: ClientRole,
    sender: mpsc::UnboundedSender<Message>,
    peer_id: Option<String>, // 正在连接的对方ID
}

// 房间/会话信息
struct Session {
    host_id: String,
    password: String,
    control_id: Option<String>,
}

// 全局状态
struct AppState {
    // 在线客户端: client_id -> Client
    clients: RwLock<HashMap<String, Client>>,
    // 会话: connection_code -> Session
    sessions: RwLock<HashMap<String, Session>>,
}

// 客户端发给服务器的消息格式
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientMessage {
    // 被控端请求注册，获取连接码和密码
    RegisterHost,
    // 控制端请求连接被控端
    Connect { code: String, password: String },
    // WebRTC SDP Offer/Answer
    Sdp { sdp: String, sdp_type: String },
    // WebRTC ICE Candidate
    IceCandidate { candidate: String, sdp_mid: Option<String>, sdp_m_line_index: Option<u16> },
}

// 服务器发给客户端的消息格式
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
enum ServerMessage {
    // 注册成功，返回连接码和密码
    Registered { code: String, password: String, client_id: String },
    // 连接结果
    ConnectResult { success: bool, message: String },
    // 通知被控端有控制端连入
    PeerConnected { peer_id: String },
    // 转发的 SDP
    Sdp { sdp: String, sdp_type: String },
    // 转发的 ICE Candidate
    IceCandidate { candidate: String, sdp_mid: Option<String>, sdp_m_line_index: Option<u16> },
    // 错误信息
    Error { message: String },
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        clients: RwLock::new(HashMap::new()),
        sessions: RwLock::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("Signaling server listening on ws://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // 生成唯一的客户端ID
    let client_id = uuid::Uuid::new_v4().to_string();
    
    // 启动一个任务将 mpsc 接收到的消息发送到 WebSocket
    let client_id_clone = client_id.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                println!("Client {} disconnected", client_id_clone);
                break;
            }
        }
    });

    // 默认作为未注册的客户端加入
    state.clients.write().await.insert(
        client_id.clone(),
        Client {
            id: client_id.clone(),
            role: ClientRole::Control, // 默认角色，后续可改
            sender: tx.clone(),
            peer_id: None,
        },
    );

    // 处理接收到的消息
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                handle_client_message(client_id.clone(), client_msg, state.clone(), tx.clone()).await;
            } else {
                let _ = tx.send(Message::Text(serde_json::to_string(&ServerMessage::Error {
                    message: "Invalid message format".to_string(),
                }).unwrap().into()));
            }
        }
    }

    // 客户端断开连接，清理资源
    cleanup_client(&client_id, &state).await;
}

async fn handle_client_message(
    client_id: String,
    msg: ClientMessage,
    state: Arc<AppState>,
    tx: mpsc::UnboundedSender<Message>,
) {
    match msg {
        ClientMessage::RegisterHost => {
            // 生成 9 位连接码和 6 位随机密码
            let (code, password) = {
                let mut rng = rand::rng();
                let code: String = (0..9).map(|_| rng.random_range(0..10).to_string()).collect();
                let password: String = (0..6).map(|_| rng.random_range(0..10).to_string()).collect();
                (code, password)
            };

            // 更新客户端角色
            if let Some(client) = state.clients.write().await.get_mut(&client_id) {
                client.role = ClientRole::Host;
            }

            // 创建会话
            state.sessions.write().await.insert(
                code.clone(),
                Session {
                    host_id: client_id.clone(),
                    password: password.clone(),
                    control_id: None,
                },
            );

            let response = ServerMessage::Registered { code, password, client_id };
            let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap().into()));
        }
        ClientMessage::Connect { code, password } => {
            let mut sessions = state.sessions.write().await;
            let mut clients = state.clients.write().await;

            if let Some(session) = sessions.get_mut(&code) {
                if session.password == password {
                    // 密码正确，建立连接
                    session.control_id = Some(client_id.clone());
                    
                    let host_id = session.host_id.clone();
                    
                    // 更新双方的 peer_id
                    if let Some(control_client) = clients.get_mut(&client_id) {
                        control_client.peer_id = Some(host_id.clone());
                        control_client.role = ClientRole::Control;
                    }
                    if let Some(host_client) = clients.get_mut(&host_id) {
                        host_client.peer_id = Some(client_id.clone());
                        
                        // 通知被控端有控制端连入
                        let notify = ServerMessage::PeerConnected { peer_id: client_id.clone() };
                        let _ = host_client.sender.send(Message::Text(serde_json::to_string(&notify).unwrap().into()));
                    }

                    let response = ServerMessage::ConnectResult {
                        success: true,
                        message: "Connected successfully".to_string(),
                    };
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap().into()));
                } else {
                    let response = ServerMessage::ConnectResult {
                        success: false,
                        message: "Invalid password".to_string(),
                    };
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap().into()));
                }
            } else {
                let response = ServerMessage::ConnectResult {
                    success: false,
                    message: "Invalid connection code".to_string(),
                };
                let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap().into()));
            }
        }
        ClientMessage::Sdp { sdp, sdp_type } => {
            forward_to_peer(&client_id, &state, ServerMessage::Sdp { sdp, sdp_type }).await;
        }
        ClientMessage::IceCandidate { candidate, sdp_mid, sdp_m_line_index } => {
            forward_to_peer(&client_id, &state, ServerMessage::IceCandidate { candidate, sdp_mid, sdp_m_line_index }).await;
        }
    }
}

async fn forward_to_peer(client_id: &str, state: &Arc<AppState>, msg: ServerMessage) {
    let clients = state.clients.read().await;
    if let Some(client) = clients.get(client_id) {
        if let Some(peer_id) = &client.peer_id {
            if let Some(peer) = clients.get(peer_id) {
                let _ = peer.sender.send(Message::Text(serde_json::to_string(&msg).unwrap().into()));
            }
        }
    }
}

async fn cleanup_client(client_id: &str, state: &Arc<AppState>) {
    let mut clients = state.clients.write().await;
    let mut sessions = state.sessions.write().await;

    if let Some(client) = clients.remove(client_id) {
        // 如果是被控端，删除对应的会话
        if client.role == ClientRole::Host {
            sessions.retain(|_, session| session.host_id != client_id);
        }
        
        // 通知对方断开连接 (这里可以扩展一个 PeerDisconnected 消息)
        if let Some(peer_id) = client.peer_id {
            if let Some(peer) = clients.get_mut(&peer_id) {
                peer.peer_id = None;
                let msg = ServerMessage::Error { message: "Peer disconnected".to_string() };
                let _ = peer.sender.send(Message::Text(serde_json::to_string(&msg).unwrap().into()));
            }
        }
    }
}
