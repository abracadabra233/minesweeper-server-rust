use crate::logic::board::CellState::{Closed, Flagged};
use crate::utils::{neighbors::Neighbors, show_matrix, Point};
use rand::prelude::SliceRandom;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

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
    Over { mines: Vec<Point>, mine: Point },
    Win { mines: Vec<Point>, cells: Vec<CellInfo> },
}

impl GameBoard {
    pub fn handle_operation(&mut self, x: usize, y: usize, is_flagged: bool) -> OpResult {
        if is_flagged {
            match self.cell_states[x][y] {
                CellState::Closed => self.toggle_cell_flag(x, y, true),
                _ => OpResult::Ok { cells: vec![] },
            }
        } else {
            match self.cell_states[x][y] {
                CellState::Closed => {
                    if self.is_first_op {
                        self.place_mines(x, y);
                        self.is_first_op = false;
                    }
                    self.open_cell(x, y)
                }
                CellState::Flagged => self.toggle_cell_flag(x, y, false),
                CellState::Opened { a_mines: _ } => {
                    if self.around_mines[x][y] == self.around_flags[x][y] {
                        self.open_around_cell(x, y)
                    } else {
                        OpResult::Ok { cells: vec![] }
                    }
                }
            }
        }
    }

    fn toggle_cell_flag(&mut self, x: usize, y: usize, flag: bool) -> OpResult {
        self.cell_states[x][y] = if flag { Flagged } else { Closed };

        let adjust_amount = if flag { 1 } else { -1 };
        let neighbors: Neighbors = Neighbors::new(x, y, self.rows, self.cols);
        neighbors.for_each(|(drow, dcol)| {
            self.around_flags[drow][dcol] =
                ((self.around_flags[drow][dcol] as isize) + adjust_amount) as u8;
        });

        let status = self.cell_states[x][y];
        let cells = vec![CellInfo { x, y, status }];
        OpResult::Ok { cells }
    }

    fn open_around_cell(&mut self, x: usize, y: usize) -> OpResult {
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

    fn open_cell(&mut self, x: usize, y: usize) -> OpResult {
        if self.mine_states[x][y] {
            let mines = self.mines_point();
            let mine = Point { x, y };
            return OpResult::Over { mines, mine };
        }
        let a_mines = self.around_mines[x][y];
        self.cell_states[x][y] = CellState::Opened { a_mines };
        self.n_open += 1;
        let status = self.cell_states[x][y];
        let mut op_results = vec![CellInfo { x, y, status }];
        if self.around_mines[x][y] == 0 {
            if let OpResult::Ok { cells } = self.open_around_cell(x, y) {
                op_results.extend(cells);
            }
        }
        if self.is_win() {
            OpResult::Win {
                mines: self.mines_point(),
                cells: op_results,
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
        for (x, row) in self.mine_states.iter().enumerate() {
            for (y, &is_mine) in row.iter().enumerate() {
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

        show_matrix::<u8, String>(&self.around_mines, "around_mines", &HashMap::new());
        let mut replacements_for_mine_states = HashMap::new();
        replacements_for_mine_states.insert(true, "💣".to_string());
        replacements_for_mine_states.insert(false, "🚩".to_string());
        show_matrix(&self.mine_states, "mine_states", &replacements_for_mine_states);
    }
}
