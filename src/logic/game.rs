use crate::logic::room::Room;
use crate::utils::generate_room_id;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures::stream::StreamExt;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

lazy_static! {
    static ref ROOMS: Mutex<HashMap<String, Room>> = Mutex::new(HashMap::new());
    static ref ROOMS_SENDERS: Mutex<HashMap<String, broadcast::Sender<String>>> =
        Mutex::new(HashMap::new());
}

static PLAYER_NUM: usize = 2;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub user_id: String,   //玩家id
    pub user_name: String, //玩家name
    pub user_icon: String, //玩家头像，base64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Gconfig {
    pub cols: usize,  // 棋盘宽度
    pub rows: usize,  // 棋盘高度
    pub mines: usize, // 雷的总数
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InitParam {
    pub player: Player,
    pub config: Gconfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct JoinParam {
    pub room_id: String,
    pub player: Player,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ActionParam {
    pub x: usize,
    pub y: usize,
    pub is_flaged: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ClientMessage {
    InitParam { init_param: InitParam },
    JoinParam { join_param: JoinParam },
    ActionParam { action_param: ActionParam },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ServerMessage {
    JoinRoom {
        player: Player,
    },
    InitRoom {
        room_id: String,
    },
    GameStart {
        room: Room,
    },
    GameEnd {
        success: bool,
        scores: usize,
        duration: usize,
        steps: usize,
    },
    InvalidRequest {
        error: String,
    },
}

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let shared_socket = Arc::new(Mutex::new(socket));
    while let Some(msg) = {
        let mut socket_lock = shared_socket.lock().await;
        socket_lock.next().await
    } {
        match msg {
            Ok(Message::Close(_)) => {
                println!("玩家断开连接"); // 还需要清除全局变量的room_id,并广播信息给其他‘用户断开’，
                break;
            }
            Ok(message) => {
                let message = message.into_text().unwrap();
                handle_message(message, shared_socket.clone()).await
            }
            Err(e) => {
                println!("连接出错: {:?}", e);
                break;
            }
        }
    }
}

async fn handle_message(text: String, socket: Arc<Mutex<WebSocket>>) {
    let parsed_message: Result<ClientMessage, _> = serde_json::from_str(&text);
    match parsed_message {
        Ok(ClientMessage::InitParam { init_param }) => {
            // 创建房间，并加入到全局变量中
            init_room(socket, &init_param).await;
            println!("Received InitRoom message config: {:?}", init_param);
        }
        Ok(ClientMessage::JoinParam { join_param }) => {
            // 加入房间，并通知所有连接游戏开始
            join_room(socket, &join_param).await;
            println!("Received InitRoom message config: {:?}", join_param);
        }
        Ok(ClientMessage::ActionParam { action_param }) => {
            // 处理玩家操作，并通知所有连接
            handle_action(socket, &action_param).await;
            println!("Received ActionBody message with x: {:?}", action_param);
        }
        Err(e) => {
            println!("Error parsing message: {}", e);
        }
    }
}

async fn broadcast_action(
    mut receiver: broadcast::Receiver<String>,
    socket: Arc<Mutex<WebSocket>>,
) {
    let mut socket_lock = socket.lock().await;
    while let Ok(message) = receiver.recv().await {
        socket_lock.send(Message::Text(message)).await.unwrap();
    }
}

async fn handle_action(socket: Arc<Mutex<WebSocket>>, params: &ActionParam) {}

async fn init_room(socket: Arc<Mutex<WebSocket>>, params: &InitParam) {
    let room_id = generate_room_id();
    let room = Room::new(
        room_id.clone(),
        params.player.clone(),
        params.config.clone(),
    );
    {
        let mut rooms = ROOMS.lock().await;
        rooms.insert(room_id.clone(), room);
    }

    let init_room_mes = ServerMessage::InitRoom {
        room_id: room_id.clone(),
    };
    let init_room_mes = serde_json::to_string(&init_room_mes).unwrap();
    {
        let mut socket_lock = socket.lock().await;
        socket_lock
            .send(Message::Text(init_room_mes))
            .await
            .unwrap();
    }
    let (sender, _) = broadcast::channel(PLAYER_NUM);
    let receiver = sender.subscribe();
    tokio::spawn(broadcast_action(receiver, socket.clone()));
    let mut rooms_senders = ROOMS_SENDERS.lock().await;
    rooms_senders.insert(room_id.to_string(), sender);
}

async fn join_room(socket: Arc<Mutex<WebSocket>>, params: &JoinParam) {
    let mut rooms = ROOMS.lock().await;
    let mut socket_lock = socket.lock().await;
    if let Some(room) = rooms.get_mut(&params.room_id) {
        room.players.push(params.player.clone());

        let mut rooms_senders = ROOMS_SENDERS.lock().await;
        let sender = rooms_senders.get_mut(&params.room_id).unwrap();
        let receiver = sender.subscribe();
        tokio::spawn(broadcast_action(receiver, socket.clone()));

        // 广播新玩家加入房间
        let player_join_mes = ServerMessage::JoinRoom {
            player: params.player.clone(),
        };
        sender.send(serde_json::to_string(&player_join_mes).unwrap());
        // 广播游戏开始，游戏初始化数据
        let game_start_mes = ServerMessage::GameStart { room: room.clone() };
        sender.send(serde_json::to_string(&game_start_mes).unwrap());
    } else {
        let error_mes = ServerMessage::InvalidRequest {
            error: format!("Room {} does not exist", params.room_id),
        };
        let error_mes = serde_json::to_string(&error_mes).unwrap();
        socket_lock.send(Message::Text(error_mes)).await.unwrap();
    }
}

// fn parse_params(
//     mut params: JoinRoomParams,
//     headers: axum::http::header::HeaderMap,
// ) -> JoinRoomParams {
//     if let Some(room_id) = headers.get("room_id") {
//         params.room_id = Some(room_id.to_str().unwrap().to_string());
//     }
//     if let Some(user_id) = headers.get("user_id") {
//         params.user_id = user_id.to_str().unwrap().to_string();
//     }
//     if let Some(user_name) = headers.get("user_name") {
//         params.user_name = user_name.to_str().unwrap().to_string();
//     }
//     if let Some(user_icon) = headers.get("user_icon") {
//         params.user_icon = user_icon.to_str().unwrap().to_string();
//     }
//     if let Some(cols) = headers.get("cols") {
//         params.cols = usize::from_str(cols.to_str().unwrap()).ok();
//     }
//     if let Some(rows) = headers.get("rows") {
//         params.rows = usize::from_str(rows.to_str().unwrap()).ok();
//     }
//     if let Some(mines) = headers.get("mines") {
//         params.mines = usize::from_str(mines.to_str().unwrap()).ok();
//     }
//     params
// }