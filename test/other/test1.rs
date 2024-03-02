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
