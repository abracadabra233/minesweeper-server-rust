use rand::prelude::SliceRandom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Closed,  // 单元格未打开
    Opened,  // 单元格已打开
    Flagged, // 单元格被标记为雷
}

#[derive(Debug, Clone)]
pub struct GameBoard {
    pub cols: usize,                      // 棋盘宽度
    pub rows: usize,                      // 棋盘高度
    pub mines: usize,                     // 雷的总数
    pub around_mines: Vec<Vec<u8>>,       // 每个格子周围雷的个数
    pub around_flags: Vec<Vec<u8>>,       // 每个格子周围旗的个数
    pub mine_states: Vec<Vec<bool>>,      // 表示格子是否含雷
    pub cell_states: Vec<Vec<CellState>>, // 每个格子的状态
    pub n_flags: usize,                   // 用户已经标记的个数
    pub cor_n_flags: usize,               // 用户正确标记的个数
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
            n_flags: 0,
            cor_n_flags: 0,
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
            CellState::Closed => {
                if is_flaged {
                    return self.flag_cell(x, y);
                } else {
                    return self.open_cell(x, y);
                }

                true
            }
            CellState::Flagged => {
                self.cell_states[x][y] = CellState::Closed;
                true
            }
            CellState::Opened => false,
            _ => false,
        }

        if self.mine_states[x][y] {
            self.cell_states[x][y] = CellState::Opened;
            return false; // 游戏结束
        }
        self.expand_empty_cells(x, y);
        true // 游戏继续
    }
    pub fn flag_cell(&mut self, x: usize, y: usize) -> bool {
        for drow in [-1, 0, 1] {
            for dcol in [-1, 0, 1] {
                if drow == 0 && dcol == 0 {
                    continue; // 跳过雷所在的格子本身
                }
                let nrow = x as isize + drow;
                let ncol = y as isize + dcol;
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
        true
    }
    pub fn open_cell(&mut self, x: usize, y: usize) -> bool {
        // 如果点击的是雷
        if self.mine_states[x][y] {
            self.cell_states[x][y] = CellState::Revealed;
            return false; // 游戏结束
        }

        // 递归地展开空白区域
        self.expand_empty_cells(x, y);
        true // 游戏继续
    }

    // 辅助方法：递归地展开没有雷的区域
    fn expand_empty_cells(&mut self, x: usize, y: usize) {
        // 越界检查
        if x >= self.rows || y >= self.cols || self.cell_states[x][y] == CellState::Revealed {
            return;
        }

        // 标记为已展开
        self.cell_states[x][y] = CellState::Revealed;

        // 如果这个格子周围没有雷，则尝试展开周围的格子
        if self.around_mines[x][y] == 0 {
            for drow in [-1, 0, 1].iter().cloned() {
                for dcol in [-1, 0, 1].iter().cloned() {
                    if drow == 0 && dcol == 0 {
                        continue;
                    }
                    let new_x = x as isize + drow;
                    let new_y = y as isize + dcol;

                    if new_x >= 0
                        && new_x < self.rows as isize
                        && new_y >= 0
                        && new_y < self.cols as isize
                    {
                        self.expand_empty_cells(new_x as usize, new_y as usize);
                    }
                }
            }
        }
    }
}
