use crate::utils::{neighbors::Neighbors, show_matrix, Point};
use rand::prelude::SliceRandom;
use serde::Serialize;
use std::collections::HashSet;

pub struct GameBoard {
    // ------------- static data -------------
    pub cols: usize,                 // 棋盘宽度
    pub rows: usize,                 // 棋盘高度
    pub mines: usize,                // 雷的总数
    pub around_mines: Vec<Vec<u8>>,  // 每个格子周围雷的个数
    pub mine_states: Vec<Vec<bool>>, // 表示格子是否含雷

    // ------------- dynamic record -------------
    pub cell_states: Vec<Vec<CellState>>, // 每个格子的状态
    pub around_flags: Vec<Vec<u8>>,       // 每个格子周围旗的个数
    pub n_open: usize,                    // 已经打开的个数
    pub is_first_op: bool,                // 第一次玩家点击
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Closed,                 // 单元格未打开
    Opened { a_mines: u8 }, // 单元格已打开
    Flagged,                // 单元格被标记为雷
}

#[derive(Serialize, Debug, Clone)]
pub struct CellInfo {
    pub x: usize,
    pub y: usize,
    pub status: CellState,
}

pub enum OpResult {
    Ok { cells: Vec<CellInfo> },
    Over { all_mines: Vec<Point>, err_mine: Point },
    Win { all_mines: Vec<Point> },
}

impl GameBoard {
    pub fn op(&mut self, x: usize, y: usize, is_flaged: bool) -> OpResult {
        match self.cell_states[x][y] {
            CellState::Closed => match is_flaged {
                true => self.mark_cell(x, y),
                false => {
                    if self.is_first_op {
                        self.place_mines(x, y);
                        self.is_first_op = false;
                    }
                    self.open_cell(x, y)
                }
            },
            CellState::Flagged => self.cancel_mark_cell(x, y),
            CellState::Opened { a_mines: _ } => {
                if self.around_mines[x][y] == self.around_flags[x][y] {
                    self.open_around_cell(x, y)
                } else {
                    OpResult::Ok { cells: vec![] }
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
                        OpResult::Ok { cells } => op_results.extend(cells),
                        OpResult::Over { .. } | OpResult::Win { .. } => return op_res,
                    }
                }
                _ => {}
            };
        }
        OpResult::Ok { cells: op_results }
    }

    pub fn cancel_mark_cell(&mut self, x: usize, y: usize) -> OpResult {
        self.cell_states[x][y] = CellState::Closed;

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_flags[drow][dcol] -= 1;
        }

        OpResult::Ok {
            cells: vec![CellInfo {
                x,
                y,
                status: CellState::Closed,
            }],
        }
    }

    pub fn mark_cell(&mut self, x: usize, y: usize) -> OpResult {
        self.cell_states[x][y] = CellState::Flagged;

        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        for (drow, dcol) in neighbors {
            self.around_flags[drow][dcol] += 1;
        }
        OpResult::Ok {
            cells: vec![CellInfo {
                x,
                y,
                status: CellState::Flagged,
            }],
        }
    }

    pub fn open_cell(&mut self, x: usize, y: usize) -> OpResult {
        if self.mine_states[x][y] {
            return OpResult::Over {
                all_mines: self.mines_point(),
                err_mine: Point { x, y },
            };
        }
        self.cell_states[x][y] = CellState::Opened {
            a_mines: self.around_mines[x][y],
        };
        self.n_open += 1;

        let mut op_results = vec![CellInfo {
            x,
            y,
            status: self.cell_states[x][y],
        }];
        if self.around_mines[x][y] == 0 {
            let op_res = self.open_around_cell(x, y);
            match op_res {
                OpResult::Ok { cells } => op_results.extend(cells),
                _ => {}
            }
        }
        if self.is_win() {
            OpResult::Win {
                all_mines: self.mines_point(),
            }
        } else {
            OpResult::Ok { cells: op_results }
        }
    }

    fn is_win(&mut self) -> bool {
        self.n_open == self.rows * self.cols - self.mines
    }

    fn mines_point(&mut self) -> Vec<Point> {
        let mut mine_coordinates = Vec::new();
        for (y, row) in self.mine_states.iter().enumerate() {
            for (x, &is_mine) in row.iter().enumerate() {
                if is_mine {
                    mine_coordinates.push(Point { x, y });
                }
            }
        }
        mine_coordinates
    }
}

impl GameBoard {
    pub fn new(cols: usize, rows: usize, mines: usize) -> GameBoard {
        let board = GameBoard {
            cols,
            rows,
            mines,
            mine_states: vec![vec![false; cols]; rows],
            cell_states: vec![vec![CellState::Closed; cols]; rows],
            around_mines: vec![vec![0; cols]; rows],
            around_flags: vec![vec![0; cols]; rows],
            n_open: 0,
            is_first_op: true,
        };
        board
    }

    fn place_mines(&mut self, first_x: usize, first_y: usize) {
        let not_mine: Neighbors = Neighbors::new(first_x, first_y, self.rows, self.cols);
        let mut not_mine: HashSet<_> = not_mine.map(|(a, b)| (a, b)).collect();
        not_mine.insert((first_x, first_y));

        let all_positions: HashSet<(usize, usize)> = (0..self.rows)
            .flat_map(|row| (0..self.cols).map(move |col| (row, col)))
            .collect();
        let mut may_mine: Vec<_> = all_positions.difference(&not_mine).cloned().collect();
        may_mine.shuffle(&mut rand::thread_rng());

        for &(row, col) in may_mine.iter().take(self.mines) {
            self.mine_states[row][col] = true;
            let neighbors: Neighbors = Neighbors::new(row, col, self.rows, self.cols);
            for (drow, dcol) in neighbors {
                self.around_mines[drow][dcol] += 1;
            }
        }
        show_matrix(&self.around_mines, "around_mines");
        show_matrix(&self.mine_states, "mine_states");
    }
}
