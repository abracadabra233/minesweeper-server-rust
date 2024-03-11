use super::board::GameBoard;
use super::game::{Gconfig, Player};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomState {
    Waiting,
    Gameing,
    Logout,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub room_id: String,
    pub players: Vec<Player>,
    pub room_state: RoomState,
    pub game_state: GameBoard,
    pub gconfig: Gconfig,
}

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
        if self.players.len() == self.gconfig.n_player {
            self.room_state = RoomState::Gameing;
        }
        self.room_state
    }

    pub fn remove_player(&mut self, player_id: &str) -> RoomState {
        self.players.retain(|player| player.id != player_id);
        if self.players.len() == 0 {
            self.room_state = RoomState::Logout;
        } else {
            self.room_state = RoomState::Waiting;
            self.game_state =
                GameBoard::new(self.gconfig.cols, self.gconfig.rows, self.gconfig.mines)
        }
        self.room_state
    }
}
