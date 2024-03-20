use std::collections::HashMap;

use super::board::GameBoard;
use super::game::{GameConfig, Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    Waiting, // 等待玩家加入准备
    Gameing, // 游戏进行中
    Logout,  // 房间已注销
}

#[derive(Debug, Clone)]
pub struct Room {
    pub room_id: String,
    pub players: HashMap<String, Player>,
    pub room_state: RoomState,
    pub game_board: GameBoard,
    pub gconfig: GameConfig,
}

impl Room {
    pub fn new(room_id: String, gconfig: GameConfig) -> Self {
        Room {
            room_id,
            players: HashMap::new(),
            room_state: RoomState::Waiting,
            game_board: GameBoard::new(gconfig.cols, gconfig.rows, gconfig.mines),
            gconfig,
        }
    }

    pub fn set_player_status(&mut self, player_id: &String, is_ready: bool) -> RoomState {
        if let Some(player) = self.players.get_mut(player_id) {
            player.is_ready = is_ready;
            if self.players.len() == self.gconfig.n_player
                && self.players.values().all(|player| player.is_ready)
            {
                self.room_state = RoomState::Gameing;
            }
        }
        self.room_state
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.insert(player.id, player);
    }

    pub fn is_full(&mut self) -> bool {
        self.players.len() == self.gconfig.n_player
    }

    pub fn pop_player(&mut self, player_id: &str) -> RoomState {
        self.players.remove(player_id);
        if self.players.len() == 0 {
            self.room_state = RoomState::Logout;
        } else {
            self.room_state = RoomState::Waiting;
            self.game_board = GameBoard::new(self.gconfig.cols, self.gconfig.rows, self.gconfig.mines)
        }
        self.room_state
    }
}
