use crate::logic::board::OpResult;
use crate::logic::room::{Room, RoomState};
use crate::utils::generate_room_id;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::StreamExt;
use futures_util::{sink::SinkExt, stream::SplitSink};
use lazy_static::lazy_static;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tokio::sync::{broadcast, oneshot, Mutex};

lazy_static! {
    // 根据房间id，维护所有的房间
    static ref ROOMS: Mutex<HashMap<String, Room>> = Mutex::new(HashMap::new());
    // 根据房间id，维护所有的房间的消息广播器，用于在一个房间内广播玩家加入，玩家离开，游戏开始，玩家操作结果 信息
    static ref ROOMS_SENDERS: Mutex<HashMap<String, broadcast::Sender<ResponseModel>>> =
        Mutex::new(HashMap::new());
}

// 玩家
#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: String,     // 玩家id
    pub name: String,   // 玩家name
    pub icon: String,   // 玩家头像，base64字符串
    pub is_ready: bool, // 玩家状态
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Player {{ id: {}, name: {}, is_ready:{} }}",
            self.id, self.name, self.is_ready
        )
    }
}

// 玩家操作，玩家点击（x,y）出的格子，f 表示是否是插旗操作
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameAction {
    pub x: usize,
    pub y: usize,
    pub f: bool,
}

// 游戏设置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameConfig {
    pub cols: usize,  // 棋盘宽度
    pub rows: usize,  // 棋盘高度
    pub mines: usize, // 雷的总数
    #[serde(default = "default_n_player")]
    pub n_player: usize, // 房间人数
}

fn default_n_player() -> usize {
    2
}

// 客户端发送给服务端的消息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum RequestModel {
    RoomCreate { player: Player, config: GameConfig }, // 玩家创建房间
    RoomJoin { room_id: String, player: Player },      // 玩家加入房间
    PlayerOperation { action: GameAction },            // 玩家操作
    PlayerStatusSet { is_ready: bool },                // 准备、取消准备
}

// 服务端广播给客户端的消息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseModel {
    JoinSuccess { players: Vec<Player>, room_id: String }, // 不广播，玩家加入成功
    PlayerJoin { player: Player },                         // 广播新玩家加入
    PlayerLeft { player_id: String },                      // 广播新玩家离开
    PlayerStatusSet { player_id: String, is_ready: bool }, // 玩家准备取消准备
    GameStart { config: GameConfig },                      // 玩家齐了，游戏开始
    GameOpRes { op_res: OpResult },                        // 玩家操作后，需要改变的信息
    InvalidRequest { error: String },                      // 游戏异常：玩家离开，房间丢失等
}

type WsSender = SplitSink<WebSocket, Message>;
type BrRecver = broadcast::Receiver<ResponseModel>;

pub async fn handle_socket(socket: WebSocket) {
    let (ws_sender, mut ws_recver) = socket.split();
    let mut ws_sr: Option<SplitSink<WebSocket, Message>> = Some(ws_sender);
    let mut cur_room_id = None;
    let mut cur_player: Option<Player> = None;
    while let Some(msg) = ws_recver.next().await {
        match msg {
            Ok(Message::Close(_)) => {
                info!("{cur_room_id:?} | Request | Close, {cur_player:?}");
                leave_room(&cur_room_id, &cur_player).await;
                break;
            }
            Ok(message) => {
                let message = message.into_text().unwrap();
                let request_model: Result<RequestModel, _> = serde_json::from_str(&message);
                match request_model {
                    Ok(RequestModel::RoomCreate { player, config }) => {
                        if let Some(ws_sender) = ws_sr.take() {
                            info!("None | Request | InitRoom, {player:?}, {config:?}");
                            let room_id = init_room(&config).await;
                            if join_room(ws_sender, &room_id, &player).await {
                                cur_player = Some(player);
                                cur_room_id = Some(room_id);
                            } else {
                                break;
                            }
                        }
                    }
                    Ok(RequestModel::RoomJoin { room_id, player }) => {
                        if let Some(ws_sender) = ws_sr.take() {
                            info!("{room_id} | Request | JoinRoom, {player:?}");
                            if join_room(ws_sender, &room_id, &player).await {
                                cur_player = Some(player);
                                cur_room_id = Some(room_id);
                            } else {
                                break;
                            }
                        }
                    }
                    Ok(RequestModel::PlayerOperation { action }) => match (&cur_room_id, &cur_player) {
                        (Some(room_id), Some(player)) => {
                            info!("{room_id} | Request | GAction, {}, {action:?}", player.id);
                            handle_action(room_id, player, &action).await;
                        }
                        _ => error!("Error | Room {cur_room_id:?} or Player {cur_player:?} loss"),
                    },
                    Ok(RequestModel::PlayerStatusSet { is_ready }) => match (&cur_room_id, &cur_player) {
                        (Some(room_id), Some(player)) => {
                            info!("{room_id} | Request | PlayerStatusSet, {},{is_ready}", player.id);
                            set_player_status(room_id, &player.id, is_ready).await;
                        }
                        _ => error!("Error | Room {cur_room_id:?} or Player {cur_player:?} loss"),
                    },
                    Err(e) => {
                        warn!("{cur_room_id:?} | Warn | Parsing message:{e}");
                        if let Some(mut ws_sender) = ws_sr.take() {
                            let resp_body = serde_json::to_string(&e.to_string()).unwrap();
                            ws_sender.send(Message::Text(resp_body)).await.unwrap();
                            ws_sender.close().await.unwrap();
                        }
                        leave_room(&cur_room_id, &cur_player).await;
                        break;
                    }
                }
            }
            Err(e) => {
                error!("{cur_room_id:?} | Error | Connection exception:{e}");
                if let Some(mut ws_sender) = ws_sr.take() {
                    ws_sender.close().await.unwrap();
                }
                leave_room(&cur_room_id, &cur_player).await;
                break;
            }
        }
    }
}

async fn broadcast_action(room_id: String, mut br_recver: BrRecver, mut ws_sender: WsSender) {
    while let Ok(response) = br_recver.recv().await {
        let resp_body = serde_json::to_string(&response).unwrap();
        let _ = ws_sender.send(Message::Text(resp_body)).await;
    }
    let _ = ws_sender.close().await;
    info!("{room_id:?} | Info | release resource");
}

async fn set_player_status(room_id: &String, player_id: &String, is_ready: bool) {
    let mut rooms = ROOMS.lock().await;
    let mut rooms_senders = ROOMS_SENDERS.lock().await;

    if let Some(room) = rooms.get_mut(room_id) {
        let op_res = room.game_board.op(action.x, action.y, action.f);
        info!("{room_id} | Broadcast | PlayerStatusSet, {action:?}, {op_res:?}");
        let br_sender = rooms_senders.get_mut(room_id).unwrap();
        let _ = br_sender.send(ResponseModel::PlayerStatusSet {
            player_id: player_id.clone(),
            is_ready,
        });

        if rx.await.is_ok() && room_state == RoomState::Gameing {
            info!("{room_id} | Broadcast | GameStart | {0:?}", room.gconfig);
            room.game_board.game_start();
            let _ = br_sender.send(ResponseModel::GameStart {
                config: room.gconfig.clone(),
            });
        }
    } else {
        error!("{room_id:?} | Error | Room does not exist while {player_id:?} gaming");
    }
}

async fn handle_action(room_id: &String, player: &Player, action: &GameAction) {
    let mut rooms = ROOMS.lock().await;
    let mut rooms_senders = ROOMS_SENDERS.lock().await;

    if let Some(room) = rooms.get_mut(room_id) {
        let op_res = room.game_board.op(action.x, action.y, action.f);
        info!("{room_id} | Broadcast | GameOpRes, {action:?}, {op_res:?}");
        let br_sender = rooms_senders.get_mut(room_id).unwrap();
        let _ = br_sender.send(ResponseModel::GameOpRes { op_res });
    } else {
        error!("{room_id:?} | Error | Room does not exist while {player:?} gaming");
    }
}

async fn init_room(config: &GameConfig) -> String {
    let room_id: String = generate_room_id();
    let room = Room::new(room_id.clone(), config.clone());
    let mut rooms = ROOMS.lock().await;
    rooms.insert(room_id.clone(), room);

    let (br_sender, _) = broadcast::channel(config.n_player);
    let mut rooms_senders = ROOMS_SENDERS.lock().await;
    rooms_senders.insert(room_id.to_string(), br_sender);
    room_id
}

async fn join_room(mut ws_sender: WsSender, room_id: &String, player: &Player) -> bool {
    let mut rooms = ROOMS.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        if room.is_full() {
            let err_mes = format!("Room {} is already full,{:?}", room_id, player);
            info!("{room_id} | Response | InvalidRequest, {err_mes:?}");
            let error_mes = ResponseModel::InvalidRequest { error: err_mes };
            let error_mes = serde_json::to_string(&error_mes).unwrap();
            ws_sender.send(Message::Text(error_mes)).await.unwrap();
            ws_sender.close().await.unwrap();
            return false;
        }
        // Tell the client which players are already in the current room
        let response = ResponseModel::JoinSuccess {
            players: room.players.clone(),
            room_id: room_id.clone(),
        };
        let response = serde_json::to_string(&response).unwrap();
        info!("{room_id} | Response | JoinSuccess, {player:?}");
        ws_sender.send(Message::Text(response)).await.unwrap();

        // Broadcast to players in the current room with new players joining
        info!("{room_id} | Broadcast | PlayerJoin, {player:?}");
        let mut rooms_senders = ROOMS_SENDERS.lock().await;
        let br_sender = rooms_senders.get_mut(room_id).unwrap();
        let _ = br_sender.send(ResponseModel::PlayerJoin {
            player: player.clone(),
        });

        // Add the new player to the current room
        room.add_player(player.clone());
        let br_recver = br_sender.subscribe();
        tokio::spawn(async move { broadcast_action(room_id.clone(), br_recver, ws_sender).await });
    } else {
        let err_mes = format!("Room {} does not exist,{:?}", room_id, player);
        info!("{room_id} | Response | InvalidRequest, {err_mes:?}");
        let error_mes = ResponseModel::InvalidRequest { error: err_mes };
        let error_mes = serde_json::to_string(&error_mes).unwrap();
        ws_sender.send(Message::Text(error_mes)).await.unwrap();
        ws_sender.close().await.unwrap();
        return false;
    }
    true
}

async fn leave_room(room_id: &Option<String>, player: &Option<Player>) {
    if let Some(ref room_id) = room_id {
        let player_id = player.as_ref().unwrap().id.clone();
        let mut rooms = ROOMS.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            let mut rooms_senders = ROOMS_SENDERS.lock().await;
            match room.pop_player(&player_id) {
                RoomState::Logout => {
                    info!("{room_id} | XXXXXX | Last PlayerLeave, Drop, {player_id}");
                    rooms.remove(room_id);
                    rooms_senders.remove(room_id);
                }
                RoomState::Waiting => {
                    info!("{room_id} | Broadcast | PlayerLeave, {player_id}");
                    let br_sender = rooms_senders.get_mut(room_id).unwrap();
                    let _ = br_sender.send(ResponseModel::PlayerLeft {
                        player_id: player_id.clone(),
                    });
                }
                _ => {}
            }
        }
    }
}
