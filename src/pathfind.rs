use crate::grid::EntityGrid;
use std::cmp::{Eq, Ordering};
use std::collections::binary_heap::BinaryHeap;
use std::collections::HashMap;

// TODO: we should allow searching for a path that leads to a cell adjacent to
//       another entity. That mode should be used for attacking, gathering,
//       returning with resource, etc. As it is now, a unit may search to the
//       top-left corner of a structure, which is occupied, and not find a
//       path.

pub fn find_path(start: [u32; 2], goal: [u32; 2], grid: &EntityGrid) -> Option<Vec<[u32; 2]>> {
    if distance(start, goal) < 10.0 {
        a_star(start, goal, grid)
    } else {
        // Especially when AI moves a lot of units at the exact same time,
        // our frame-rate takes a big hit, so we fall back to a naive version for
        // long paths.
        Some(naive_path(start, goal))
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

fn a_star(start: [u32; 2], goal: [u32; 2], grid: &EntityGrid) -> Option<Vec<[u32; 2]>> {
    let [w, h] = grid.dimensions;

    let mut open_set = BinaryHeap::new();
    //println!("open_set={:?}", open_set);
    open_set.push(RatedNode(start, distance(start, goal)));
    let mut came_from: HashMap<[u32; 2], [u32; 2]> = Default::default();

    let mut shortest_known_to: HashMap<[u32; 2], f32> = Default::default();
    shortest_known_to.insert(start, 0.0);
    // println!("shortest_known_to={:?}", shortest_known_to);

    while !open_set.is_empty() {
        let RatedNode(current, _) = open_set.pop().unwrap();
        // println!("current={:?}", current);
        if current == goal {
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
                        if !grid.get(&neighbor) {
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
                                    maybe_shortest_to_neighbor + distance(neighbor, goal);
                                open_set.push(RatedNode(neighbor, rating_of_neighbor));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn distance(cell: [u32; 2], goal: [u32; 2]) -> f32 {
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
