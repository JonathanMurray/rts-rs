use std::cmp::{Eq, Ordering};
use std::collections::binary_heap::BinaryHeap;
use std::collections::HashMap;

use crate::core::ObstacleType;
use crate::grid::{CellRect, Grid};

pub fn find_path(
    start: [u32; 2],
    destination: Destination,
    grid: &Grid<ObstacleType>,
) -> Option<Vec<[u32; 2]>> {
    let center = destination.center();
    let rect = destination.rect();
    //println!("Finding path from {:?} to {:?}, i.e. {:?}", start, destination, rect);
    if rect.distance(start) < 10.0 {
        a_star(start, rect, grid)
    } else {
        // Especially when AI moves a lot of units at the exact same time,
        // our frame-rate takes a big hit, so we fall back to a naive version for
        // long paths.
        Some(naive_path(start, center))
    }
}

fn naive_path(start: [u32; 2], goal: [u32; 2]) -> Vec<[u32; 2]> {
    let [mut x, mut y] = start;
    let mut plan = Vec::new();
    while [x, y] != goal {
        match goal[0].cmp(&x) {
            Ordering::Less => x -= 1,
            Ordering::Greater => x += 1,
            Ordering::Equal => {}
        };
        match goal[1].cmp(&y) {
            Ordering::Less => y -= 1,
            Ordering::Greater => y += 1,
            Ordering::Equal => {}
        };
        plan.push([x, y]);
    }
    plan.reverse();
    plan
}

fn a_star(start: [u32; 2], destination: Rect, grid: &Grid<ObstacleType>) -> Option<Vec<[u32; 2]>> {
    let [w, h] = grid.dimensions;

    let mut open_set = BinaryHeap::new();
    //println!("open_set={:?}", open_set);
    open_set.push(RatedNode(start, destination.distance(start)));
    let mut came_from: HashMap<[u32; 2], [u32; 2]> = Default::default();

    let mut shortest_known_to: HashMap<[u32; 2], f32> = Default::default();
    shortest_known_to.insert(start, 0.0);
    // println!("shortest_known_to={:?}", shortest_known_to);

    while !open_set.is_empty() {
        let RatedNode(current, _) = open_set.pop().unwrap();
        // println!("current={:?}", current);
        if destination.contains(current) {
            return Some(reconstruct_path(came_from, current));
        }

        for dx in -1..=1 {
            for dy in -1..=1 {
                if [dx, dy] != [0, 0] {
                    let neighbor = [current[0] as i32 + dx, current[1] as i32 + dy];

                    if neighbor[0] >= 0
                        && neighbor[0] < w as i32
                        && neighbor[1] >= 0
                        && neighbor[1] < h as i32
                    {
                        let neighbor = [neighbor[0] as u32, neighbor[1] as u32];
                        let is_free = grid
                            .get(&neighbor)
                            .map_or(false, |obstacle| obstacle == ObstacleType::None);
                        if is_free {
                            // println!("neighbor={:?}", neighbor);

                            let maybe_shortest_to_neighbor =
                                shortest_known_to.get(&current).unwrap_or(&f32::MAX)
                                    + neighbor_distance(current, neighbor);
                            if maybe_shortest_to_neighbor
                                < *shortest_known_to.get(&neighbor).unwrap_or(&f32::MAX)
                            {
                                came_from.insert(neighbor, current);
                                shortest_known_to.insert(neighbor, maybe_shortest_to_neighbor);
                                // println!("shortest_known_to={:?}", shortest_known_to);
                                let rating_of_neighbor =
                                    maybe_shortest_to_neighbor + destination.distance(neighbor);
                                let rated_neighbor = RatedNode(neighbor, rating_of_neighbor);
                                // println!("Adding to open_set={:?}", rated_neighbor);
                                open_set.push(rated_neighbor);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug)]
pub enum Destination {
    Point([u32; 2]),
    AdjacentToEntity(CellRect),
}

impl Destination {
    fn center(&self) -> [u32; 2] {
        match &self {
            Destination::Point(p) => *p,
            Destination::AdjacentToEntity(entity_rect) => [
                entity_rect.position[0] + (entity_rect.size[0] as f32 / 2.0) as u32,
                entity_rect.position[1] + (entity_rect.size[1] as f32 / 2.0) as u32,
            ],
        }
    }

    fn rect(&self) -> Rect {
        match self {
            Destination::Point(position) => Rect {
                left: position[0] as i32,
                top: position[1] as i32,
                right: position[0],
                bottom: position[1],
            },
            Destination::AdjacentToEntity(entity_rect) => Rect {
                left: entity_rect.position[0] as i32 - 1,
                top: entity_rect.position[1] as i32 - 1,
                right: entity_rect.position[0] + entity_rect.size[0],
                bottom: entity_rect.position[1] + entity_rect.size[1],
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Rect {
    // Left and top can be negative when they extend outside of the grid.
    // For example, if a unit path-finds towards another entity that covers
    // the top-left corner of the game world, some adjacent cells of that
    // entity have negative coordinates.
    left: i32,
    top: i32,
    right: u32,
    bottom: u32,
}

impl Rect {
    fn distance(&self, cell: [u32; 2]) -> f32 {
        if (cell[0] as i32) < self.left {
            // cell is some west
            let left = self.left as u32; // Safe because cell[0] is u32 and smaller
            if (cell[1] as i32) < self.top {
                //cell is north-west
                let top = self.top as u32; // Safe because cell[1] is u32 and smaller
                distance(cell, [left, top])
            } else if cell[1] > self.bottom {
                //cell is south-west
                distance(cell, [left, self.bottom])
            } else {
                //cell is west
                distance(cell, [left, cell[1]])
            }
        } else if cell[0] > self.right {
            // cell is some east
            if (cell[1] as i32) < self.top {
                //cell is north-east
                let top = self.top as u32; // Safe because cell[1] is u32 and smaller
                distance(cell, [self.right, top])
            } else if cell[1] > self.bottom {
                //cell is south-east
                distance(cell, [self.right, self.bottom])
            } else {
                //cell is east
                distance(cell, [self.right, cell[1]])
            }
        } else if (cell[1] as i32) < self.top {
            // cell is north
            let top = self.top as u32; // Safe because cell[1] is u32 and smaller
            distance(cell, [cell[0], top])
        } else if cell[1] > self.bottom {
            // cell is south
            distance(cell, [cell[0], self.bottom])
        } else {
            // cell is within
            0.0
        }
    }

    fn contains(&self, cell: [u32; 2]) -> bool {
        cell[0] as i32 >= self.left
            && cell[1] as i32 >= self.top
            && cell[0] <= self.right
            && cell[1] <= self.bottom
    }
}

fn distance(a: [u32; 2], b: [u32; 2]) -> f32 {
    (((a[0] as i32 - b[0] as i32).pow(2) + (a[1] as i32 - b[1] as i32).pow(2)) as f32).sqrt()
}

fn neighbor_distance(cell: [u32; 2], neighbor: [u32; 2]) -> f32 {
    if cell[0] != neighbor[0] && cell[1] != neighbor[1] {
        // Diagonal distance
        1.414
    } else {
        // Straight distance
        1.0
    }
}

fn reconstruct_path(
    mut came_from: HashMap<[u32; 2], [u32; 2]>,
    mut current: [u32; 2],
) -> Vec<[u32; 2]> {
    let mut total_path = vec![current];
    while came_from.contains_key(&current) {
        current = came_from.remove(&current).unwrap();
        total_path.push(current);
    }
    total_path.pop().unwrap(); // We don't want the start position to be included
    total_path
}

#[derive(PartialEq, Debug)]
struct RatedNode([u32; 2], f32);

impl PartialOrd for RatedNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // NOTE: Inverted in order to get a min-heap instead of max-heap
        other.1.partial_cmp(&self.1)
    }
}

impl Ord for RatedNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // NOTE: Inverted in order to get a min-heap instead of max-heap
        other.1.partial_cmp(&self.1).unwrap()
    }
}

impl Eq for RatedNode {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::entities::Team;

    #[test]
    fn trivial_straight_line_path() {
        let grid = Grid::new([10, 10]);
        let path = find_path([0, 0], Destination::Point([2, 0]), &grid);
        let expected = vec![[2, 0], [1, 0]];
        assert_eq!(path, Some(expected));
    }

    #[test]
    fn diagonal_line_path() {
        let grid = Grid::new([10, 10]);
        let path = find_path([0, 0], Destination::Point([2, 2]), &grid);
        let expected = vec![[2, 2], [1, 1]];
        assert_eq!(path, Some(expected));
    }

    #[test]
    fn path_going_around_obstacle() {
        let mut grid = Grid::new([10, 10]);
        grid.set([1, 0], ObstacleType::Entity(Team::Enemy1));
        let path = find_path([0, 0], Destination::Point([2, 0]), &grid);
        let expected = vec![[2, 0], [1, 1]];
        assert_eq!(path, Some(expected));
    }

    #[test]
    fn impossible_path() {
        let mut grid = Grid::new([10, 2]);
        grid.set([2, 0], ObstacleType::Entity(Team::Enemy1));
        grid.set([2, 1], ObstacleType::Entity(Team::Enemy1));
        let path = find_path([0, 0], Destination::Point([4, 0]), &grid);
        assert_eq!(path, None);
    }

    #[test]
    fn zigzag_path() {
        let mut grid = Grid::new([10, 4]);
        grid.set([2, 0], ObstacleType::Entity(Team::Enemy1));
        grid.set([2, 1], ObstacleType::Entity(Team::Enemy1));
        grid.set([2, 2], ObstacleType::Entity(Team::Enemy1));
        grid.set([4, 3], ObstacleType::Entity(Team::Enemy1));
        grid.set([4, 2], ObstacleType::Entity(Team::Enemy1));
        grid.set([4, 1], ObstacleType::Entity(Team::Enemy1));
        let start = [0, 0];
        let path = find_path(start, Destination::Point([6, 3]), &grid).unwrap();
        visualize_path(&grid, start, &path[..]);
        let expected = vec![
            [6, 3],
            [6, 2],
            [5, 1],
            [4, 0],
            [3, 1],
            [3, 2],
            [2, 3],
            [1, 2],
            [1, 1],
        ];
        assert_eq!(path, expected);
    }

    #[test]
    fn to_structure_path() {
        let mut grid = Grid::new([10, 10]);
        let structure_cell_rect = CellRect {
            position: [7, 3],
            size: [3, 2],
        };
        grid.set([7, 3], ObstacleType::Entity(Team::Enemy1));
        grid.set([8, 3], ObstacleType::Entity(Team::Enemy1));
        grid.set([9, 3], ObstacleType::Entity(Team::Enemy1));
        grid.set([7, 4], ObstacleType::Entity(Team::Enemy1));
        grid.set([8, 4], ObstacleType::Entity(Team::Enemy1));
        grid.set([9, 4], ObstacleType::Entity(Team::Enemy1));

        let start = [4, 4];
        let path = find_path(
            start,
            Destination::AdjacentToEntity(structure_cell_rect),
            &grid,
        )
        .unwrap();
        visualize_path(&grid, start, &path[..]);
        let expected = vec![[6, 4], [5, 4]];
        assert_eq!(path, expected);
    }

    fn visualize_path(grid: &Grid<ObstacleType>, start: [u32; 2], path: &[[u32; 2]]) {
        let w = grid.dimensions[0];
        let h = grid.dimensions[1];
        print!("+");
        for _ in 0..w {
            print!("-");
        }
        println!("+");
        for y in 0..h {
            print!("|");
            for x in 0..w {
                if grid.get(&[x, y]).unwrap() != ObstacleType::None {
                    print!("#");
                } else if path.contains(&[x, y]) {
                    print!(".");
                } else if start == [x, y] {
                    print!("S");
                } else {
                    print!(" ");
                }
            }
            println!("|");
        }
        print!("+");
        for _ in 0..w {
            print!("-");
        }
        println!("+");
    }
}
