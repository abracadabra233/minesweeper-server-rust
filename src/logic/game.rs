use crate::logic::board::OpResult;
use crate::logic::room::{Room, RoomState};
use crate::utils::generate_room_id;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::StreamExt;
use futures_util::{sink::SinkExt, stream::SplitSink};
use lazy_static::lazy_static;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, oneshot, Mutex};

static PLAYER_NUM: usize = 2;
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
    pub id: String,   //玩家id
    pub name: String, //玩家name
    pub icon: String, //玩家头像，base64字符串
}

// 玩家操作，玩家点击（x,y）出的格子，f 表示是否是插旗操作
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerAction {
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
}

// 客户端发送给服务端的消息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum RequestModel {
    InitRoom { player: Player, config: Gconfig },
    JoinRoom { room_id: String, player: Player },
    PlayerAction { player_action: PlayerAction },
}

// 服务端广播给客户端的消息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseModel {
    // 有新玩家加入
    JoinRoom {
        player: Player,
    },
    // 该消息不广播，创建的房间id
    InitRoom {
        room_id: String,
    },
    // 玩家齐了，游戏开始
    GameStart {
        players: Vec<Player>,
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

pub async fn handle_socket(socket: WebSocket) {
    let (ws_sender, mut ws_recver) = socket.split();
    let mut ws_sender_op = Some(ws_sender);
    let mut room_id = "room_id".to_string();
    while let Some(msg) = ws_recver.next().await {
        match msg {
            Ok(Message::Close(_)) => {
                //TODO 还需要清除全局变量的room_id,并广播信息给其他‘用户断开’，
                info!("断开连接");
                break;
            }
            Ok(message) => {
                let message = message.into_text().unwrap();
                let parsed_message: Result<RequestModel, _> = serde_json::from_str(&message);
                match parsed_message {
                    Ok(RequestModel::InitRoom { player, config }) => {
                        // 创建房间，并加入到全局变量中
                        if let Some(mut ws_sender) = ws_sender_op.take() {
                            room_id = init_room(&mut ws_sender, &config).await;
                            info!("InitRoom: {:?} {:?}", player, config);
                            join_room(ws_sender, &room_id, &player).await;
                            info!("JoinRoom: {:?} {:?}", room_id, player);
                        }
                    }
                    Ok(RequestModel::JoinRoom { room_id, player }) => {
                        if let Some(ws_sender) = ws_sender_op.take() {
                            join_room(ws_sender, &room_id, &player).await;
                            info!("JoinRoom: {:?} {:?}", room_id, player);
                        }
                    }
                    Ok(RequestModel::PlayerAction { player_action }) => {
                        // 处理玩家操作，并通知所有连接
                        handle_action(&room_id, &player_action).await;
                        info!("PlayerAction: {:?}", player_action);
                    }
                    Err(e) => {
                        warn!("Error parsing message: {}", e);
                    }
                }
                //TODO 增加错误处理，如果出错则应该断掉socket 连接
            }
            Err(e) => {
                warn!("连接出错 {}", e);
                break;
            }
        }
    }
}

async fn broadcast_action(
    mut br_recver: broadcast::Receiver<ResponseModel>,
    mut ws_sender: WsSender,
) {
    while let Ok(respose) = br_recver.recv().await {
        debug!("Broadcasting message: '{:?}' ", respose);
        let respose_mes = serde_json::to_string(&respose).unwrap();
        ws_sender.send(Message::Text(respose_mes)).await.unwrap();
    }
}

async fn handle_action(room_id: &String, action: &PlayerAction) {
    let mut rooms = ROOMS.lock().await;
    let mut rooms_senders = ROOMS_SENDERS.lock().await;
    let br_sender = rooms_senders.get_mut(room_id).unwrap();

    if let Some(room) = rooms.get_mut(room_id) {
        let op_res = room.game_state.op(action.x, action.y, action.f);
        let _ = br_sender.send(ResponseModel::GameOpRes { op_res });
    } else {
        let _ = br_sender.send(ResponseModel::InvalidRequest {
            error: format!("Room {} does not exist", room_id),
        });
    }
}

async fn init_room(ws_sender: &mut WsSender, config: &Gconfig) -> String {
    let room_id = generate_room_id();
    // 初始化房间
    let room = Room::new(room_id.clone(), config.clone());
    let mut rooms = ROOMS.lock().await;
    rooms.insert(room_id.clone(), room);

    // 发送创建房间的消息，
    let respose = ResponseModel::InitRoom {
        room_id: room_id.clone(),
    };
    let respose = serde_json::to_string(&respose).unwrap();
    ws_sender.send(Message::Text(respose)).await.unwrap();

    //初始化广播通道
    let (br_sender, _) = broadcast::channel(PLAYER_NUM);
    let mut rooms_senders = ROOMS_SENDERS.lock().await;
    rooms_senders.insert(room_id.to_string(), br_sender);

    room_id
}

async fn join_room(mut ws_sender: WsSender, room_id: &String, player: &Player) {
    let mut rooms = ROOMS.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        let room_state = room.add_player(player.clone());

        let mut rooms_senders = ROOMS_SENDERS.lock().await;
        let br_sender = rooms_senders.get_mut(room_id).unwrap();
        let br_recver = br_sender.subscribe();

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = tx.send(1);
            broadcast_action(br_recver, ws_sender).await
        });

        match rx.await {
            Ok(_) => {
                // 广播新玩家加入房间
                let _ = br_sender.send(ResponseModel::JoinRoom {
                    player: player.clone(),
                });
                // 广播游戏开始，游戏初始化数据
                if room_state == RoomState::Gameing {
                    let _ = br_sender.send(ResponseModel::GameStart {
                        players: room.players.clone(),
                        config: room.gconfig.clone(),
                    });
                }
            }
            _ => (),
        }
    } else {
        let error_mes = ResponseModel::InvalidRequest {
            error: format!("Room {} does not exist", room_id),
        };
        let error_mes = serde_json::to_string(&error_mes).unwrap();
        ws_sender.send(Message::Text(error_mes)).await.unwrap();
    }
}
