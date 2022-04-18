use rand::Rng;

use ggez::Context;
use std::io::Read;
use std::path::Path;

use crate::data::{self, create_entity, EntityType};
use crate::entities::{Entity, Team};
use crate::grid::{CellRect, Grid};

#[derive(Debug, PartialEq)]
pub enum MapType {
    Empty,
    Small,
    Medium,
    LoadTest,
}

pub enum MapConfig {
    Type(MapType),
    FromFile(Box<dyn AsRef<Path>>),
}

pub struct WorldInitData {
    pub dimensions: [u32; 2],
    pub entities: Vec<Entity>,
    pub water_grid: Grid<()>,
    pub tile_grid: Grid<TileId>,
}

impl WorldInitData {
    pub fn load(ctx: &mut Context, config: MapConfig) -> Self {
        match config {
            MapConfig::Type(map_type) => Self::load_from_type(map_type),
            MapConfig::FromFile(path) => Self::load_from_file(ctx, path.as_ref()),
        }
    }

    pub fn load_from_type(map_type: MapType) -> Self {
        let dimensions = match map_type {
            MapType::Empty => [30, 20],
            MapType::Small => [30, 20],
            MapType::Medium => [30, 20],
            MapType::LoadTest => [100, 100],
        };

        let mut water_grid = Grid::new(dimensions);
        for x in 0..dimensions[0] {
            for y in 0..dimensions[1] {
                let water_cell = x % 4 == 0 && (y % 3 < 2);
                if water_cell {
                    water_grid.set([x, y], Some(()));
                }
            }
        }
        let tile_grid = create_tile_grid(&water_grid);

        let mut entities = vec![
            data::create_entity(EntityType::Worker, [6, 2], Team::Player),
            data::create_entity(EntityType::Fighter, [8, 2], Team::Player),
            data::create_entity(EntityType::TechLab, [1, 6], Team::Player),
        ];

        entities.push(data::create_entity(
            EntityType::FuelRift,
            [6, 4],
            Team::Neutral,
        ));

        match map_type {
            MapType::Empty => {}
            MapType::Small => {
                entities.push(data::create_entity(
                    EntityType::Fighter,
                    [7, 7],
                    Team::Enemy,
                ));
                entities.push(data::create_entity(
                    EntityType::TechLab,
                    [1, 2],
                    Team::Enemy,
                ));
            }
            MapType::Medium => {
                entities.push(data::create_entity(EntityType::Worker, [5, 2], Team::Enemy));
                entities.push(data::create_entity(EntityType::Worker, [3, 0], Team::Enemy));
                entities.push(data::create_entity(EntityType::Worker, [0, 4], Team::Enemy));
                entities.push(data::create_entity(EntityType::Worker, [3, 4], Team::Enemy));
                entities.push(data::create_entity(
                    EntityType::TechLab,
                    [8, 4],
                    Team::Enemy,
                ));
            }
            MapType::LoadTest => {
                let mut rng = rand::thread_rng();
                let dimensions = [100, 100];
                for y in 5..dimensions[1] {
                    for x in 5..dimensions[0] {
                        if rng.gen_bool(0.2) {
                            let team = if rng.gen_bool(0.5) {
                                Team::Player
                            } else {
                                Team::Enemy
                            };
                            let entity_type = if rng.gen_bool(0.5) {
                                EntityType::Worker
                            } else {
                                EntityType::Fighter
                            };
                            entities.push(data::create_entity(entity_type, [x, y], team));
                        }
                    }
                }
            }
        };

        entities.retain(|entity| {
            let r = entity.cell_rect();
            for x in r.position[0]..r.position[0] + r.size[0] {
                for y in r.position[1]..r.position[1] + r.size[1] {
                    if water_grid.get(&[x, y]).is_some() {
                        println!(
                            "WARN: Removing {:?} because it's occupying {:?} which is already covered by water",
                            entity,
                            [x, y]
                        );
                        return false;
                    }
                }
            }
            true
        });

        Self {
            dimensions,
            entities,
            water_grid,
            tile_grid,
        }
    }

    pub fn load_from_file(ctx: &mut Context, path: impl AsRef<Path>) -> Self {
        let mut file = ggez::filesystem::open(ctx, path).unwrap();
        let mut map = String::new();
        file.read_to_string(&mut map).unwrap();
        let rows: Vec<&str> = map.lines().collect();
        let w = (rows[0].len() - 2) as u32;
        let h = (rows.len() - 2) as u32;
        for line in &rows {
            assert_eq!(line.len() - 2, w as usize);
        }

        let mut entities = Vec::new();
        let mut water_grid = Grid::new([w, h]);

        for x in 0..w {
            for y in 0..h {
                let ch = rows[(y + 1) as usize].as_bytes()[(x + 1) as usize] as char;
                match ch {
                    'W' => {
                        water_grid.set([x as u32, y as u32], Some(()));
                    }
                    '1' => {
                        entities.push(create_entity(EntityType::TechLab, [x, y], Team::Player));
                    }
                    '2' => {
                        entities.push(create_entity(EntityType::TechLab, [x, y], Team::Enemy));
                    }
                    'R' => {
                        entities.push(create_entity(EntityType::FuelRift, [x, y], Team::Neutral));
                    }
                    _ => {}
                }
            }
        }

        let tile_grid = create_tile_grid(&water_grid);

        Self {
            dimensions: [w as u32, h as u32],
            entities,
            water_grid,
            tile_grid,
        }
    }
}

fn create_tile_grid(water_grid: &Grid<()>) -> Grid<TileId> {
    let [w, h] = water_grid.dimensions;
    let mut tile_grid = Grid::new([w * 2, h * 2]);
    for x in 0..w {
        for y in 0..h {
            if water_grid.get(&[x, y]).is_some() {
                // Pick water tiles based on neighbouring cells,

                let land_n = if y > 0 {
                    water_grid.get(&[x, y - 1]).is_none()
                } else {
                    false
                };
                let land_ne = if x < w - 1 && y > 0 {
                    water_grid.get(&[x + 1, y - 1]).is_none()
                } else {
                    false
                };
                let land_e = if x < w - 1 {
                    water_grid.get(&[x + 1, y]).is_none()
                } else {
                    false
                };
                let land_se = if x < w - 1 && y < h - 1 {
                    water_grid.get(&[x + 1, y + 1]).is_none()
                } else {
                    false
                };
                let land_s = if y < h - 1 {
                    water_grid.get(&[x, y + 1]).is_none()
                } else {
                    false
                };
                let land_sw = if x > 0 && y < h - 1 {
                    water_grid.get(&[x - 1, y + 1]).is_none()
                } else {
                    false
                };
                let land_w = if x > 0 {
                    water_grid.get(&[x - 1, y]).is_none()
                } else {
                    false
                };
                let land_nw = if x > 0 && y > 0 {
                    water_grid.get(&[x - 1, y - 1]).is_none()
                } else {
                    false
                };

                let topright = if land_n && land_e {
                    TileId::WaterCornerNE
                } else if land_n {
                    TileId::WaterEdgeNorth
                } else if land_e {
                    TileId::WaterEdgeEast
                } else if land_ne {
                    TileId::WaterConcaveNE
                } else {
                    TileId::WaterCenter
                };
                tile_grid.set([x * 2 + 1, y * 2], Some(topright));

                let botright = if land_s && land_e {
                    TileId::WaterCornerSE
                } else if land_s {
                    TileId::WaterEdgeSouth
                } else if land_e {
                    TileId::WaterEdgeEast
                } else if land_se {
                    TileId::WaterConcaveSE
                } else {
                    TileId::WaterCenter
                };
                tile_grid.set([x * 2 + 1, y * 2 + 1], Some(botright));

                let botleft = if land_s && land_w {
                    TileId::WaterCornerSW
                } else if land_s {
                    TileId::WaterEdgeSouth
                } else if land_w {
                    TileId::WaterEdgeWest
                } else if land_sw {
                    TileId::WaterConcaveSW
                } else {
                    TileId::WaterCenter
                };
                tile_grid.set([x * 2, y * 2 + 1], Some(botleft));

                let topleft = if land_n && land_w {
                    TileId::WaterCornerNW
                } else if land_n {
                    TileId::WaterEdgeNorth
                } else if land_w {
                    TileId::WaterEdgeWest
                } else if land_nw {
                    TileId::WaterConcaveNW
                } else {
                    TileId::WaterCenter
                };
                tile_grid.set([x * 2, y * 2], Some(topleft));
            } else {
                tile_grid.set_area(
                    CellRect {
                        position: [x * 2, y * 2],
                        size: [2, 2],
                    },
                    Some(TileId::Ground),
                );
            }
        }
    }
    tile_grid
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TileId {
    Ground,
    WaterCenter,
    WaterEdgeNorth,
    WaterCornerNE,
    WaterEdgeEast,
    WaterCornerSE,
    WaterEdgeSouth,
    WaterCornerSW,
    WaterEdgeWest,
    WaterCornerNW,
    WaterConcaveNE,
    WaterConcaveSE,
    WaterConcaveSW,
    WaterConcaveNW,
}
