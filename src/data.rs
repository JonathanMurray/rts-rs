use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

use crate::entities::{
    Action, Entity, EntityConfig, EntitySprite, PhysicalTypeConfig, Team, TrainingConfig,
    NUM_ENTITY_ACTIONS,
};

#[derive(Debug, PartialEq)]
pub enum MapType {
    Empty,
    Small,
    Medium,
    LoadTest,
}

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum EntityType {
    SquareUnit,
    CircleUnit,
    SmallBuilding,
    LargeBuilding,
}

pub struct WorldInitData {
    pub dimensions: [u32; 2],
    pub entities: Vec<Entity>,
}

impl WorldInitData {
    pub fn new(map_type: MapType) -> Self {
        let mut entities = vec![
            create_entity(EntityType::SquareUnit, [4, 4], Team::Player),
            create_entity(EntityType::CircleUnit, [6, 2], Team::Player),
            create_entity(EntityType::SmallBuilding, [2, 1], Team::Player),
            create_entity(EntityType::LargeBuilding, [1, 7], Team::Player),
        ];

        if map_type != MapType::Empty {
            let neutral_entity = Entity::new(
                EntityConfig {
                    name: "Resource",
                    is_solid: true,
                    sprite: EntitySprite::Neutral,
                    max_health: None,
                    physical_type: PhysicalTypeConfig::StructureSize([1, 1]),
                    actions: [None; NUM_ENTITY_ACTIONS],
                },
                [6, 4],
                Team::Neutral,
            );
            entities.push(neutral_entity);
        }

        match map_type {
            MapType::Empty => Self {
                dimensions: [30, 20],
                entities,
            },
            MapType::Small => {
                entities.push(create_entity(EntityType::CircleUnit, [7, 7], Team::Enemy));
                entities.push(create_entity(
                    EntityType::LargeBuilding,
                    [6, 8],
                    Team::Enemy,
                ));
                Self {
                    dimensions: [30, 20],
                    entities,
                }
            }
            MapType::Medium => {
                let dimensions = [30, 20];

                entities.push(create_entity(EntityType::CircleUnit, [5, 2], Team::Enemy));
                entities.push(create_entity(EntityType::CircleUnit, [3, 0], Team::Enemy));
                entities.push(create_entity(EntityType::CircleUnit, [0, 4], Team::Enemy));
                entities.push(create_entity(EntityType::CircleUnit, [3, 4], Team::Enemy));
                entities.push(create_entity(
                    EntityType::LargeBuilding,
                    [8, 4],
                    Team::Enemy,
                ));
                Self {
                    dimensions,
                    entities,
                }
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
                                EntityType::CircleUnit
                            } else {
                                EntityType::SquareUnit
                            };
                            entities.push(create_entity(entity_type, [x, y], team));
                        }
                    }
                }
                Self {
                    dimensions,
                    entities,
                }
            }
        }
    }
}

pub fn create_entity(entity_type: EntityType, position: [u32; 2], team: Team) -> Entity {
    let config = entity_config(entity_type);
    Entity::new(config, position, team)
}

pub fn structure_sizes() -> HashMap<EntityType, [u32; 2]> {
    let mut map: HashMap<EntityType, [u32; 2]> = Default::default();
    let structure_types = [EntityType::SmallBuilding, EntityType::LargeBuilding];
    for structure_type in structure_types {
        let config = entity_config(structure_type);
        let size = match config.physical_type {
            PhysicalTypeConfig::MovementCooldown(_) => {
                panic!("{:?} is not a structure", structure_type)
            }
            PhysicalTypeConfig::StructureSize(size) => size,
        };
        map.insert(structure_type, size);
    }
    map
}

fn entity_config(entity_type: EntityType) -> EntityConfig {
    match entity_type {
        EntityType::SquareUnit => EntityConfig {
            name: "Square",
            is_solid: true,
            sprite: EntitySprite::SquareUnit,
            max_health: Some(3),
            physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(600)),
            actions: [
                Some(Action::Move),
                Some(Action::Attack),
                None,
                None,
                None,
                None,
            ],
        },
        EntityType::CircleUnit => EntityConfig {
            name: "Circle",
            is_solid: true,
            sprite: EntitySprite::CircleUnit,
            max_health: Some(5),
            physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(900)),
            actions: [
                Some(Action::Move),
                Some(Action::GatherResource),
                Some(Action::ReturnResource),
                Some(Action::Construct(EntityType::SmallBuilding)),
                Some(Action::Construct(EntityType::LargeBuilding)),
                None,
            ],
        },
        EntityType::SmallBuilding => EntityConfig {
            name: "Small building",
            is_solid: true,
            sprite: EntitySprite::SmallBuilding,
            max_health: Some(3),
            physical_type: PhysicalTypeConfig::StructureSize([2, 2]),
            actions: [
                Some(Action::Train(
                    EntityType::SquareUnit,
                    TrainingConfig {
                        duration: Duration::from_secs(7),
                        cost: 1,
                    },
                )),
                None,
                None,
                None,
                None,
                None,
            ],
        },
        EntityType::LargeBuilding => EntityConfig {
            name: "Large building",
            is_solid: true,
            sprite: EntitySprite::LargeBuilding,
            max_health: Some(5),
            physical_type: PhysicalTypeConfig::StructureSize([3, 2]),
            actions: [
                Some(Action::Train(
                    EntityType::CircleUnit,
                    TrainingConfig {
                        duration: Duration::from_secs(4),
                        cost: 1,
                    },
                )),
                None,
                None,
                None,
                None,
                None,
            ],
        },
    }
}
