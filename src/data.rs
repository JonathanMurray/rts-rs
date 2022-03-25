use std::time::Duration;

use crate::entities::{
    Entity, EntityConfig, EntitySprite, MobileOrStructureConfig, Team, TrainingActionComponent,
};
use rand::Rng;

#[derive(Debug, PartialEq)]
pub enum MapType {
    Empty,
    Small,
    LoadTest,
}

pub struct Map {
    pub dimensions: (u32, u32),
    pub entities: Vec<Entity>,
}

impl Map {
    pub fn new(map_type: MapType) -> Self {
        let player_unit = create_player_unit([0, 0]);
        let player_building = Entity::new(
            EntityConfig {
                name: "Player building",
                is_solid: true,
                sprite: EntitySprite::PlayerBuilding,
                max_health: Some(3),
            },
            [1, 0],
            MobileOrStructureConfig::StructureSize([2, 2]),
            Team::Player,
            Some(TrainingActionComponent::new()),
        );

        let mut entities = vec![];

        if map_type != MapType::Empty {
            let neutral_entity = Entity::new(
                EntityConfig {
                    name: "Neutral entity",
                    is_solid: false,
                    sprite: EntitySprite::Neutral,
                    max_health: Some(5),
                },
                [4, 0],
                MobileOrStructureConfig::StructureSize([1, 1]), //TODO
                Team::Neutral,
                None,
            );
            entities.push(neutral_entity);
        }

        entities.push(player_unit);
        entities.push(player_building);

        match map_type {
            MapType::Empty => {
                let dimensions = (12, 12);
                Self {
                    dimensions,
                    entities,
                }
            }
            MapType::Small => {
                let dimensions = (12, 12);
                entities.push(create_enemy_entity([5, 2]));
                entities.push(create_enemy_entity([3, 0]));
                entities.push(create_enemy_entity([0, 4]));
                entities.push(create_enemy_entity([3, 4]));
                Self {
                    dimensions,
                    entities,
                }
            }
            MapType::LoadTest => {
                let mut rng = rand::thread_rng();
                let dimensions = (50, 50);
                for y in 2..dimensions.1 {
                    for x in 0..dimensions.0 {
                        if rng.gen_bool(0.6) {
                            entities.push(create_enemy_entity([x, y]));
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

fn create_enemy_entity(position: [u32; 2]) -> Entity {
    Entity::new(
        EntityConfig {
            name: "Enemy unit",
            is_solid: true,
            sprite: EntitySprite::Enemy,
            max_health: Some(1),
        },
        position,
        MobileOrStructureConfig::MovementCooldown(Duration::from_millis(800)),
        Team::Enemy,
        None,
    )
}

pub fn create_player_unit(position: [u32; 2]) -> Entity {
    Entity::new(
        EntityConfig {
            name: "Player unit",
            is_solid: true,
            sprite: EntitySprite::PlayerUnit,
            max_health: Some(2),
        },
        position,
        MobileOrStructureConfig::MovementCooldown(Duration::from_millis(600)),
        Team::Player,
        None,
    )
}
