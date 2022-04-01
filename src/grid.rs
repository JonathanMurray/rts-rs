pub struct EntityGrid {
    // TODO Store EntityId's instead, to get constant position->entity_id lookup?
    //      (although entity_id->entity is still not constant currently)
    grid: Vec<bool>,
    pub dimensions: [u32; 2],
}

impl EntityGrid {
    pub fn new(dimensions: [u32; 2]) -> Self {
        let grid = vec![false; (dimensions[0] * dimensions[1]) as usize];
        Self { grid, dimensions }
    }

    pub fn set(&mut self, position: [u32; 2], occupied: bool) {
        if position[0] >= self.dimensions[0] || position[1] >= self.dimensions[1] {
            panic!(
                "Trying to set grid{:?}={} but this is outside of the grid!",
                position, occupied
            );
        }
        let i = self.index(&position);
        // Protect against bugs where two entities occupy same cell or we "double free" a cell
        assert_ne!(
            self.grid[i], occupied,
            "Trying to set grid{:?}={} but it already has that value!",
            position, occupied
        );
        self.grid[i] = occupied;
    }

    pub fn get(&self, position: &[u32; 2]) -> bool {
        if position[0] >= self.dimensions[0] || position[1] >= self.dimensions[1] {
            // If someone asks about a cell that's outside of the grid, we're being
            // nice and report the cell as occupied instead of crashing. Doing
            // the bound-checks at every call-site seems error-prone and cumbersome.
            // Also, as we translate a 2D coordinate to a 1D-index we would risk
            // reading the status of an entirely different cell.
            return true;
        }
        self.grid[self.index(position)]
    }

    pub fn set_area(&mut self, position: &[u32; 2], size: &[u32; 2], occupied: bool) {
        let [w, h] = size;
        for x in position[0]..position[0] + w {
            for y in position[1]..position[1] + h {
                self.set([x, y], occupied);
            }
        }
    }

    fn index(&self, position: &[u32; 2]) -> usize {
        let [x, y] = position;
        (y * self.dimensions[0] + x) as usize
    }
}
