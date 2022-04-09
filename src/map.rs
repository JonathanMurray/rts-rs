use rand::Rng;

use crate::data::{self, EntityType};
use crate::entities::{Entity, Team};
use crate::grid::Grid;
use data::create_entity;
use ggez::Context;
use std::io::Read;
use std::path::Path;

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

        let mut entities = vec![
            data::create_entity(EntityType::Worker, [6, 2], Team::Player),
            data::create_entity(EntityType::Fighter, [8, 2], Team::Player),
            data::create_entity(EntityType::Townhall, [1, 7], Team::Player),
        ];

        entities.push(data::create_entity(
            EntityType::Resource,
            [6, 4],
            Team::Neutral,
        ));

        match map_type {
            MapType::Empty => {}
            MapType::Small => {
                entities.push(data::create_entity(EntityType::Worker, [7, 7], Team::Enemy));
                entities.push(data::create_entity(
                    EntityType::Townhall,
                    [6, 8],
                    Team::Enemy,
                ));
            }
            MapType::Medium => {
                entities.push(data::create_entity(EntityType::Worker, [5, 2], Team::Enemy));
                entities.push(data::create_entity(EntityType::Worker, [3, 0], Team::Enemy));
                entities.push(data::create_entity(EntityType::Worker, [0, 4], Team::Enemy));
                entities.push(data::create_entity(EntityType::Worker, [3, 4], Team::Enemy));
                entities.push(data::create_entity(
                    EntityType::Townhall,
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
        }
    }

    pub fn load_from_file(ctx: &mut Context, path: impl AsRef<Path>) -> Self {
        let mut file = ggez::filesystem::open(ctx, path).unwrap();
        let mut map = String::new();
        file.read_to_string(&mut map).unwrap();
        let rows: Vec<&str> = map.lines().collect();
        let w = rows[0].len() as u32;
        let h = rows.len() as u32;
        for line in &rows {
            assert_eq!(line.len(), w as usize);
        }

        let mut entities = Vec::new();
        let mut water_grid = Grid::new([w, h]);
        for x in 0..w {
            for y in 0..h {
                let ch = rows[y as usize].as_bytes()[x as usize] as char;
                match ch {
                    'W' => {
                        water_grid.set([x as u32, y as u32], Some(()));
                    }
                    '1' => {
                        entities.push(create_entity(EntityType::Townhall, [x, y], Team::Player));
                    }
                    '2' => {
                        entities.push(create_entity(EntityType::Townhall, [x, y], Team::Enemy));
                    }
                    'R' => {
                        entities.push(create_entity(EntityType::Resource, [x, y], Team::Neutral));
                    }
                    _ => {}
                }
            }
        }

        Self {
            dimensions: [w as u32, h as u32],
            entities,
            water_grid,
        }
    }
}
