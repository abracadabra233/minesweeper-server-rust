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
use tokio::sync::{broadcast, oneshot, Mutex};

lazy_static! {
    // 根据房间id，维护所有的房间
    static ref ROOMS: Mutex<HashMap<String, Room>> = Mutex::new(HashMap::new());
    // 根据房间id，维护所有的房间的消息广播器，用于在一个房间内广播玩家加入，玩家离开，游戏开始，玩家操作结果 信息
    static ref ROOMS_SENDERS: Mutex<HashMap<String, broadcast::Sender<ResponseModel>>> =
        Mutex::new(HashMap::new());
}

// 玩家
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub id: String,   // 玩家id
    pub name: String, // 玩家name
    pub icon: String, // 玩家头像，base64字符串
}

// 玩家操作，玩家点击（x,y）出的格子，f 表示是否是插旗操作
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GAction {
    pub x: usize,
    pub y: usize,
    pub f: bool,
}

// 游戏设置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Gconfig {
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
    InitRoom { player: Player, config: Gconfig },
    JoinRoom { room_id: String, player: Player },
    GAction { action: GAction },
}

// 服务端广播给客户端的消息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseModel {
    // 该消息不广播，玩家加入前，有哪些玩家
    JoinSuccess {
        players: Vec<Player>,
        room_id: String,
    },
    // 广播新玩家加入
    PlayerJoin {
        player: Player,
    },
    // 广播新玩家离开
    PlayerLeave {
        player_id: String,
    },
    // 玩家齐了，游戏开始
    GameStart {
        config: Gconfig,
    },
    // 玩家操作后，需要改变的信息
    GameOpRes {
        op_res: OpResult,
    },
    // 游戏异常：玩家离开，房间丢失等
    InvalidRequest {
        error: String,
    },
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
                    Ok(RequestModel::InitRoom { player, config }) => {
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
                    Ok(RequestModel::JoinRoom { room_id, player }) => {
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
                    Ok(RequestModel::GAction { action }) => match (&cur_room_id, &cur_player) {
                        (Some(room_id), Some(player)) => {
                            info!("{room_id} | Request | GAction, {player:?}, {action:?}");
                            handle_action(room_id, player, &action).await;
                        }
                        _ => {}
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

async fn broadcast_action(mut br_recver: BrRecver, mut ws_sender: WsSender) {
    while let Ok(response) = br_recver.recv().await {
        let resp_body = serde_json::to_string(&response).unwrap();
        let _ = ws_sender.send(Message::Text(resp_body)).await;
    }
    let _ = ws_sender.close().await;
}

async fn handle_action(room_id: &String, player: &Player, action: &GAction) {
    let mut rooms = ROOMS.lock().await;
    let mut rooms_senders = ROOMS_SENDERS.lock().await;

    if let Some(room) = rooms.get_mut(room_id) {
        let op_res = room.game_state.op(action.x, action.y, action.f);
        info!("{room_id} | Broadcast | GameOpRes, {action:?}, {op_res:?}");
        let br_sender = rooms_senders.get_mut(room_id).unwrap();
        let _ = br_sender.send(ResponseModel::GameOpRes { op_res });
    } else {
        error!("{room_id:?} | Error | Room does not exist while {player:?} gaming");
    }
}

async fn init_room(config: &Gconfig) -> String {
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
        if RoomState::Gameing == room.room_state {
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
        let room_state = room.add_player(player.clone());
        let br_recver = br_sender.subscribe();
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = tx.send(1);
            broadcast_action(br_recver, ws_sender).await
        });
        if rx.await.is_ok() && room_state == RoomState::Gameing {
            info!("{room_id} | Broadcast | GameStart ");
            let _ = br_sender.send(ResponseModel::GameStart {
                config: room.gconfig.clone(),
            });
        }
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
            match room.remove_player(&player_id) {
                RoomState::Logout => {
                    info!("{room_id} | XXXXXX | Last PlayerLeave, Drop, {player_id}");
                    rooms.remove(room_id);
                    rooms_senders.remove(room_id);
                }
                RoomState::Waiting => {
                    info!("{room_id} | Broadcast | PlayerLeave, {player_id}");
                    let br_sender = rooms_senders.get_mut(room_id).unwrap();
                    let _ = br_sender.send(ResponseModel::PlayerLeave {
                        player_id: player_id.clone(),
                    });
                }
                _ => {}
            }
        }
    }
}
