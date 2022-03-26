use std::time::Duration;

use crate::entities::{
    ActionType, Entity, EntityConfig, EntitySprite, PhysicalTypeConfig, Team, NUM_UNIT_ACTIONS,
};
use rand::Rng;

#[derive(Debug, PartialEq)]
pub enum MapType {
    Empty,
    Small,
    LoadTest,
}

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum EntityType {
    SquareUnit,
    SmallBuilding,
    CircleUnit,
    LargeBuilding,
}

pub struct Map {
    pub dimensions: [u32; 2],
    pub entities: Vec<Entity>,
}

impl Map {
    pub fn new(map_type: MapType) -> Self {
        let mut entities = vec![
            create_entity(EntityType::SquareUnit, [4, 4], Team::Player),
            create_entity(EntityType::SmallBuilding, [2, 1], Team::Player),
            create_entity(EntityType::LargeBuilding, [1, 7], Team::Player),
        ];

        if map_type != MapType::Empty {
            let neutral_entity = Entity::new(
                EntityConfig {
                    name: "Neutral entity",
                    is_solid: false,
                    sprite: EntitySprite::Neutral,
                    max_health: Some(5),
                    physical_type: PhysicalTypeConfig::StructureSize([1, 1]), //TODO
                },
                [1, 3],
                Team::Neutral,
                [None; NUM_UNIT_ACTIONS],
            );
            entities.push(neutral_entity);
        }

        match map_type {
            MapType::Empty => {
                let dimensions = [30, 20];
                Self {
                    dimensions,
                    entities,
                }
            }
            MapType::Small => {
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
                let dimensions = [50, 25];
                for y in 5..dimensions[1] {
                    for x in 5..dimensions[0] {
                        if rng.gen_bool(0.6) {
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
    let (config, actions) = match entity_type {
        EntityType::SquareUnit => (
            EntityConfig {
                name: "Square",
                is_solid: true,
                sprite: EntitySprite::SquareUnit,
                max_health: Some(3),
                physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(600)),
            },
            [
                Some(ActionType::Move),
                Some(ActionType::Harm),
                Some(ActionType::Heal),
            ],
        ),
        EntityType::CircleUnit => (
            EntityConfig {
                name: "Circle",
                is_solid: true,
                sprite: EntitySprite::CircleUnit,
                max_health: Some(2),
                physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(800)),
            },
            [Some(ActionType::Move), Some(ActionType::Harm), None],
        ),
        EntityType::SmallBuilding => (
            EntityConfig {
                name: "Small building",
                is_solid: true,
                sprite: EntitySprite::SmallBuilding,
                max_health: Some(3),
                physical_type: PhysicalTypeConfig::StructureSize([2, 2]),
            },
            [Some(ActionType::Train(EntityType::SquareUnit)), None, None],
        ),
        EntityType::LargeBuilding => (
            EntityConfig {
                name: "Large building",
                is_solid: true,
                sprite: EntitySprite::LargeBuilding,
                max_health: Some(5),
                physical_type: PhysicalTypeConfig::StructureSize([3, 2]),
            },
            [
                Some(ActionType::Train(EntityType::CircleUnit)),
                Some(ActionType::Train(EntityType::SquareUnit)),
                None,
            ],
        ),
    };
    Entity::new(config, position, team, actions)
}
