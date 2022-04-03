use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

use crate::entities::{
    Action, Entity, EntityConfig, EntitySprite, PhysicalTypeConfig, Team, TrainingConfig,
    NUM_ENTITY_ACTIONS,
};
use ggez::graphics::{Color, DrawMode, Mesh, Rect};
use ggez::{Context, GameResult};

#[derive(Debug, PartialEq)]
pub enum MapType {
    Empty,
    Small,
    Medium,
    LoadTest,
}

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum EntityType {
    Resource,
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
            create_entity(EntityType::CircleUnit, [6, 2], Team::Player),
            create_entity(EntityType::SquareUnit, [8, 2], Team::Player),
            create_entity(EntityType::LargeBuilding, [1, 7], Team::Player),
        ];

        entities.push(create_entity(EntityType::Resource, [6, 4], Team::Neutral));

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
    Entity::new(entity_type, config, position, team)
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
        EntityType::Resource => EntityConfig {
            is_solid: true,
            sprite: EntitySprite::Neutral,
            max_health: None,
            physical_type: PhysicalTypeConfig::StructureSize([1, 1]),
            actions: [None; NUM_ENTITY_ACTIONS],
        },
    }
}

pub struct EntityHudConfig {
    pub name: String,
    pub portrait: Mesh,
}

pub struct HudAssets {
    square_unit: EntityHudConfig,
    circle_unit: EntityHudConfig,
    small_building: EntityHudConfig,
    large_building: EntityHudConfig,
    resource: EntityHudConfig,
}

impl HudAssets {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let color = Color::new(0.6, 0.6, 0.6, 1.0);
        Ok(Self {
            square_unit: EntityHudConfig {
                name: "Square unit".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(0.0, 0.0, 50.0, 50.0),
                    color,
                )?,
            },
            circle_unit: EntityHudConfig {
                name: "Circle unit".to_string(),
                portrait: Mesh::new_circle(
                    ctx,
                    DrawMode::fill(),
                    [25.0, 25.0],
                    25.0,
                    0.001,
                    color,
                )?,
            },
            small_building: EntityHudConfig {
                name: "Small building".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(0.0, 0.0, 50.0, 50.0),
                    color,
                )?,
            },
            large_building: EntityHudConfig {
                name: "Large building".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(0.0, 10.0, 50.0, 35.0),
                    color,
                )?,
            },
            resource: EntityHudConfig {
                name: "Resource location".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(0.0, 0.0, 50.0, 50.0),
                    color,
                )?,
            },
        })
    }

    pub fn get(&self, entity_type: EntityType) -> &EntityHudConfig {
        match entity_type {
            EntityType::SquareUnit => &self.square_unit,
            EntityType::CircleUnit => &self.circle_unit,
            EntityType::SmallBuilding => &self.small_building,
            EntityType::LargeBuilding => &self.large_building,
            EntityType::Resource => &self.resource,
        }
    }
}
