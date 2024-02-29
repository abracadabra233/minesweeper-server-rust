mod board;

use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Player {
    user_id: String,
    user_name: String,
    user_icon: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GameBoard {
    board: Vec<Vec<u8>>, // 用0表示空位，1表示雷
    duration: u64,       // 游戏时长，以秒计
                         // duration: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomState {
    Waiting,
    InGame,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub room_id: String,
    pub players: Vec<Player>,
    pub room_state: RoomState,
    pub game_state: GameBoard,
}

impl Room {
    // 可以添加一个新的函数来初始化Room，包括设置初始状态
    pub fn new(room_id: String, player: Player) -> Self {
        Room {
            room_id: room_id,
            players: vec![player],
            room_state: RoomState::Waiting, // 默认状态为等待中
            game_state: GameBoard::new(),   // 假设GameState也有一个new函数
        }
    }
}
