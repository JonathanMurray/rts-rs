use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

use ggez::graphics::{Color, DrawMode, Font, Mesh, Rect, Text};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

use crate::entities::{
    Action, Entity, EntityConfig, EntitySprite, PhysicalTypeConfig, Team, TrainingConfig,
    NUM_ENTITY_ACTIONS,
};
use crate::hud_graphics::entity_portrait::PORTRAIT_DIMENSIONS;
use crate::hud_graphics::DrawableWithDebug;

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
    Fighter,
    Worker,
    Barracks,
    Townhall,
}

pub struct WorldInitData {
    pub dimensions: [u32; 2],
    pub entities: Vec<Entity>,
}

impl WorldInitData {
    pub fn new(map_type: MapType) -> Self {
        let mut entities = vec![
            create_entity(EntityType::Worker, [6, 2], Team::Player),
            create_entity(EntityType::Fighter, [8, 2], Team::Player),
            create_entity(EntityType::Townhall, [1, 7], Team::Player),
        ];

        entities.push(create_entity(EntityType::Resource, [6, 4], Team::Neutral));

        match map_type {
            MapType::Empty => Self {
                dimensions: [30, 20],
                entities,
            },
            MapType::Small => {
                entities.push(create_entity(EntityType::Worker, [7, 7], Team::Enemy));
                entities.push(create_entity(EntityType::Townhall, [6, 8], Team::Enemy));
                Self {
                    dimensions: [30, 20],
                    entities,
                }
            }
            MapType::Medium => {
                let dimensions = [30, 20];

                entities.push(create_entity(EntityType::Worker, [5, 2], Team::Enemy));
                entities.push(create_entity(EntityType::Worker, [3, 0], Team::Enemy));
                entities.push(create_entity(EntityType::Worker, [0, 4], Team::Enemy));
                entities.push(create_entity(EntityType::Worker, [3, 4], Team::Enemy));
                entities.push(create_entity(EntityType::Townhall, [8, 4], Team::Enemy));
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
                                EntityType::Worker
                            } else {
                                EntityType::Fighter
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
    let structure_types = [EntityType::Barracks, EntityType::Townhall];
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
        EntityType::Fighter => EntityConfig {
            is_solid: true,
            sprite: EntitySprite::Fighter,
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
        EntityType::Worker => EntityConfig {
            is_solid: true,
            sprite: EntitySprite::Worker,
            max_health: Some(5),
            physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(900)),
            actions: [
                Some(Action::Move),
                Some(Action::GatherResource),
                Some(Action::ReturnResource),
                Some(Action::Construct(EntityType::Barracks)),
                Some(Action::Construct(EntityType::Townhall)),
                None,
            ],
        },
        EntityType::Barracks => EntityConfig {
            is_solid: true,
            sprite: EntitySprite::Barracks,
            max_health: Some(3),
            physical_type: PhysicalTypeConfig::StructureSize([2, 2]),
            actions: [
                Some(Action::Train(
                    EntityType::Fighter,
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
        EntityType::Townhall => EntityConfig {
            is_solid: true,
            sprite: EntitySprite::Townhall,
            max_health: Some(5),
            physical_type: PhysicalTypeConfig::StructureSize([3, 2]),
            actions: [
                Some(Action::Train(
                    EntityType::Worker,
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
            sprite: EntitySprite::Resource,
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

pub struct ActionHudConfig {
    pub text: String,
    pub icon: Box<dyn DrawableWithDebug>,
    pub keycode: KeyCode,
}

pub struct HudAssets {
    font: Font,
    fighter: EntityHudConfig,
    worker: EntityHudConfig,
    barracks: EntityHudConfig,
    townhall: EntityHudConfig,
    resource: EntityHudConfig,
}

impl HudAssets {
    pub fn new(ctx: &mut Context, font: Font) -> GameResult<Self> {
        let color = Color::new(0.6, 0.6, 0.6, 1.0);
        Ok(Self {
            font,
            fighter: EntityHudConfig {
                name: "Fighter".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(
                        5.0,
                        5.0,
                        PORTRAIT_DIMENSIONS[0] - 10.0,
                        PORTRAIT_DIMENSIONS[1] - 10.0,
                    ),
                    color,
                )?,
            },
            worker: EntityHudConfig {
                name: "Worker".to_string(),
                portrait: Mesh::new_circle(
                    ctx,
                    DrawMode::fill(),
                    [
                        (PORTRAIT_DIMENSIONS[0]) / 2.0,
                        (PORTRAIT_DIMENSIONS[1]) / 2.0,
                    ],
                    (PORTRAIT_DIMENSIONS[0] - 10.0) / 2.0,
                    0.001,
                    color,
                )?,
            },
            barracks: EntityHudConfig {
                name: "Barracks".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(
                        5.0,
                        5.0,
                        PORTRAIT_DIMENSIONS[0] - 10.0,
                        PORTRAIT_DIMENSIONS[1] - 10.0,
                    ),
                    color,
                )?,
            },
            townhall: EntityHudConfig {
                name: "Townhall".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(
                        5.0,
                        5.0,
                        PORTRAIT_DIMENSIONS[0] - 10.0,
                        (PORTRAIT_DIMENSIONS[1] - 10.0) * 0.85,
                    ),
                    color,
                )?,
            },
            resource: EntityHudConfig {
                name: "Resource location".to_string(),
                portrait: Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(
                        5.0,
                        5.0,
                        PORTRAIT_DIMENSIONS[0] - 10.0,
                        PORTRAIT_DIMENSIONS[1] - 10.0,
                    ),
                    color,
                )?,
            },
        })
    }

    pub fn entity(&self, entity_type: EntityType) -> &EntityHudConfig {
        match entity_type {
            EntityType::Fighter => &self.fighter,
            EntityType::Worker => &self.worker,
            EntityType::Barracks => &self.barracks,
            EntityType::Townhall => &self.townhall,
            EntityType::Resource => &self.resource,
        }
    }

    pub fn action(&self, action: Action) -> ActionHudConfig {
        let font_size = 30.0;

        // TODO: mind the allocations

        match action {
            Action::Train(entity_type, training_config) => {
                let unit_config = self.entity(entity_type);
                let keycode = match entity_type {
                    EntityType::Worker => KeyCode::C,
                    EntityType::Fighter => KeyCode::S,
                    _ => panic!("No keycode for training: {:?}", entity_type),
                };
                ActionHudConfig {
                    text: format!(
                        "Train {} [cost {}, {}s]",
                        &unit_config.name,
                        training_config.cost,
                        training_config.duration.as_secs()
                    ),
                    icon: Box::new(unit_config.portrait.clone()),
                    keycode,
                }
            }
            Action::Construct(structure_type) => {
                let keycode = match structure_type {
                    EntityType::Barracks => KeyCode::S,
                    EntityType::Townhall => KeyCode::L,
                    _ => panic!("No keycode for constructing: {:?}", structure_type),
                };
                let structure_config = self.entity(structure_type);
                ActionHudConfig {
                    text: format!("Construct {}", &structure_config.name),
                    icon: Box::new(structure_config.portrait.clone()),
                    keycode,
                }
            }
            Action::Move => ActionHudConfig {
                text: "Move".to_owned(),
                icon: Box::new(Text::new(("M", self.font, font_size))),
                keycode: KeyCode::M,
            },
            Action::Attack => ActionHudConfig {
                text: "Attack".to_owned(),
                icon: Box::new(Text::new(("A", self.font, font_size))),
                keycode: KeyCode::A,
            },
            Action::GatherResource => ActionHudConfig {
                text: "Gather".to_owned(),
                icon: Box::new(Text::new(("G", self.font, font_size))),
                keycode: KeyCode::G,
            },
            Action::ReturnResource => ActionHudConfig {
                text: "Return".to_owned(),
                icon: Box::new(Text::new(("R", self.font, font_size))),
                keycode: KeyCode::R,
            },
        }
    }
}
