pub struct Neighbors {
    x: isize,
    y: isize,
    rows: isize,
    cols: isize,
    current: isize,
}

impl Neighbors {
    pub fn new(x: usize, y: usize, rows: usize, cols: usize) -> Self {
        Neighbors {
            x: x as isize,
            y: y as isize,
            rows: rows as isize,
            cols: cols as isize,
            current: 0,
        }
    }
}

impl Iterator for Neighbors {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < 9 {
            let dx = (self.current % 3) - 1;
            let dy = (self.current / 3) - 1;
            self.current += 1;

            if dx == 0 && dy == 0 {
                continue; // Skip the center point itself
            }

            let nx = self.x + dx;
            let ny = self.y + dy;

            if nx >= 0 && nx < self.cols && ny >= 0 && ny < self.rows {
                return Some((nx as usize, ny as usize));
            }
        }
        None
    }
}
