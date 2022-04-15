use std::collections::HashMap;
use std::time::Duration;

use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Image, Mesh, MeshBuilder, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

use crate::entities::{
    Action, AnimationState, Direction, Entity, EntityConfig, PhysicalTypeConfig, Team,
    TrainingConfig, NUM_ENTITY_ACTIONS,
};
use crate::game::CELL_PIXEL_SIZE;
use crate::hud_graphics::entity_portrait::PORTRAIT_DIMENSIONS;
use crate::images;

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
            max_health: Some(3),
            physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(600)),
            actions: [
                Some(Action::Move),
                Some(Action::Stop),
                Some(Action::Attack),
                None,
                None,
                None,
            ],
        },
        EntityType::Worker => EntityConfig {
            is_solid: true,
            max_health: Some(5),
            physical_type: PhysicalTypeConfig::MovementCooldown(Duration::from_millis(900)),
            actions: [
                Some(Action::Move),
                Some(Action::Stop),
                Some(Action::GatherResource),
                Some(Action::ReturnResource),
                Some(Action::Construct(EntityType::Barracks)),
                Some(Action::Construct(EntityType::Townhall)),
            ],
        },
        EntityType::Barracks => EntityConfig {
            is_solid: true,
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
    stop_icon: Image,
    move_icon: Image,
    attack_icon: Image,
    gather_icon: Image,
    return_icon: Image,
}

impl HudAssets {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let color = Color::new(0.6, 0.6, 0.6, 1.0);

        let stop_icon = Image::new(ctx, "/images/icons/stop.png")?;
        let move_icon = Image::new(ctx, "/images/icons/move.png")?;
        let attack_icon = Image::new(ctx, "/images/icons/attack.png")?;
        let gather_icon = Image::new(ctx, "/images/icons/gather.png")?;
        let return_icon = Image::new(ctx, "/images/icons/return.png")?;

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
            stop_icon,
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
            Action::Stop => ActionHudConfig {
                text: "Stop".to_owned(),
                icon: Box::new(self.stop_icon.clone()),
                keycode: KeyCode::S,
            },
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
) -> GameResult<HashMap<(EntityType, Team), Animation>> {
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
    sprite_batches: &mut HashMap<(EntityType, Team), Animation>,
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
        let image = images::mesh_into_image(ctx, mesh)?;
        sprite_batches.insert((EntityType::Fighter, team), Animation::Static(image));
    }
    Ok(())
}

// Sprites must be designed with these reserved colors in mind.
// Pixels that use these exact color are changed to an appropriate team color.
const TEMPLATE_COLOR_LIGHT: [u8; 4] = [122, 171, 255, 255];
const TEMPLATE_COLOR_DARK: [u8; 4] = [99, 155, 255, 255];

const TEAM_COLOR_FAMILIES: [(Team, EntityColorFamily); 2] = [
    (
        Team::Player,
        EntityColorFamily {
            light: [120, 200, 120, 255],
            dark: [100, 180, 100, 255],
        },
    ),
    (
        Team::Enemy,
        EntityColorFamily {
            light: [200, 120, 120, 255],
            dark: [180, 100, 100, 255],
        },
    ),
];

struct EntityColorFamily {
    light: [u8; 4],
    dark: [u8; 4],
}

fn create_worker(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let image = Image::new(ctx, "/images/worker_sheet.png")?;
    let rgba = image.to_rgba8(ctx)?;
    for (team, color_family) in TEAM_COLOR_FAMILIES {
        let mut recolored = Vec::with_capacity(rgba.len());

        let mut i = 0;
        while i <= rgba.len() - 4 {
            let mut color = &rgba[i..i + 4];
            if color == &TEMPLATE_COLOR_LIGHT[..] {
                color = &color_family.light[..];
            } else if color == &TEMPLATE_COLOR_DARK[..] {
                color = &color_family.dark[..];
            }
            recolored.extend_from_slice(color);
            i += 4;
        }
        let team_image = Image::from_rgba8(ctx, image.width(), image.height(), &recolored[..])?;

        let mut frames = HashMap::new();

        let directions_per_row = [
            Direction::South,
            Direction::SouthEast,
            Direction::East,
            Direction::NorthEast,
            Direction::North,
            Direction::NorthWest,
            Direction::West,
            Direction::SouthWest,
        ];
        for (row, &direction) in directions_per_row.iter().enumerate() {
            frames.insert(
                direction,
                vec![
                    Frame::new(1.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
                    Frame::new(0.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
                    Frame::new(1.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
                    Frame::new(2.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
                ],
            );
        }

        // TODO specify idle-pose
        sprite_batches.insert(
            (EntityType::Worker, team),
            Animation::Tilesheet(Tilesheet {
                sheet: team_image,
                origin: [0.0, 16.0],
                frames,
            }),
        );
    }
    Ok(())
}

pub enum Animation {
    Tilesheet(Tilesheet),
    Static(Image),
}

impl Animation {
    pub fn draw(
        &self,
        ctx: &mut Context,
        animation: &AnimationState,
        direction: Direction,
        position_on_screen: [f32; 2],
    ) -> GameResult {
        match self {
            Animation::Tilesheet(tilesheet) => {
                tilesheet.draw(ctx, animation, direction, position_on_screen)
            }
            Animation::Static(image) => image.draw(ctx, DrawParam::new().dest(position_on_screen)),
        }
    }
}

pub struct Tilesheet {
    // Sheet contains multiple individual sprites
    sheet: Image,
    // origin y == 20, means that the top part of the sprite
    // will protrude 20 pixels above the cell that it occupies.
    origin: [f32; 2],
    frames: HashMap<Direction, Vec<Frame>>,
}

impl Tilesheet {
    pub fn draw(
        &self,
        ctx: &mut Context,
        animation: &AnimationState,
        direction: Direction,
        position_on_screen: [f32; 2],
    ) -> GameResult {
        let pos = [
            position_on_screen[0] - self.origin[0],
            position_on_screen[1] - self.origin[1],
        ];
        let frames = self
            .frames
            .get(&direction)
            .unwrap_or_else(|| self.frames.get(&Direction::South).unwrap());
        let frame_duration_ms = 150.0;
        let i = (animation.ms_counter as f32 / frame_duration_ms) as usize % frames.len();
        let frame = frames[i];
        self.sheet
            .draw(ctx, DrawParam::new().src(frame.src_rect).dest(pos))
    }
}

#[derive(Copy, Clone)]
struct Frame {
    // Which part of the sheet is used for this frame
    src_rect: Rect,
}

impl Frame {
    fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            src_rect: Rect::new(x, y, w, h),
        }
    }
}

fn create_barracks(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntityType, Team), Animation>,
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

        let image = images::mesh_into_image(ctx, mesh)?;
        sprite_batches.insert((EntityType::Barracks, team), Animation::Static(image));
    }
    Ok(())
}

fn create_townhall(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntityType, Team), Animation>,
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

        let image = images::mesh_into_image(ctx, mesh)?;
        sprite_batches.insert((EntityType::Townhall, team), Animation::Static(image));
    }
    Ok(())
}

fn create_resource(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntityType, Team), Animation>,
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

    let image = images::mesh_into_image(ctx, mesh)?;
    sprite_batches.insert(
        (EntityType::Resource, Team::Neutral),
        Animation::Static(image),
    );
    Ok(())
}
