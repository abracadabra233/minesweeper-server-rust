use crate::utils::{neighbors::Neighbors, show_matrix};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameBoard {
    pub cols: usize,                      // 棋盘宽度
    pub rows: usize,                      // 棋盘高度
    pub mines: usize,                     // 雷的总数
    pub around_mines: Vec<Vec<u8>>,       // 每个格子周围雷的个数
    pub around_flags: Vec<Vec<u8>>,       // 每个格子周围旗的个数
    pub mine_states: Vec<Vec<bool>>,      // 表示格子是否含雷
    pub cell_states: Vec<Vec<CellState>>, // 每个格子的状态
    pub n_marks: usize,                   // 用户已经标记的个数
    pub cor_n_marks: usize,               // 用户正确标记的个数
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Closed,                 // 单元格未打开
    Opened { a_mines: u8 }, // 单元格已打开
    Flagged,                // 单元格被标记为雷
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CellInfo {
    pub x: usize,
    pub y: usize,
    pub status: CellState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OpResult {
    Success { cells: Vec<CellInfo> }, // 操作成功，返回操作影响的cell信息
    GameOver,                         // 游戏失败，踩到雷
    GameWon,                          // 游戏胜利
}

impl GameBoard {
    pub fn op(&mut self, x: usize, y: usize, is_flaged: bool) -> OpResult {
        match self.cell_states[x][y] {
            CellState::Closed => match is_flaged {
                true => self.mark_cell(x, y),
                false => self.open_cell(x, y),
            },
            CellState::Flagged => self.cancel_mark_cell(x, y),
            CellState::Opened { a_mines: _ } => {
                if self.around_mines[x][y] == self.around_flags[x][y] {
                    self.open_around_cell(x, y)
                } else {
                    OpResult::Success { cells: vec![] }
                }
            }
        }
    }
    pub fn open_around_cell(&mut self, x: usize, y: usize) -> OpResult {
        let mut op_results = vec![];
        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            match self.cell_states[drow][dcol] {
                CellState::Closed => {
                    let op_res = self.open_cell(drow, dcol);
                    match op_res {
                        OpResult::Success { cells } => op_results.extend(cells),
                        OpResult::GameOver => return op_res,
                        OpResult::GameWon => return op_res,
                    }
                }
                _ => {}
            };
        }
        OpResult::Success { cells: op_results }
    }
    pub fn cancel_mark_cell(&mut self, x: usize, y: usize) -> OpResult {
        self.cell_states[x][y] = CellState::Closed;
        self.n_marks -= 1;
        if self.mine_states[x][y] {
            self.cor_n_marks -= 1;
        }

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_flags[drow][dcol] -= 1;
        }

        OpResult::Success {
            cells: vec![CellInfo {
                x,
                y,
                status: CellState::Closed,
            }],
        }
    }
    pub fn mark_cell(&mut self, x: usize, y: usize) -> OpResult {
        self.cell_states[x][y] = CellState::Flagged;
        self.n_marks += 1;
        if self.mine_states[x][y] {
            self.cor_n_marks += 1;
        }

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_flags[drow][dcol] += 1;
        }

        OpResult::Success {
            cells: vec![CellInfo {
                x,
                y,
                status: CellState::Flagged,
            }],
        }
    }
    pub fn open_cell(&mut self, x: usize, y: usize) -> OpResult {
        if self.mine_states[x][y] {
            return OpResult::GameOver;
        }
        self.cell_states[x][y] = CellState::Opened {
            a_mines: self.around_mines[x][y],
        };

        let mut op_results = vec![CellInfo {
            x,
            y,
            status: self.cell_states[x][y],
        }];
        if self.around_mines[x][y] == 0 {
            let op_res = self.open_around_cell(x, y);
            match op_res {
                OpResult::Success { cells } => op_results.extend(cells),
                OpResult::GameWon | OpResult::GameOver => {}
            }
        }
        OpResult::Success { cells: op_results }
    }
}

impl GameBoard {
    pub fn new(cols: usize, rows: usize, mines: usize) -> GameBoard {
        let mut board = GameBoard {
            cols,
            rows,
            mines,
            mine_states: vec![vec![false; cols]; rows],
            cell_states: vec![vec![CellState::Closed; cols]; rows],
            around_mines: vec![vec![0; cols]; rows],
            around_flags: vec![vec![0; cols]; rows],
            n_marks: 0,
            cor_n_marks: 0,
        };
        board.place_mines();
        board
    }
    fn place_mines(&mut self) {
        let mut rng = rand::thread_rng();
        let mut positions: Vec<usize> = (0..self.rows * self.cols).collect();
        positions.shuffle(&mut rng);

        for &pos in positions.iter().take(self.mines) {
            let row = pos / self.cols;
            let col = pos % self.cols;
            self.mine_states[row][col] = true;

            for drow in [-1, 0, 1] {
                for dcol in [-1, 0, 1] {
                    if drow == 0 && dcol == 0 {
                        continue;
                    }
                    let nrow = row as isize + drow;
                    let ncol = col as isize + dcol;
                    if nrow >= 0
                        && nrow < self.rows as isize
                        && ncol >= 0
                        && ncol < self.cols as isize
                        && !self.mine_states[nrow as usize][ncol as usize]
                    {
                        self.around_mines[nrow as usize][ncol as usize] += 1;
                    }
                }
            }
        }
        show_matrix(&self.around_mines, "around_mines");
        show_matrix(&self.mine_states, "mine_states");
    }
}
