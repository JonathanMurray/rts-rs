use std::time::Duration;

use crate::entities::{Entity, EntitySprite, Team, TrainingActionComponent};
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
        let player_entity_1 = create_player_entity_1([0, 0]);
        let player_entity_2 = Entity::new(
            "Player building",
            [1, 0],
            true,
            None,
            Team::Player,
            EntitySprite::Player2,
            Some(3),
            Some(TrainingActionComponent::new()),
        );

        let mut entities = vec![];

        if map_type != MapType::Empty {
            let neutral_entity = Entity::new(
                "Neutral entity",
                [2, 0],
                false,
                None,
                Team::Neutral,
                EntitySprite::Neutral,
                Some(5),
                None,
            );
            entities.push(neutral_entity);
        }

        entities.push(player_entity_1);
        entities.push(player_entity_2);

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
                let dimensions = (30, 20);
                for y in 1..dimensions.1 {
                    for x in 0..dimensions.0 {
                        if rng.gen_bool(0.8) {
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
        "Enemy unit",
        position,
        true,
        Some(Duration::from_millis(800)),
        Team::Enemy,
        EntitySprite::Enemy,
        Some(1),
        None,
    )
}

pub fn create_player_entity_1(position: [u32; 2]) -> Entity {
    Entity::new(
        "Player unit",
        position,
        true,
        Some(Duration::from_millis(600)),
        Team::Player,
        EntitySprite::Player,
        Some(2),
        None,
    )
}
