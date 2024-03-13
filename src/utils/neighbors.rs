pub struct Neighbors {
    x: isize,
    y: isize,
    rows: isize,
    cols: isize,
    delta: Vec<(isize, isize)>,
    index: usize,
}

impl Neighbors {
    pub fn new(x: usize, y: usize, rows: usize, cols: usize) -> Self {
        // 定义相对于中心点可能的8个方向
        let delta = vec![
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];
        Neighbors {
            x: x as isize,
            y: y as isize,
            rows: rows as isize,
            cols: cols as isize,
            delta,
            index: 0,
        }
    }
}

impl Iterator for Neighbors {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.delta.len() {
            let (dx, dy) = self.delta[self.index];
            self.index += 1;

            let nx = self.x + dx;
            let ny = self.y + dy;

            // 检查新坐标是否在网格内
            if nx >= 0 && nx < self.rows && ny >= 0 && ny < self.cols {
                return Some((nx as usize, ny as usize));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_top_left_neighbors_3x4() {
        let neighbors = Neighbors::new(0, 0, 3, 4);
        let expected = vec![(0, 1), (1, 0), (1, 1)];
        let result: Vec<(usize, usize)> = neighbors.collect();
        assert_eq!(result, expected);
    }
    #[test]
    fn corner_top_right_neighbors_3x4() {
        let neighbors = Neighbors::new(0, 3, 3, 4);
        let expected = vec![(0, 2), (1, 2), (1, 3)];
        let result: Vec<(usize, usize)> = neighbors.collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn edge_top_middle_neighbors_3x4() {
        let neighbors = Neighbors::new(0, 2, 3, 4);
        let expected = vec![(0, 1), (0, 3), (1, 1), (1, 2), (1, 3)];
        let result: Vec<(usize, usize)> = neighbors.collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn edge_left_middle_neighbors_3x4() {
        let neighbors = Neighbors::new(1, 0, 3, 4);
        let expected = vec![(0, 0), (0, 1), (1, 1), (2, 0), (2, 1)];
        let result: Vec<(usize, usize)> = neighbors.collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn middle_cell_neighbors_3x4() {
        let neighbors = Neighbors::new(1, 2, 3, 4);
        let expected = vec![
            (0, 1),
            (0, 2),
            (0, 3),
            (1, 1),
            (1, 3),
            (2, 1),
            (2, 2),
            (2, 3),
        ];
        let result: Vec<(usize, usize)> = neighbors.collect();
        assert_eq!(result.len(), expected.len());
        for neighbor in expected {
            assert!(result.contains(&neighbor));
        }
    }
}
