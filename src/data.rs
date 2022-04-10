use std::collections::HashMap;
use std::time::Duration;

use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, Drawable, Image, Mesh, MeshBuilder, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

use crate::entities::{
    Action, Entity, EntityConfig, EntitySprite, PhysicalTypeConfig, Team, TrainingConfig,
    NUM_ENTITY_ACTIONS,
};
use crate::game::CELL_PIXEL_SIZE;
use crate::hud_graphics::entity_portrait::PORTRAIT_DIMENSIONS;
use crate::images;
use crate::text::SharpFont;

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum EntityType {
    Resource,
    Fighter,
    Worker,
    Barracks,
    Townhall,
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
    pub icon: Box<dyn Drawable>,
    pub keycode: KeyCode,
}

pub struct HudAssets {
    fighter: EntityHudConfig,
    worker: EntityHudConfig,
    barracks: EntityHudConfig,
    townhall: EntityHudConfig,
    resource: EntityHudConfig,
    move_icon: Image,
    attack_icon: Image,
    gather_icon: Image,
    return_icon: Image,
}

impl HudAssets {
    pub fn new(ctx: &mut Context, font: SharpFont) -> GameResult<Self> {
        let color = Color::new(0.6, 0.6, 0.6, 1.0);
        let font_size = 15.0;
        let move_text = font.text(font_size, "M");
        let attack_text = font.text(font_size, "A");
        let gather_text = font.text(font_size, "G");
        let return_text = font.text(font_size, "R");

        let move_dimensions = move_text.dimensions(ctx);
        let move_icon = images::drawable_into_image(ctx, move_dimensions, |ctx| {
            move_text.draw(ctx, [0.0, 0.0])
        })?;
        let attack_dimensions = attack_text.dimensions(ctx);
        let attack_icon = images::drawable_into_image(ctx, attack_dimensions, |ctx| {
            attack_text.draw(ctx, [0.0, 0.0])
        })?;
        let gather_dimensions = gather_text.dimensions(ctx);
        let gather_icon = images::drawable_into_image(ctx, gather_dimensions, |ctx| {
            gather_text.draw(ctx, [0.0, 0.0])
        })?;
        let return_dimensions = return_text.dimensions(ctx);
        let return_icon = images::drawable_into_image(ctx, return_dimensions, |ctx| {
            return_text.draw(ctx, [0.0, 0.0])
        })?;
        Ok(Self {
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
            move_icon,
            attack_icon,
            gather_icon,
            return_icon,
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
        // TODO: mind the allocations

        match action {
            Action::Train(entity_type, training_config) => {
                let unit_config = self.entity(entity_type);
                let keycode = match entity_type {
                    EntityType::Worker => KeyCode::W,
                    EntityType::Fighter => KeyCode::F,
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
                    EntityType::Barracks => KeyCode::B,
                    EntityType::Townhall => KeyCode::T,
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
                icon: Box::new(self.move_icon.clone()),
                keycode: KeyCode::M,
            },
            Action::Attack => ActionHudConfig {
                text: "Attack".to_owned(),
                icon: Box::new(self.attack_icon.clone()),
                keycode: KeyCode::A,
            },
            Action::GatherResource => ActionHudConfig {
                text: "Gather".to_owned(),
                icon: Box::new(self.gather_icon.clone()),
                keycode: KeyCode::G,
            },
            Action::ReturnResource => ActionHudConfig {
                text: "Return".to_owned(),
                icon: Box::new(self.return_icon.clone()),
                keycode: KeyCode::R,
            },
        }
    }
}

pub fn create_entity_sprites(
    ctx: &mut Context,
) -> GameResult<HashMap<(EntitySprite, Team), SpriteBatch>> {
    let mut sprite_batches = Default::default();
    create_fighter(ctx, &mut sprite_batches)?;
    create_worker(ctx, &mut sprite_batches)?;
    create_barracks(ctx, &mut sprite_batches)?;
    create_townhall(ctx, &mut sprite_batches)?;
    create_resource(ctx, &mut sprite_batches)?;

    Ok(sprite_batches)
}

fn create_fighter(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let size = [CELL_PIXEL_SIZE[0] * 0.7, CELL_PIXEL_SIZE[1] * 0.8];
    let rect = Rect::new(
        (CELL_PIXEL_SIZE[0] - size[0]) / 2.0,
        (CELL_PIXEL_SIZE[1] - size[1]) / 2.0,
        size[0],
        size[1],
    );
    let colors = HashMap::from([
        (Team::Player, Color::new(0.6, 0.8, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.8, 0.4, 0.4, 1.0)),
    ]);
    for (team, color) in colors {
        let mesh = MeshBuilder::new()
            .rounded_rectangle(DrawMode::fill(), rect, 5.0, color)?
            .build(ctx)?;
        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Fighter, team), batch);
    }
    Ok(())
}

fn create_worker(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let colors = HashMap::from([
        (Team::Player, Color::new(0.6, 0.8, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.8, 0.4, 0.4, 1.0)),
    ]);
    for (team, color) in colors {
        let mesh = MeshBuilder::new()
            .circle(
                DrawMode::fill(),
                [CELL_PIXEL_SIZE[0] / 2.0, CELL_PIXEL_SIZE[1] / 2.0],
                CELL_PIXEL_SIZE[0] * 0.35,
                0.05,
                color,
            )?
            .build(ctx)?;
        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Worker, team), batch);
    }
    Ok(())
}

fn create_barracks(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let colors = HashMap::from([
        (Team::Player, Color::new(0.6, 0.8, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.8, 0.4, 0.4, 1.0)),
    ]);
    for (team, color) in colors {
        let size = [CELL_PIXEL_SIZE[0] * 1.9, CELL_PIXEL_SIZE[1] * 1.9];
        let mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    (CELL_PIXEL_SIZE[0] * 2.0 - size[0]) / 2.0,
                    (CELL_PIXEL_SIZE[1] * 2.0 - size[1]) / 2.0,
                    size[0],
                    size[1],
                ),
                color,
            )?
            .rectangle(
                DrawMode::stroke(2.0),
                Rect::new(
                    CELL_PIXEL_SIZE[0] * 0.75,
                    CELL_PIXEL_SIZE[1] * 0.5,
                    CELL_PIXEL_SIZE[0] * 0.5,
                    CELL_PIXEL_SIZE[1] * 0.5,
                ),
                Color::new(0.0, 0.0, 0.0, 1.0),
            )?
            .build(ctx)?;

        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Barracks, team), batch);
    }
    Ok(())
}

fn create_townhall(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let colors = HashMap::from([
        (Team::Player, Color::new(0.5, 0.7, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.7, 0.3, 0.3, 1.0)),
    ]);
    for (team, color) in colors {
        let size = [CELL_PIXEL_SIZE[0] * 2.9, CELL_PIXEL_SIZE[1] * 1.9];
        let mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    (CELL_PIXEL_SIZE[0] * 3.0 - size[0]) / 2.0,
                    (CELL_PIXEL_SIZE[1] * 2.0 - size[1]) / 2.0,
                    size[0],
                    size[1],
                ),
                color,
            )?
            .circle(
                DrawMode::stroke(4.0),
                [CELL_PIXEL_SIZE[0] * 1.5, CELL_PIXEL_SIZE[1] * 0.7],
                CELL_PIXEL_SIZE[0] * 0.4,
                0.05,
                Color::new(0.0, 0.0, 0.0, 1.0),
            )?
            .build(ctx)?;

        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Townhall, team), batch);
    }
    Ok(())
}

fn create_resource(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let size = [CELL_PIXEL_SIZE[0] * 0.7, CELL_PIXEL_SIZE[1] * 0.8];
    let mesh = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE[0] - size[0]) / 2.0,
                (CELL_PIXEL_SIZE[1] - size[1]) / 2.0,
                size[0],
                size[1],
            ),
            Color::new(0.8, 0.6, 0.2, 1.0),
        )?
        .build(ctx)?;

    let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
    sprite_batches.insert((EntitySprite::Resource, Team::Neutral), batch);
    Ok(())
}
