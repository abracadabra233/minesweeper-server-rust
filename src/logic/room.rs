use super::board::{CellInfo, CellState, GameBoard, OpResult};
use super::game::{GameAction, GameConfig, Player};
use crate::utils::Point;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use std::u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    Waiting, // 等待玩家加入准备
    Gameing, // 游戏进行中
    Logout,  // 房间已注销
}

pub struct Room {
    pub room_id: String,                  // 房间id
    pub players: HashMap<String, Player>, // 房间内玩家
    pub room_state: RoomState,            // 房间状态
    pub game_board: GameBoard,            // 棋盘
    pub gconfig: GameConfig,              // 游戏配置

    // player op info
    pub start_time: Instant,            // 游戏开始时间
    pub all_steps: usize,               // 用户总步数
    pub id2steps: HashMap<String, u32>, // 玩家步数
    pub id2flags: HashMap<String, u8>,  // 玩家插旗数
    pub id2opens: HashMap<String, u8>,  // 玩家打开格子数
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WinInfo {
    pub id2steps: HashMap<String, u32>,
    pub id2flags: HashMap<String, u8>,
    pub id2opens: HashMap<String, u8>,
    pub all_times: u64,
    pub all_steps: usize,
    pub all_mines: Vec<Point>,
}

#[derive(Serialize, Debug, Clone)]
pub enum OpResponse {
    OpSuccess { cells: Vec<CellInfo> }, // 玩家操作后，需要改变的信息
    GameOver { all_mines: Vec<Point>, err_mine: Point }, // 玩家输了
    GameWin { win_info: WinInfo },      // 玩家赢了
}

impl Room {
    pub fn new(room_id: String, gconfig: GameConfig) -> Self {
        Room {
            room_id,
            players: HashMap::new(),
            room_state: RoomState::Waiting,
            game_board: GameBoard::new(gconfig.cols, gconfig.rows, gconfig.mines),
            gconfig,
            start_time: Instant::now(),
            all_steps: 0,
            id2steps: HashMap::new(),
            id2flags: HashMap::new(),
            id2opens: HashMap::new(),
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

    pub fn is_full(&mut self) -> bool {
        self.players.len() == self.gconfig.n_player
    }

    pub fn start_game(&mut self) {
        self.start_time = Instant::now();
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.insert(player.id.clone(), player);
    }

    pub fn pop_player(&mut self, player_id: &String) -> RoomState {
        self.players.remove(player_id);
        if self.players.len() == 0 {
            self.room_state = RoomState::Logout;
        } else {
            self.room_state = RoomState::Waiting;
            self.game_board = GameBoard::new(self.gconfig.cols, self.gconfig.rows, self.gconfig.mines)
        }
        self.room_state
    }

    pub fn op(&mut self, player_id: &String, action: &GameAction) -> OpResponse {
        self.all_steps += 1;
        if let Some(n_step) = self.id2steps.get_mut(player_id) {
            *n_step += 1;
        } else {
            self.id2steps.insert(player_id.clone(), 1);
        }
        let op_res = self.game_board.op(action.x, action.y, action.f);

        // ========================== record op res  ==========================
        match op_res {
            OpResult::Ok { cells } => {
                let (closed_count, opened_count, flagged_count) =
                    cells
                        .iter()
                        .fold((0, 0, 0), |(closed, opened, flagged), cell| match cell.status {
                            CellState::Closed => (closed + 1, opened, flagged),
                            CellState::Opened { a_mines: _ } => (closed, opened + 1, flagged),
                            CellState::Flagged => (closed, opened, flagged + 1),
                        });

                if let Some(n_flag) = self.id2flags.get_mut(player_id) {
                    *n_flag += flagged_count as u8;
                    *n_flag -= closed_count as u8;
                } else {
                    self.id2flags.insert(player_id.clone(), flagged_count as u8);
                }

                if let Some(n_open) = self.id2opens.get_mut(player_id) {
                    *n_open += opened_count as u8;
                } else {
                    self.id2opens.insert(player_id.clone(), opened_count as u8);
                }
                // ========================== check is_valid ==========================
                let counts = [closed_count, opened_count, flagged_count];
                let non_zero_counts = counts.iter().filter(|&&count| count != 0).count();
                let is_valid =
                    non_zero_counts == 1 && (opened_count != 0 || closed_count == 1 || flagged_count == 1);
                if !is_valid {
                    error!("Error: Invalid cell counts");
                }
                OpResponse::OpSuccess { cells }
            }
            OpResult::Over { all_mines, err_mine } => {
                self.reset();
                OpResponse::GameOver { all_mines, err_mine }
            }
            OpResult::Win { all_mines } => {
                self.reset();
                OpResponse::GameWin {
                    win_info: WinInfo {
                        id2steps: self.id2steps.clone(),
                        id2flags: self.id2flags.clone(),
                        id2opens: self.id2opens.clone(),
                        all_times: Instant::now().duration_since(self.start_time).as_secs(),
                        all_steps: self.all_steps,
                        all_mines,
                    },
                }
            }
        }
    }
    pub fn reset(&mut self) {
        for (_, value) in self.players.iter_mut() {
            value.is_ready = false;
        }
        self.room_state = RoomState::Waiting;
        self.game_board = GameBoard::new(self.gconfig.cols, self.gconfig.rows, self.gconfig.mines);
        self.all_steps = 0;
        self.id2steps = HashMap::new();
        self.id2flags = HashMap::new();
        self.id2opens = HashMap::new();
    }
}
