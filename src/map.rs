use rand::Rng;

use crate::data::{self, EntityType};
use crate::entities::{Entity, Team};
use crate::grid::Grid;

#[derive(Debug, PartialEq)]
pub enum MapType {
    Empty,
    Small,
    Medium,
    LoadTest,
}

pub struct WorldInitData {
    pub dimensions: [u32; 2],
    pub entities: Vec<Entity>,
    pub water_grid: Grid<()>,
}

impl WorldInitData {
    pub fn new(map_type: MapType) -> Self {
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
}
