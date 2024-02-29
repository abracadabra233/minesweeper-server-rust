pub mod logic;
pub mod utils;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Query,
    response::Response,
    routing::get,
    Router,
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", get(ws_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
#[derive(Deserialize, Debug)]
struct JoinRoomParams {
    room_id: Option<String>,
    user_id: String,
    user_name: String,
    user_icon: String,
}

async fn ws_handler(
    Query(params): Query<JoinRoomParams>,
    ws: WebSocketUpgrade,
    headers: axum::http::header::HeaderMap,
) -> Response {
    println!("{:?}", params);
    println!("{:?}", headers);
    ws.on_upgrade(move |socket| handle_socket(socket, params))
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
