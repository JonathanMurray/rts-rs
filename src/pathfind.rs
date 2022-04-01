use crate::grid::EntityGrid;
use std::cmp::{Eq, Ordering};
use std::collections::HashMap;

pub fn find_path(start: [u32; 2], goal: [u32; 2], grid: &EntityGrid) -> Option<Vec<[u32; 2]>> {
    a_star(start, goal, grid)
}

pub fn a_star(start: [u32; 2], goal: [u32; 2], grid: &EntityGrid) -> Option<Vec<[u32; 2]>> {
    let [w, h] = grid.dimensions;

    let mut open_set: Vec<[u32; 2]> = Default::default();
    //println!("open_set={:?}", open_set);
    open_set.push(start);
    let mut came_from: HashMap<[u32; 2], [u32; 2]> = Default::default();

    let mut shortest_known_to: HashMap<[u32; 2], f32> = Default::default();
    shortest_known_to.insert(start, 0.0);
    // println!("shortest_known_to={:?}", shortest_known_to);

    let mut estimated_goodness: HashMap<[u32; 2], f32> = Default::default();
    estimated_goodness.insert(start, estimate_distance_to_goal(start, goal));
    // println!("estimated_goodness={:?}", estimated_goodness);

    while !open_set.is_empty() {
        let current = *open_set
            .iter()
            .min_by_key(|node| OrderedFloat(*estimated_goodness.get(*node).unwrap()))
            .unwrap();
        // println!("current={:?}", current);
        if current == goal {
            return Some(reconstruct_path(came_from, current));
        }

        open_set.retain(|node| node != &current);
        // println!("open_set={:?}", open_set);

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
                        if !grid.get(&neighbor) {
                            // println!("neighbor={:?}", neighbor);

                            let maybe_shortest_to =
                                shortest_known_to.get(&current).unwrap_or(&f32::MAX)
                                    + neighbor_distance(current, neighbor);
                            if maybe_shortest_to
                                < *shortest_known_to.get(&neighbor).unwrap_or(&f32::MAX)
                            {
                                came_from.insert(neighbor, current);
                                shortest_known_to.insert(neighbor, maybe_shortest_to);
                                // println!("shortest_known_to={:?}", shortest_known_to);
                                estimated_goodness.insert(
                                    neighbor,
                                    maybe_shortest_to + estimate_distance_to_goal(neighbor, goal),
                                );
                                // println!("estimated_goodness={:?}", estimated_goodness);
                                if !open_set.contains(&neighbor) {
                                    open_set.push(neighbor);
                                    // println!("open_set={:?}", open_set);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn estimate_distance_to_goal(cell: [u32; 2], goal: [u32; 2]) -> f32 {
    (((cell[0] as i32 - goal[0] as i32).pow(2) + (cell[1] as i32 - goal[1] as i32).pow(2)) as f32)
        .sqrt()
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

#[derive(PartialEq)]
struct OrderedFloat(f32);

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}

impl Eq for OrderedFloat {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn trivial_straight_line_path() {
        let grid = EntityGrid::new([10, 10]);
        let path = find_path([0, 0], [2, 0], &grid);
        let expected = vec![[2, 0], [1, 0]];
        assert_eq!(path, Some(expected));
    }

    #[test]
    fn diagonal_line_path() {
        let grid = EntityGrid::new([10, 10]);
        let path = find_path([0, 0], [2, 2], &grid);
        let expected = vec![[2, 2], [1, 1]];
        assert_eq!(path, Some(expected));
    }

    #[test]
    fn path_going_around_obstacle() {
        let mut grid = EntityGrid::new([10, 10]);
        grid.set([1, 0], true);
        let path = find_path([0, 0], [2, 0], &grid);
        let expected = vec![[2, 0], [1, 1]];
        assert_eq!(path, Some(expected));
    }

    #[test]
    fn impossible_path() {
        let mut grid = EntityGrid::new([10, 2]);
        grid.set([2, 0], true);
        grid.set([2, 1], true);
        let path = find_path([0, 0], [4, 0], &grid);
        assert_eq!(path, None);
    }

    #[test]
    fn zigzag_path() {
        let mut grid = EntityGrid::new([10, 4]);
        grid.set([2, 0], true);
        grid.set([2, 1], true);
        grid.set([2, 2], true);
        grid.set([4, 3], true);
        grid.set([4, 2], true);
        grid.set([4, 1], true);
        let path = find_path([0, 0], [6, 3], &grid).unwrap();
        visualize_path(&grid, &path[..]);
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

    fn visualize_path(grid: &EntityGrid, path: &[[u32; 2]]) {
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
                if grid.get(&[x, y]) {
                    print!("#");
                } else if path.contains(&[x, y]) {
                    print!(".");
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
