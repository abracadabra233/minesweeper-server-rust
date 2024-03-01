pub mod logic;
pub mod utils;
use crate::logic::room::{Player, Room, RoomState};
use crate::utils::generate_room_id;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Query,
    response::Response,
    routing::get,
    Router,
};
use futures::stream::StreamExt;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;
lazy_static! {
    static ref ROOMS: Mutex<HashMap<String, Room>> = Mutex::new(HashMap::new());
}
#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", get(ws_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
#[derive(Deserialize, Debug)]
struct JoinRoomParams {
    user_id: String,
    user_name: String,
    user_icon: String,
    room_id: Option<String>,  // 玩家2加入时
    pub cols: Option<usize>,  // 玩家1加入时棋盘宽度
    pub rows: Option<usize>,  // 玩家1加入时棋盘高度
    pub mines: Option<usize>, // 玩家1加入时雷的总数
}
fn parse_params(
    mut params: JoinRoomParams,
    headers: axum::http::header::HeaderMap,
) -> JoinRoomParams {
    if let Some(room_id) = headers.get("room_id") {
        params.room_id = Some(room_id.to_str().unwrap().to_string());
    }
    if let Some(user_id) = headers.get("user_id") {
        params.user_id = user_id.to_str().unwrap().to_string();
    }
    if let Some(user_name) = headers.get("user_name") {
        params.user_name = user_name.to_str().unwrap().to_string();
    }
    if let Some(user_icon) = headers.get("user_icon") {
        params.user_icon = user_icon.to_str().unwrap().to_string();
    }
    if let Some(cols) = headers.get("cols") {
        params.cols = usize::from_str(cols.to_str().unwrap()).ok();
    }
    if let Some(rows) = headers.get("rows") {
        params.rows = usize::from_str(rows.to_str().unwrap()).ok();
    }
    if let Some(mines) = headers.get("mines") {
        params.mines = usize::from_str(mines.to_str().unwrap()).ok();
    }
    params
}
async fn ws_handler(
    Query(params): Query<JoinRoomParams>,
    ws: WebSocketUpgrade,
    headers: axum::http::header::HeaderMap,
) -> Response {
    let params = parse_params(params, headers);
    let room_id = params.room_id.clone().unwrap_or_else(generate_room_id);
    let mut rooms = ROOMS.lock().unwrap();
    let player1 = Player {
        user_id: params.user_id.clone(),
        user_name: params.user_name.clone(),
        user_icon: params.user_icon.clone(),
    };
    let room = Room::new(room_id, player1, params.cols, params.rows, params.mines);
    rooms.insert(room_id.clone(), room);
    ws.on_upgrade(move |socket| handle_socket(socket, params))
}
async fn create_room(socket: &mut WebSocket, params: &JoinRoomParams) {
    let room_id = generate_room_id();
    let player: Player = Player {
        user_id: params.user_id.clone(),
        user_name: params.user_name.clone(),
        user_icon: params.user_icon.clone(),
    };
    let room = Room::new(
        room_id.clone(),
        player,
        params.cols.unwrap(),
        params.rows.unwrap(),
        params.mines.unwrap(),
    );
    let mut rooms = ROOMS.lock().unwrap();
    rooms.insert(room_id.clone(), room);
    let response = json!({
        "type": "room_created",
        "room_id": room_id,
        "status": "waiting_for_player",
    });
    socket
        .send(Message::Text(response.to_string()))
        .await
        .unwrap();
}
async fn join_room(socket: &mut WebSocket, params: &JoinRoomParams) {
    match params.room_id.as_ref() {
        Some(id) => {
            let mut rooms = ROOMS.lock().unwrap();
            if let Some(room) = rooms.get_mut(id) {
                let player: Player = Player {
                    user_id: params.user_id.clone(),
                    user_name: params.user_name.clone(),
                    user_icon: params.user_icon.clone(),
                };
                room.players.push(player);
                let message = json!({
                    "status": "joined",
                    "room_id": room_id,
                })
                .to_string();
                let _ = socket.send(Message::Text(message)).await;
            } else {
                // 房间不存在的错误处理
                let message = json!({
                    "error": "Room does not exist",
                    "room_id": room_id
                })
                .to_string();
                let _ = socket.send(Message::Text(message)).await;
            }
        }
        None => {
            let message = json!({ "error": "Missing room ID" }).to_string();
            let _ = socket.send(Message::Text(message)).await;
            return;
        }
    };
}
async fn handle_socket(mut socket: WebSocket, params: JoinRoomParams) {
    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Close(_)) => {
                println!("玩家断开连接");
                break;
            }
            Ok(message) => {
                handle_message(message.to_text().unwrap().to_string(), &mut socket).await
            }
            Err(e) => {
                println!("连接出错: {:?}", e);
                break;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RoomMessage {
    #[serde(rename = "type")]
    message_type: String,
    room_id: Option<String>,
}

async fn handle_message(text: String, socket: &mut WebSocket) {
    let parsed_message: Result<RoomMessage, _> = serde_json::from_str(&text);

    if let Ok(message) = parsed_message {
        match message.message_type.as_str() {
            "create_room" => {
                let room_id: String = Uuid::new_v4().to_string();
                let response = json!({
                    "type": "room_created",
                    "room_id": room_id,
                    "status": "waiting_for_player",
                });
                socket
                    .send(Message::Text(response.to_string()))
                    .await
                    .unwrap();
            }
            "join_room" => {
                // 这里需要实际的逻辑来检查房间是否存在
                let response = if let Some(room_id) = message.room_id {
                    json!({
                        "type": "room_joined",
                        "room_id": room_id,
                        "status": "ready_to_start",
                    })
                } else {
                    json!({
                        "type": "error",
                        "message": "房间不存在或已满",
                    })
                };
                socket
                    .send(Message::Text(response.to_string()))
                    .await
                    .unwrap();
            }
            _ => (),
        }
    }
}
