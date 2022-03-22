use std::time::Duration;

use crate::entities::{Entity, EntitySprite, HealthComponent, PhysicsComponent, Team};
use rand::Rng;

#[derive(Debug)]
pub enum MapType {
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
            PhysicsComponent::new([0, 0], Duration::from_millis(400)),
            Team::Player,
            EntitySprite::Player,
            None,
        );

        let neutral_entity = Entity::new(
            PhysicsComponent::new([2, 0], Duration::from_millis(1000)),
            Team::Neutral,
            EntitySprite::Neutral,
            Some(HealthComponent::new(5)),
        );

        let mut entities = vec![player_entity, neutral_entity];

        match map_type {
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
        PhysicsComponent::new(position, Duration::from_millis(800)),
        Team::Enemy,
        EntitySprite::Enemy,
        None,
    )
}
