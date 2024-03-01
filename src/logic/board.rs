use crate::utils::neighbors::Neighbors;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Closed, // 单元格未打开
    Opened, // 单元格已打开
    Marked, // 单元格被标记为雷
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameBoard {
    pub cols: usize,                      // 棋盘宽度
    pub rows: usize,                      // 棋盘高度
    pub mines: usize,                     // 雷的总数
    pub around_mines: Vec<Vec<u8>>,       // 每个格子周围雷的个数
    pub around_marks: Vec<Vec<u8>>,       // 每个格子周围旗的个数
    pub mine_states: Vec<Vec<bool>>,      // 表示格子是否含雷
    pub cell_states: Vec<Vec<CellState>>, // 每个格子的状态
    pub n_marks: usize,                   // 用户已经标记的个数
    pub cor_n_marks: usize,               // 用户正确标记的个数
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
            around_marks: vec![vec![0; cols]; rows],
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
            self.mine_states[row][col] = true; // 设置为含雷

            for drow in [-1, 0, 1] {
                for dcol in [-1, 0, 1] {
                    if drow == 0 && dcol == 0 {
                        continue; // 跳过雷所在的格子本身
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
    }
}

impl GameBoard {
    pub fn op(&mut self, x: usize, y: usize, is_flaged: bool) -> bool {
        match self.cell_states[x][y] {
            CellState::Closed => match is_flaged {
                true => self.mark_cell(x, y),
                false => self.open_cell(x, y),
            },
            CellState::Marked => self.cancel_mark_cell(x, y),
            CellState::Opened => {
                self.around_mines[x][y] == self.around_marks[x][y] && self.open_around_cell(x, y)
            }
        }
    }
    pub fn open_around_cell(&mut self, x: usize, y: usize) -> bool {
        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            let op_res = match self.cell_states[drow][dcol] {
                CellState::Closed => self.open_cell(drow, dcol),
                _ => true,
            };
            if !op_res {
                return false;
            };
        }
        true
    }
    pub fn cancel_mark_cell(&mut self, x: usize, y: usize) -> bool {
        self.cell_states[x][y] = CellState::Closed;
        self.n_marks -= 1;
        if self.mine_states[x][y] {
            self.cor_n_marks -= 1;
        }

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_marks[drow][dcol] -= 1;
        }
        true
    }
    pub fn mark_cell(&mut self, x: usize, y: usize) -> bool {
        self.cell_states[x][y] = CellState::Marked;
        self.n_marks += 1;
        if self.mine_states[x][y] {
            self.cor_n_marks += 1;
        }

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_marks[drow][dcol] += 1;
        }
        true
    }
    pub fn open_cell(&mut self, x: usize, y: usize) -> bool {
        if self.mine_states[x][y] {
            return false; // 游戏结束
        }
        self.cell_states[x][y] = CellState::Opened;
        if self.around_marks[x][y] == 0 {
            let op_res = self.open_around_cell(x, y);
            if !op_res {
                return false;
            };
        }
        true
    }
}
