use std::time::Duration;

use crate::entities::{
    ActionType, Entity, EntityConfig, EntitySprite, PhysicalTypeConfig, Team,
    TrainingActionComponent,
};
use rand::Rng;

#[derive(Debug, PartialEq)]
pub enum MapType {
    Empty,
    Small,
    LoadTest,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum EntityType {
    SquareUnit,
    PlayerBuilding,
    CircleUnit,
    EnemyBuilding,
}

pub struct Map {
    pub dimensions: [u32; 2],
    pub entities: Vec<Entity>,
}

impl Map {
    pub fn new(map_type: MapType) -> Self {
        let player_unit = create_entity(EntityType::SquareUnit, [4, 4], Team::Player);
        let player_building = Entity::new(
            EntityConfig {
                name: "Player building",
                is_solid: true,
                sprite: EntitySprite::PlayerBuilding,
                max_health: Some(3),
                physical_type: PhysicalTypeConfig::StructureSize([2, 2]),
            },
            [2, 1],
            Team::Player,
            Some(TrainingActionComponent::new(EntityType::SquareUnit)),
            [None, None],
        );

        let mut entities = vec![];

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
                None,
                [None, None],
            );
            entities.push(neutral_entity);
        }

        entities.push(player_unit);
        entities.push(player_building);

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
                entities.push(create_enemy_building([8, 4]));
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

fn create_enemy_building(position: [u32; 2]) -> Entity {
    Entity::new(
        EntityConfig {
            name: "Enemy building",
            is_solid: true,
            sprite: EntitySprite::EnemyBuilding,
            max_health: Some(2),
            physical_type: PhysicalTypeConfig::StructureSize([3, 2]),
        },
        position,
        Team::Enemy,
        Some(TrainingActionComponent::new(EntityType::CircleUnit)),
        [None, None],
    )
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
            [Some(ActionType::SelfHarm), Some(ActionType::Heal)],
        ),
        EntityType::CircleUnit => (
            EntityConfig {
                name: "Circle",
                is_solid: true,
                sprite: EntitySprite::CircleUnit,
                max_health: Some(2),
                physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(800)),
            },
            [Some(ActionType::SelfHarm), None],
        ),
        _ => panic!("Unhandled entity type: {:?}", entity_type),
    };
    Entity::new(config, position, team, None, actions)
}
