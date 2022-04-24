pub struct Grid<T> {
    grid: Vec<T>,
    pub dimensions: [u32; 2],
}

impl<T: std::fmt::Debug + PartialEq + Copy + Default> Grid<T> {
    pub fn new(dimensions: [u32; 2]) -> Self {
        let grid = vec![Default::default(); (dimensions[0] * dimensions[1]) as usize];
        Self { grid, dimensions }
    }

    pub fn set(&mut self, position: [u32; 2], value: T) {
        if position[0] >= self.dimensions[0] || position[1] >= self.dimensions[1] {
            panic!(
                "Trying to set grid{:?}={:?} but this is outside of the grid!",
                position, value
            );
        }
        let i = self.index(&position);
        // Protect against bugs where two entities occupy same cell or we "double free" a cell
        if self.grid[i] == value {
            panic!(
                "Trying to set grid{:?}={:?} but it already has that value",
                position, value
            )
        }
        self.grid[i] = value;
    }

    pub fn get(&self, position: &[u32; 2]) -> Option<T> {
        if position[0] >= self.dimensions[0] || position[1] >= self.dimensions[1] {
            return None;
        }
        Some(self.grid[self.index(position)])
    }

    pub fn set_area(&mut self, area: CellRect, value: T) {
        for x in area.position[0]..area.position[0] + area.size[0] {
            for y in area.position[1]..area.position[1] + area.size[1] {
                self.set([x, y], value);
            }
        }
    }

    fn index(&self, position: &[u32; 2]) -> usize {
        let [x, y] = position;
        (y * self.dimensions[0] + x) as usize
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CellRect {
    pub position: [u32; 2],
    pub size: [u32; 2],
}

impl CellRect {
    pub fn contains(&self, point: [u32; 2]) -> bool {
        point[0] >= self.position[0]
            && point[0] < self.position[0] + self.size[0]
            && point[1] >= self.position[1]
            && point[1] < self.position[1] + self.size[1]
    }
}
