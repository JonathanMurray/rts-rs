use crate::core::ObstacleType;

pub struct ObstacleGrid {
    grid: _Grid<ObstacleType>,
}

impl ObstacleGrid {
    pub fn new(dimensions: [u32; 2]) -> Self {
        let grid = _Grid::new(dimensions);
        Self { grid }
    }

    pub fn set(&mut self, position: [u32; 2], obstacle: ObstacleType) {
        let cell_index = self.grid.cell_index(&position).unwrap_or_else(|| {
            panic!(
                "Trying to set grid{:?}={:?} but this is outside of the grid!",
                position, obstacle
            );
        });

        let old = self.grid.cells[cell_index];
        if obstacle == ObstacleType::None && old == ObstacleType::None {
            panic!("Trying to double-free obstacle grid{:?}", position);
        }
        if obstacle != ObstacleType::None && old != ObstacleType::None {
            panic!(
                "Trying to occupy grid{:?}={:?} but it's already occupied by {:?}",
                position, obstacle, old
            )
        }
        self.grid.cells[cell_index] = obstacle;
    }

    pub fn set_area(&mut self, area: CellRect, obstacle: ObstacleType) {
        for x in area.position[0]..area.position[0] + area.size[0] {
            for y in area.position[1]..area.position[1] + area.size[1] {
                self.set([x, y], obstacle);
            }
        }
    }

    pub fn get(&self, position: &[u32; 2]) -> Option<ObstacleType> {
        self.grid.cell_index(position).map(|i| self.grid.cells[i])
    }

    pub fn dimensions(&self) -> [u32; 2] {
        self.grid.dimensions
    }
}

pub struct Grid<T> {
    grid: _Grid<T>,
}

impl<T: std::fmt::Debug + PartialEq + Copy + Default> Grid<T> {
    pub fn new(dimensions: [u32; 2]) -> Self {
        let grid = _Grid::new(dimensions);
        Self { grid }
    }

    pub fn set(&mut self, position: [u32; 2], value: T) {
        let cell_index = self.grid.cell_index(&position).unwrap_or_else(|| {
            panic!(
                "Trying to set grid{:?}={:?} but this is outside of the grid!",
                position, value
            );
        });
        self.grid.cells[cell_index] = value;
    }

    pub fn set_area(&mut self, area: CellRect, value: T) {
        for x in area.position[0]..area.position[0] + area.size[0] {
            for y in area.position[1]..area.position[1] + area.size[1] {
                self.set([x, y], value);
            }
        }
    }

    pub fn get(&self, position: &[u32; 2]) -> Option<T> {
        self.grid.cell_index(position).map(|i| self.grid.cells[i])
    }

    pub fn dimensions(&self) -> [u32; 2] {
        self.grid.dimensions
    }
}

struct _Grid<T> {
    cells: Vec<T>,
    dimensions: [u32; 2],
}

impl<T: std::fmt::Debug + PartialEq + Copy + Default> _Grid<T> {
    pub fn new(dimensions: [u32; 2]) -> Self {
        let cells = vec![Default::default(); (dimensions[0] * dimensions[1]) as usize];
        Self { cells, dimensions }
    }

    fn cell_index(&self, position: &[u32; 2]) -> Option<usize> {
        let [x, y] = *position;
        if x >= self.dimensions[0] || y >= self.dimensions[1] {
            None
        } else {
            Some((y * self.dimensions[0] + x) as usize)
        }
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
