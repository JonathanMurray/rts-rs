pub struct EntityGrid {
    grid: Vec<bool>,
    pub dimensions: [u32; 2],
}

impl EntityGrid {
    pub fn new(dimensions: [u32; 2]) -> Self {
        let grid = vec![false; (dimensions[0] * dimensions[1]) as usize];
        Self { grid, dimensions }
    }

    pub fn set(&mut self, position: &[u32; 2], occupied: bool) {
        let i = self.index(position);
        // Protect against bugs where two entities occupy same cell or we "double free" a cell
        assert_ne!(
            self.grid[i], occupied,
            "Trying to set grid{:?}={} but it already has that value!",
            position, occupied
        );
        self.grid[i] = occupied;
    }

    pub fn get(&self, position: &[u32; 2]) -> bool {
        self.grid[self.index(position)]
    }

    pub fn set_area(&mut self, position: &[u32; 2], size: &[u32; 2], occupied: bool) {
        let [w, h] = size;
        for x in position[0]..position[0] + w {
            for y in position[1]..position[1] + h {
                self.set(&[x, y], occupied);
            }
        }
    }

    fn index(&self, position: &[u32; 2]) -> usize {
        let [x, y] = position;
        (y * self.dimensions[0] + x) as usize
    }
}
