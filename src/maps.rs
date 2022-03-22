use std::time::Duration;

use crate::entities::{Entity, EntitySprite, HealthComponent, Team};
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
        let player_entity = Entity::new(
            [0, 0],
            true,
            Some(Duration::from_millis(400)),
            Team::Player,
            EntitySprite::Player,
            None,
        );

        let mut entities = vec![];

        if map_type != MapType::Empty {
            let neutral_entity = Entity::new(
                [2, 0],
                false,
                None,
                Team::Neutral,
                EntitySprite::Neutral,
                Some(HealthComponent::new(5)),
            );
            entities.push(neutral_entity);
        }

        entities.push(player_entity);

        match map_type {
            MapType::Empty => {
                let dimensions = (8, 8);
                Self {
                    dimensions,
                    entities,
                }
            }
            MapType::Small => {
                let dimensions = (8, 8);
                entities.push(enemy_entity([5, 2]));
                entities.push(enemy_entity([3, 0]));
                entities.push(enemy_entity([0, 4]));
                entities.push(enemy_entity([3, 4]));
                Self {
                    dimensions,
                    entities,
                }
            }
            MapType::LoadTest => {
                let mut rng = rand::thread_rng();
                let dimensions = (30, 20);
                for y in 1..dimensions.1 {
                    for x in 0..dimensions.0 {
                        if rng.gen_bool(0.8) {
                            entities.push(enemy_entity([x, y]));
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

fn enemy_entity(position: [u32; 2]) -> Entity {
    Entity::new(
        position,
        true,
        Some(Duration::from_millis(800)),
        Team::Enemy,
        EntitySprite::Enemy,
        None,
    )
}
