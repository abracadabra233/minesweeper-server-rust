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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CellInfo {
    pub x: usize,
    pub y: usize,
    pub status: CellState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum OpResult {
    Success(Vec<CellInfo>), // 操作成功，返回操作影响的cell信息
    GameOver,               // 游戏失败，踩到雷
    GameWon,                // 游戏胜利
}

impl GameBoard {
    pub fn op(&mut self, x: usize, y: usize, is_flaged: bool) -> OpResult {
        match self.cell_states[x][y] {
            CellState::Closed => match is_flaged {
                true => self.mark_cell(x, y),
                false => self.open_cell(x, y),
            },
            CellState::Marked => self.cancel_mark_cell(x, y),
            CellState::Opened => {
                if self.around_mines[x][y] == self.around_marks[x][y] {
                    self.open_around_cell(x, y)
                } else {
                    OpResult::Success(vec![])
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
                        OpResult::Success(cell_infos) => op_results.extend(cell_infos),
                        OpResult::GameWon | OpResult::GameOver => return op_res,
                    }
                }
                _ => {}
            };
        }
        OpResult::Success(op_results)
    }
    pub fn cancel_mark_cell(&mut self, x: usize, y: usize) -> OpResult {
        self.cell_states[x][y] = CellState::Closed;
        self.n_marks -= 1;
        if self.mine_states[x][y] {
            self.cor_n_marks -= 1;
        }

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_marks[drow][dcol] -= 1;
        }

        OpResult::Success(vec![CellInfo {
            x,
            y,
            status: CellState::Closed,
        }])
    }
    pub fn mark_cell(&mut self, x: usize, y: usize) -> OpResult {
        self.cell_states[x][y] = CellState::Marked;
        self.n_marks += 1;
        if self.mine_states[x][y] {
            self.cor_n_marks += 1;
        }

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_marks[drow][dcol] += 1;
        }

        OpResult::Success(vec![CellInfo {
            x,
            y,
            status: CellState::Marked,
        }])
    }
    pub fn open_cell(&mut self, x: usize, y: usize) -> OpResult {
        if self.mine_states[x][y] {
            return OpResult::GameOver;
        }
        self.cell_states[x][y] = CellState::Opened;

        let mut op_results = vec![];
        if self.around_marks[x][y] == 0 {
            let op_res = self.open_around_cell(x, y);
            match op_res {
                OpResult::Success(cell_infos) => op_results.extend(cell_infos),
                OpResult::GameWon | OpResult::GameOver => {}
            }
        }
        OpResult::Success(op_results)
    }
}

// 这是一个rust实现的多人在线扫雷的核心逻辑，请通过增加结构体或是修改函数实现，帮我完善以下逻辑：1.op函数表示用户点击x，y出的格子，is_flaged表示是否是进行插旗操作，这个函数应该返回用户这次操作后，旗子上哪些cell的状态会发送改变，或者是游戏结束（失败或成功，成功需返回相应的信息），请将上述返回的信息封装在一个 enum 中 ，并返回

// 这是一个rust实现的多人在线扫雷的服务端，请根据上面的注释优化上述代码，并给出优化建议，包括但不限于以下方面：1. 变量函数命名；2. 逻辑。 还给出一些你认为需要优化的地方

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
        // dbg!(&board.around_mines);
        // dbg!(&board.mine_states);
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
