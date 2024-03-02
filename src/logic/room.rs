use super::board::GameBoard;
use super::game::{Gconfig, Player};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomState {
    Waiting,
    Gameing,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub room_id: String,
    pub players: Vec<Player>,
    pub room_state: RoomState,
    pub game_state: GameBoard,
    pub gconfig: Gconfig,
}

static PLAYER_NUM: usize = 2;

impl Room {
    pub fn new(room_id: String, gconfig: Gconfig) -> Self {
        Room {
            room_id,
            players: vec![],
            room_state: RoomState::Waiting,
            game_state: GameBoard::new(gconfig.cols, gconfig.rows, gconfig.mines),
            gconfig,
        }
    }

    pub fn add_player(&mut self, player: Player) -> RoomState {
        self.players.push(player);
        if self.players.len() == PLAYER_NUM {
            self.room_state = RoomState::Gameing;
        }
        self.room_state
    }
}
