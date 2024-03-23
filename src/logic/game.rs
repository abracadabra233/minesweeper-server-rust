use crate::logic::room::{OpResponse, Room, RoomState};
use crate::utils::generate_room_id;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::StreamExt;
use futures_util::{sink::SinkExt, stream::SplitSink};
use lazy_static::lazy_static;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tokio::sync::{broadcast, Mutex};

lazy_static! {
    // 根据房间id，维护所有的房间
    static ref ROOMS: Mutex<HashMap<String, Room>> = Mutex::new(HashMap::new());
    // 根据房间id，维护所有的房间的消息广播器，用于在一个房间内广播玩家加入，玩家离开，游戏开始，玩家操作结果 信息
    static ref ROOMS_SENDERS: Mutex<HashMap<String, broadcast::Sender<ResponseModel>>> =
        Mutex::new(HashMap::new());
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: String,     // 玩家id
    pub name: String,   // 玩家name
    pub icon: String,   // 玩家头像，base64字符串
    pub is_ready: bool, // 玩家状态
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameAction {
    pub x: usize, // 玩家点击行号
    pub y: usize, // 玩家点击列号
    pub f: bool,  // 是否是插旗操作
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
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum RequestModel {
    RoomCreate { player: Player, config: GameConfig }, // 玩家创建房间
    RoomJoin { room_id: String, player: Player },      // 玩家加入房间
    PlayerOperation { action: GameAction },            // 玩家操作
    PlayerStatusSet { is_ready: bool },                // 准备、取消准备
}

// 服务端广播给客户端的消息
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseModel {
    JoinSuccess { players: Vec<Player>, room_id: String }, // 不广播，玩家加入成功
    PlayerJoin { player: Player },                         // 广播新玩家加入
    PlayerLeft { player_id: String },                      // 广播新玩家离开
    PlayerStatusSet { player_id: String, is_ready: bool }, // 玩家准备取消准备
    GameStart { config: GameConfig },                      // 玩家齐了，游戏开始
    GameOpRes { player_id: String, op_res: OpResponse },   // 玩家操作后，需要改变的信息
    InvalidRequest { error: String },                      // 异常：玩家离开，房间丢失等
}

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    // ================== ServiceError ==================
    #[error("ServiceError: Room still exists while trying to create or join")]
    RoomStillExists,
    #[error("ServiceError: Room or player unexpectedly lost during player action")]
    RoomOrPlayerLost,
    #[error("ServiceError: Connection abnormal loss")]
    AbnormalDisconnection,
    #[error("ServiceError: Connection exception")]
    ConnectionException,
}

#[derive(thiserror::Error, Debug)]
pub enum InvalidRequest {
    #[error("InvalidRequest: Parsing message failed")]
    InvalidMessage,
    #[error("InvalidRequest: Room is already full")]
    RoomIsFulled,
    #[error("InvalidRequest: Room does not exist")]
    RoomNotExist,
}

#[derive(thiserror::Error, Debug)]
pub enum ApplicationError {
    #[error("Service error: {0}")]
    ServiceError(#[from] ServiceError),
    #[error("Request error: {0}")]
    InvalidRequest(#[from] InvalidRequest),
}

type WsSender = SplitSink<WebSocket, Message>;
type BrRecver = broadcast::Receiver<ResponseModel>;

async fn handle_message(
    message: String,
    c_ws_sender: &mut Option<WsSender>,
    c_player: &mut Option<Player>,
    c_room_id: &mut Option<String>,
) -> Result<(), ApplicationError> {
    let request_model: Result<RequestModel, _> = serde_json::from_str(&message);
    info!("Request | {request_model:?} ");
    match request_model {
        Ok(RequestModel::RoomCreate { player, config }) => {
            if let Some(ws_sender) = c_ws_sender.take() {
                if c_player.is_none() && c_room_id.is_none() {
                    // info!("Request | InitRoom, {player:?}, {config:?}");
                    let room_id = init_room(&config).await;
                    *c_player = Some(player);
                    *c_room_id = Some(room_id);
                    join_room(ws_sender, c_room_id.as_ref().unwrap(), c_player.as_ref().unwrap()).await
                } else {
                    // Err(ApplicationError::ServiceError(ServiceError::RoomStillExists))
                    Err(ServiceError::RoomStillExists.into())
                }
            } else {
                Err(ServiceError::AbnormalDisconnection.into())
            }
        }
        Ok(RequestModel::RoomJoin { room_id, player }) => {
            if let Some(ws_sender) = c_ws_sender.take() {
                if c_player.is_none() && c_room_id.is_none() {
                    // info!("{room_id} | Request | JoinRoom, {player:?}");
                    *c_player = Some(player);
                    *c_room_id = Some(room_id);
                    join_room(ws_sender, c_room_id.as_ref().unwrap(), c_player.as_ref().unwrap()).await
                } else {
                    Err(ServiceError::RoomStillExists.into())
                }
            } else {
                Err(ServiceError::AbnormalDisconnection.into())
            }
        }
        Ok(RequestModel::PlayerOperation { action }) => {
            if let (Some(room_id), Some(player)) = (&c_room_id, &c_player) {
                // info!("{room_id} | Request | GAction, {}, {action:?}", player.id);
                handle_action(room_id, &player.id, &action).await
            } else {
                Err(ServiceError::RoomOrPlayerLost.into())
            }
        }
        Ok(RequestModel::PlayerStatusSet { is_ready }) => {
            if let (Some(room_id), Some(player)) = (&c_room_id, &c_player) {
                // info!("{room_id} | Request | PlayerStatusSet, {},{is_ready}", player.id);
                set_player_status(room_id, &player.id, is_ready).await
            } else {
                Err(ServiceError::RoomOrPlayerLost.into())
            }
        }
        Err(e) => {
            warn!("{c_room_id:?} | InvalidRequest | Parsing message:{e}");
            if let Some(mut ws_sender) = c_ws_sender.take() {
                let resp_body = serde_json::to_string(&e.to_string()).unwrap();
                ws_sender.send(Message::Text(resp_body)).await.unwrap();
                let _ = ws_sender.send(Message::Close(None)).await;
            }
            leave_room(&c_room_id, &c_player).await;
            Err(InvalidRequest::InvalidMessage.into())
        }
    }
}
pub async fn handle_socket(socket: WebSocket) {
    let (ws_sender, mut ws_recver) = socket.split();
    let mut cur_ws_sender: Option<SplitSink<WebSocket, Message>> = Some(ws_sender);
    let mut cur_room_id = None;
    let mut cur_player: Option<Player> = None;
    while let Some(msg) = ws_recver.next().await {
        match msg {
            Ok(Message::Close(_)) => {
                info!("Request | Close | {cur_room_id:?} {cur_player:?}");
                leave_room(&cur_room_id, &cur_player).await;
                break;
            }
            Ok(message) => {
                let message = message.into_text().unwrap();
                match handle_message(message, &mut cur_ws_sender, &mut cur_player, &mut cur_room_id).await {
                    Err(e) => {
                        error!("{e}");
                        break;
                    }
                    Ok(_) => {}
                }
            }
            Err(e) => {
                error!("WebSocket Connection exception:{e} {cur_room_id:?}");
                if let Some(mut ws_sender) = cur_ws_sender.take() {
                    let _ = ws_sender.send(Message::Close(None)).await;
                }
                leave_room(&cur_room_id, &cur_player).await;
                break;
            }
        }
    }
}

async fn broadcast_action(room_id: String, mut br_recver: BrRecver, mut ws_sender: WsSender) {
    loop {
        match br_recver.recv().await {
            Ok(response) => {
                info!("Broadcast | {response:?} ");
                let resp_body = serde_json::to_string(&response).unwrap();
                match ws_sender.send(Message::Text(resp_body)).await {
                    Ok(_) => {}
                    Err(_) => {
                        error!("{room_id:?} | Error | Broadcast exception",);
                        break;
                    }
                }
            }
            Err(e) => {
                warn!("{:?} | Error | Receiver error: {:?}", room_id, e);
                break;
            }
        }
    }
    info!("Info | release resource {room_id:?} ");
}

async fn set_player_status(
    room_id: &String,
    player_id: &String,
    is_ready: bool,
) -> Result<(), ApplicationError> {
    let mut rooms = ROOMS.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        let room_state = room.set_player_status(player_id, is_ready);
        // info!("{room_id} | Broadcast | PlayerStatusSet, {player_id:?}, {is_ready:?},{room_state:?}");
        let mut rooms_senders = ROOMS_SENDERS.lock().await;
        let br_sender: &mut broadcast::Sender<ResponseModel> = rooms_senders.get_mut(room_id).unwrap();
        if room.gconfig.n_player != 1 {
            let _ = br_sender.send(ResponseModel::PlayerStatusSet {
                player_id: player_id.clone(),
                is_ready,
            });
        }
        if room_state == RoomState::Gameing {
            // info!("{room_id} | Broadcast | GameStart | {0:?}", room.gconfig);
            room.start_game();
            let _ = br_sender.send(ResponseModel::GameStart {
                config: room.gconfig.clone(),
            });
        }
        Ok(())
    } else {
        error!("{room_id:?} | Error | Room does not exist while {player_id:?} gaming");
        Err(ServiceError::RoomOrPlayerLost.into())
    }
}

async fn handle_action(
    room_id: &String,
    player_id: &String,
    action: &GameAction,
) -> Result<(), ApplicationError> {
    let mut rooms = ROOMS.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        let op_res = room.op(&player_id, action);
        // info!("{room_id} | Broadcast | GameOpRes, {action:?}, {op_res:?}");
        let mut rooms_senders = ROOMS_SENDERS.lock().await;
        let br_sender = rooms_senders.get_mut(room_id).unwrap();
        let _ = br_sender.send(ResponseModel::GameOpRes {
            player_id: player_id.clone(),
            op_res,
        });
        Ok(())
    } else {
        error!("{room_id:?} | Error | Room does not exist while {player_id:?} gaming");
        Err(ServiceError::RoomOrPlayerLost.into())
    }
}

async fn init_room(config: &GameConfig) -> String {
    let room_id: String = generate_room_id();
    let room = Room::new(room_id.clone(), config.clone());
    let mut rooms = ROOMS.lock().await;
    rooms.insert(room_id.clone(), room);

    let (br_sender, _) = broadcast::channel(16);
    let mut rooms_senders = ROOMS_SENDERS.lock().await;
    rooms_senders.insert(room_id.to_string(), br_sender);
    room_id
}

async fn join_room(
    mut ws_sender: WsSender,
    room_id: &String,
    player: &Player,
) -> Result<(), ApplicationError> {
    let mut rooms = ROOMS.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        if room.is_full() {
            let err_mes = format!("Room {} is already full,{:?}", room_id, player);
            info!("Response | InvalidRequest, {err_mes:?}");
            let error_mes = ResponseModel::InvalidRequest { error: err_mes };
            let error_mes: String = serde_json::to_string(&error_mes).unwrap();
            ws_sender.send(Message::Text(error_mes)).await.unwrap();
            let _ = ws_sender.send(Message::Close(None)).await;
            return Err(InvalidRequest::RoomIsFulled.into());
        }
        // Tell the client which players are already in the current room
        let response = ResponseModel::JoinSuccess {
            players: room.players.values().cloned().collect::<Vec<Player>>(),
            room_id: room_id.clone(),
        };
        let response = serde_json::to_string(&response).unwrap();
        info!("Response | JoinSuccess, {room_id} {player:?}");
        ws_sender.send(Message::Text(response)).await.unwrap();

        // Broadcast to players in the current room with new players joining
        // info!("{room_id} | Broadcast | PlayerJoin, {player:?}");
        let mut rooms_senders = ROOMS_SENDERS.lock().await;
        let br_sender = rooms_senders.get_mut(room_id).unwrap();
        let _ = br_sender.send(ResponseModel::PlayerJoin {
            player: player.clone(),
        });

        // Add the new player to the current room
        room.add_player(player.clone());
        let br_recver = br_sender.subscribe();
        let c_room_id = room_id.clone();
        tokio::spawn(async move { broadcast_action(c_room_id, br_recver, ws_sender).await });
        Ok(())
    } else {
        let err_mes = format!("Room {} does not exist,{:?}", room_id, player);
        info!("Response | InvalidRequest, {err_mes:?}");
        let error_mes = ResponseModel::InvalidRequest { error: err_mes };
        let error_mes = serde_json::to_string(&error_mes).unwrap();
        ws_sender.send(Message::Text(error_mes)).await.unwrap();
        let _ = ws_sender.send(Message::Close(None)).await;
        Err(InvalidRequest::RoomNotExist.into())
    }
}

async fn leave_room(room_id: &Option<String>, player: &Option<Player>) {
    if let Some(ref room_id) = room_id {
        let player_id = player.as_ref().unwrap().id.clone();
        let mut rooms = ROOMS.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            let mut rooms_senders = ROOMS_SENDERS.lock().await;
            match room.pop_player(&player_id) {
                RoomState::Logout => {
                    info!("Info | Last PlayerLeave, Drop,{room_id} {player_id}");
                    rooms.remove(room_id);
                    rooms_senders.remove(room_id);
                }
                RoomState::Waiting => {
                    // info!("{room_id} | Broadcast | PlayerLeave, {player_id}");
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
