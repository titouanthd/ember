//! Generic 2D grid container.
//!
//! Indexed by `(col, row)`, with helpers for neighborhood iteration.
//! Used by Minesweeper first, but generic enough for Snake, Match-3,
//! Roguelike, or anything tile-based.

/// A dense 2D grid. Row-major storage: `cells[row * width + col]`.
#[derive(Debug, Clone)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    cells: Vec<T>,
}

impl<T: Clone> Grid<T> {
    /// Build a grid filled with clones of `init`.
    pub fn new(width: usize, height: usize, init: T) -> Self {
        assert!(width > 0 && height > 0, "Grid must be non-empty");
        Self {
            width,
            height,
            cells: vec![init; width * height],
        }
    }

    /// Build a grid from a row-major Vec. Panics if the length doesn't
    /// match `width * height`.
    pub fn from_vec(width: usize, height: usize, cells: Vec<T>) -> Self {
        assert_eq!(cells.len(), width * height, "Grid size mismatch");
        Self {
            width,
            height,
            cells,
        }
    }
}

impl<T> Grid<T> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        false // constructed non-empty
    }

    pub fn in_bounds(&self, col: isize, row: isize) -> bool {
        col >= 0 && row >= 0 && (col as usize) < self.width && (row as usize) < self.height
    }

    pub fn get(&self, col: usize, row: usize) -> Option<&T> {
        if col >= self.width || row >= self.height {
            return None;
        }
        Some(&self.cells[row * self.width + col])
    }

    pub fn get_mut(&mut self, col: usize, row: usize) -> Option<&mut T> {
        if col >= self.width || row >= self.height {
            return None;
        }
        Some(&mut self.cells[row * self.width + col])
    }

    /// Iterate over all cells in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = (usize, usize, &T)> + '_ {
        let w = self.width;
        self.cells
            .iter()
            .enumerate()
            .map(move |(i, c)| (i % w, i / w, c))
    }

    /// Iterate mutably over all cells in row-major order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, usize, &mut T)> + '_ {
        let w = self.width;
        self.cells
            .iter_mut()
            .enumerate()
            .map(move |(i, c)| (i % w, i / w, c))
    }

    /// Iterate over the 8 neighbors of `(col, row)`. Skips out-of-bounds.
    ///
    /// Returned tuples are `(col, row, &T)`. Corners return 3, edges 5,
    /// interior 8.
    pub fn neighbors_8(&self, col: usize, row: usize) -> Vec<(usize, usize, &T)> {
        let mut out = Vec::with_capacity(8);
        for dr in -1isize..=1 {
            for dc in -1isize..=1 {
                if dc == 0 && dr == 0 {
                    continue;
                }
                let nc = col as isize + dc;
                let nr = row as isize + dr;
                if self.in_bounds(nc, nr) {
                    let (nc, nr) = (nc as usize, nr as usize);
                    out.push((nc, nr, &self.cells[nr * self.width + nc]));
                }
            }
        }
        out
    }

    /// Iterate over the 4 orthogonal neighbors of `(col, row)`.
    pub fn neighbors_4(&self, col: usize, row: usize) -> Vec<(usize, usize, &T)> {
        let mut out = Vec::with_capacity(4);
        for (dc, dr) in [(0isize, -1isize), (1, 0), (0, 1), (-1, 0)] {
            let nc = col as isize + dc;
            let nr = row as isize + dr;
            if self.in_bounds(nc, nr) {
                let (nc, nr) = (nc as usize, nr as usize);
                out.push((nc, nr, &self.cells[nr * self.width + nc]));
            }
        }
        out
    }

    /// Iterate over the 8 neighbor *coordinates* of `(col, row)` without
    /// holding a borrow on the grid. Useful when you need to mutate cells
    /// based on their neighbors.
    pub fn neighbor_coords_8(&self, col: usize, row: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::with_capacity(8);
        for dr in -1isize..=1 {
            for dc in -1isize..=1 {
                if dc == 0 && dr == 0 {
                    continue;
                }
                let nc = col as isize + dc;
                let nr = row as isize + dr;
                if self.in_bounds(nc, nr) {
                    out.push((nc as usize, nr as usize));
                }
            }
        }
        out
    }

    /// Call `f(col, row, &cell)` for every cell.
    pub fn for_each<F: FnMut(usize, usize, &T)>(&self, mut f: F) {
        for (c, r, cell) in self.iter() {
            f(c, r, cell);
        }
    }

    /// Call `f(col, row, &mut cell)` for every cell.
    pub fn for_each_mut<F: FnMut(usize, usize, &mut T)>(&mut self, mut f: F) {
        let w = self.width;
        for (i, cell) in self.cells.iter_mut().enumerate() {
            f(i % w, i / w, cell);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_fills_with_init() {
        let g: Grid<i32> = Grid::new(3, 2, 7);
        assert_eq!(g.width(), 3);
        assert_eq!(g.height(), 2);
        assert_eq!(g.len(), 6);
        g.for_each(|_, _, c| assert_eq!(*c, 7));
    }

    #[test]
    fn test_from_vec_row_major() {
        let g = Grid::from_vec(3, 2, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(*g.get(0, 0).unwrap(), 1);
        assert_eq!(*g.get(2, 0).unwrap(), 3);
        assert_eq!(*g.get(0, 1).unwrap(), 4);
        assert_eq!(*g.get(2, 1).unwrap(), 6);
    }

    #[test]
    #[should_panic(expected = "Grid size mismatch")]
    fn test_from_vec_wrong_size_panics() {
        let _ = Grid::from_vec(3, 2, vec![1, 2, 3]);
    }

    #[test]
    fn test_get_out_of_bounds() {
        let g: Grid<i32> = Grid::new(3, 3, 0);
        assert!(g.get(3, 0).is_none());
        assert!(g.get(0, 3).is_none());
        assert!(g.get(10, 10).is_none());
    }

    #[test]
    fn test_get_mut_modifies() {
        let mut g = Grid::new(2, 2, 0);
        *g.get_mut(1, 1).unwrap() = 42;
        assert_eq!(*g.get(1, 1).unwrap(), 42);
        assert_eq!(*g.get(0, 0).unwrap(), 0);
    }

    #[test]
    fn test_neighbors_8_interior() {
        let g: Grid<i32> = Grid::new(3, 3, 0);
        let n = g.neighbors_8(1, 1);
        assert_eq!(n.len(), 8);
    }

    #[test]
    fn test_neighbors_8_corner() {
        let g: Grid<i32> = Grid::new(3, 3, 0);
        let n = g.neighbors_8(0, 0);
        assert_eq!(n.len(), 3);
        let coords: Vec<_> = n.iter().map(|(c, r, _)| (*c, *r)).collect();
        assert!(coords.contains(&(1, 0)));
        assert!(coords.contains(&(0, 1)));
        assert!(coords.contains(&(1, 1)));
    }

    #[test]
    fn test_neighbors_8_edge() {
        let g: Grid<i32> = Grid::new(3, 3, 0);
        let n = g.neighbors_8(1, 0); // top edge, not a corner
        assert_eq!(n.len(), 5);
    }

    #[test]
    fn test_neighbors_4_interior() {
        let g: Grid<i32> = Grid::new(3, 3, 0);
        let n = g.neighbors_4(1, 1);
        assert_eq!(n.len(), 4);
        let coords: Vec<_> = n.iter().map(|(c, r, _)| (*c, *r)).collect();
        assert!(coords.contains(&(0, 1)));
        assert!(coords.contains(&(2, 1)));
        assert!(coords.contains(&(1, 0)));
        assert!(coords.contains(&(1, 2)));
    }

    #[test]
    fn test_neighbors_4_corner() {
        let g: Grid<i32> = Grid::new(3, 3, 0);
        let n = g.neighbors_4(0, 0);
        assert_eq!(n.len(), 2);
    }

    #[test]
    fn test_neighbor_coords_8_matches_neighbors_8() {
        let g: Grid<i32> = Grid::new(4, 4, 0);
        let a: Vec<_> = g
            .neighbors_8(2, 2)
            .iter()
            .map(|(c, r, _)| (*c, *r))
            .collect();
        let b = g.neighbor_coords_8(2, 2);
        assert_eq!(a.len(), b.len());
        for coord in a {
            assert!(b.contains(&coord));
        }
    }

    #[test]
    fn test_iter_row_major_order() {
        let g = Grid::from_vec(3, 2, vec![1, 2, 3, 4, 5, 6]);
        let collected: Vec<_> = g.iter().map(|(c, r, v)| (c, r, *v)).collect();
        assert_eq!(collected[0], (0, 0, 1));
        assert_eq!(collected[2], (2, 0, 3));
        assert_eq!(collected[3], (0, 1, 4));
        assert_eq!(collected[5], (2, 1, 6));
    }

    #[test]
    fn test_iter_mut_modifies_all() {
        let mut g = Grid::new(2, 2, 0);
        g.iter_mut().for_each(|(c, r, v)| *v = (c + r * 10) as i32);
        assert_eq!(*g.get(1, 1).unwrap(), 11);
    }
}
