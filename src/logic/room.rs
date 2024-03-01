use super::board::GameBoard;
use super::game::{Gconfig, Player};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
}

impl Room {
    pub fn new(room_id: String, player: Player, gconfig: Gconfig) -> Self {
        Room {
            room_id,
            players: vec![player],
            room_state: RoomState::Waiting,
            game_state: GameBoard::new(gconfig.cols, gconfig.rows, gconfig.mines),
        }
    }

    pub fn add(&mut self, player: Player) -> () {
        self.players.push(player);
        self.room_state = RoomState::Gameing;
    }
}
