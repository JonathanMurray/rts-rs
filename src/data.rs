use std::collections::HashMap;
use std::time::Duration;

use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Image, Mesh, MeshBuilder, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

use crate::entities::{
    Action, AnimationState, CategoryConfig, ConstructionConfig, Direction, Entity, EntityConfig,
    Team, TrainingConfig, NUM_ENTITY_ACTIONS,
};
use crate::game::CELL_PIXEL_SIZE;
use crate::hud_graphics::entity_portrait::PORTRAIT_DIMENSIONS;
use crate::images;

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum EntityType {
    FuelRift,
    Fighter,
    Worker,
    Barracks,
    TechLab,
}

pub fn create_entity(entity_type: EntityType, position: [u32; 2], team: Team) -> Entity {
    let config = entity_config(entity_type);
    Entity::new(entity_type, config, position, team)
}

pub fn structure_sizes() -> HashMap<EntityType, [u32; 2]> {
    let mut map: HashMap<EntityType, [u32; 2]> = Default::default();
    let structure_types = [EntityType::Barracks, EntityType::TechLab];
    for structure_type in structure_types {
        let config = entity_config(structure_type);
        let size = match config.category {
            CategoryConfig::StructureSize(size) => size,
            _ => {
                panic!("{:?} is not a structure", structure_type)
            }
        };
        map.insert(structure_type, size);
    }
    map
}

fn entity_config(entity_type: EntityType) -> EntityConfig {
    match entity_type {
        EntityType::Fighter => EntityConfig {
            max_health: Some(3),
            category: CategoryConfig::UnitMovementCooldown(Duration::from_millis(600)),
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
            max_health: Some(5),
            category: CategoryConfig::UnitMovementCooldown(Duration::from_millis(900)),
            actions: [
                Some(Action::Move),
                Some(Action::Stop),
                Some(Action::GatherResource),
                Some(Action::ReturnResource),
                Some(Action::Construct(
                    EntityType::Barracks,
                    ConstructionConfig {
                        construction_time: Duration::from_secs_f32(10.0),
                        cost: 2,
                    },
                )),
                Some(Action::Construct(
                    EntityType::TechLab,
                    ConstructionConfig {
                        construction_time: Duration::from_secs_f32(5.0),
                        cost: 1,
                    },
                )),
            ],
        },
        EntityType::Barracks => EntityConfig {
            max_health: Some(3),
            category: CategoryConfig::StructureSize([2, 2]),
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
        EntityType::TechLab => EntityConfig {
            max_health: Some(5),
            category: CategoryConfig::StructureSize([3, 3]),
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
        EntityType::FuelRift => EntityConfig {
            max_health: None,
            category: CategoryConfig::ResourceCapacity(10),
            actions: [None; NUM_ENTITY_ACTIONS],
        },
    }
}

pub struct EntityHudConfig {
    pub name: String,
    pub portrait: Picture,
}

#[derive(Clone, Debug)]
pub enum Picture {
    Mesh(Mesh),
    Image(Image),
}

impl Picture {
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        match self {
            Picture::Mesh(mesh) => mesh.draw(ctx, param),
            Picture::Image(image) => image.draw(ctx, param),
        }
    }
}

pub struct ActionHudConfig {
    pub text: String,
    pub icon: Picture,
    pub keycode: KeyCode,
}

pub struct HudAssets {
    fighter: EntityHudConfig,
    worker: EntityHudConfig,
    barracks: EntityHudConfig,
    tech_lab: EntityHudConfig,
    fuel_rift: EntityHudConfig,
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

        let worker_icon = Image::new(ctx, "/images/icons/worker.png")?;
        let tech_lab_icon = Image::new(ctx, "/images/icons/tech_lab.png")?;

        Ok(Self {
            fighter: EntityHudConfig {
                name: "Fighter".to_string(),
                portrait: Picture::Mesh(Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(
                        5.0,
                        5.0,
                        PORTRAIT_DIMENSIONS[0] - 10.0,
                        PORTRAIT_DIMENSIONS[1] - 10.0,
                    ),
                    color,
                )?),
            },
            worker: EntityHudConfig {
                name: "Worker".to_string(),
                portrait: Picture::Image(worker_icon),
            },
            barracks: EntityHudConfig {
                name: "Barracks".to_string(),
                portrait: Picture::Mesh(Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(
                        5.0,
                        5.0,
                        PORTRAIT_DIMENSIONS[0] - 10.0,
                        PORTRAIT_DIMENSIONS[1] - 10.0,
                    ),
                    color,
                )?),
            },
            tech_lab: EntityHudConfig {
                name: "Tech Lab".to_string(),
                portrait: Picture::Image(tech_lab_icon),
            },
            fuel_rift: EntityHudConfig {
                name: "Fuel rift".to_string(),
                portrait: Picture::Mesh(Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(
                        5.0,
                        5.0,
                        PORTRAIT_DIMENSIONS[0] - 10.0,
                        PORTRAIT_DIMENSIONS[1] - 10.0,
                    ),
                    color,
                )?),
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
            EntityType::TechLab => &self.tech_lab,
            EntityType::FuelRift => &self.fuel_rift,
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
                        "Train {} ({} fuel, {}s)",
                        &unit_config.name,
                        training_config.cost,
                        training_config.duration.as_secs()
                    ),
                    icon: unit_config.portrait.clone(),
                    keycode,
                }
            }
            Action::Construct(structure_type, construction_config) => {
                let keycode = match structure_type {
                    EntityType::Barracks => KeyCode::B,
                    EntityType::TechLab => KeyCode::T,
                    _ => panic!("No keycode for constructing: {:?}", structure_type),
                };
                let structure_config = self.entity(structure_type);
                ActionHudConfig {
                    text: format!(
                        "Construct {} ({} fuel, {}s)",
                        &structure_config.name,
                        construction_config.cost,
                        construction_config.construction_time.as_secs()
                    ),
                    icon: structure_config.portrait.clone(),
                    keycode,
                }
            }
            Action::Stop => ActionHudConfig {
                text: "Stop".to_owned(),
                icon: Picture::Image(self.stop_icon.clone()),
                keycode: KeyCode::S,
            },
            Action::Move => ActionHudConfig {
                text: "Move".to_owned(),
                icon: Picture::Image(self.move_icon.clone()),
                keycode: KeyCode::M,
            },
            Action::Attack => ActionHudConfig {
                text: "Attack".to_owned(),
                icon: Picture::Image(self.attack_icon.clone()),
                keycode: KeyCode::A,
            },
            Action::GatherResource => ActionHudConfig {
                text: "Gather".to_owned(),
                icon: Picture::Image(self.gather_icon.clone()),
                keycode: KeyCode::G,
            },
            Action::ReturnResource => ActionHudConfig {
                text: "Return".to_owned(),
                icon: Picture::Image(self.return_icon.clone()),
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
    create_tech_lab(ctx, &mut sprite_batches)?;
    create_fuel_rift(ctx, &mut sprite_batches)?;

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
        sprite_batches.insert(
            (EntityType::Fighter, team),
            Animation::Static(StaticImage {
                image,
                origin: [0.0, 0.0],
            }),
        );
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
        let team_image = recolor(ctx, [image.width(), image.height()], &rgba, &color_family)?;

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
    Static(StaticImage),
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
            Animation::Static(image) => image.draw(ctx, position_on_screen),
        }
    }
}

pub struct StaticImage {
    image: Image,
    // origin y == 20, means that the top part of the sprite
    // will protrude 20 pixels above the cell that it occupies.
    origin: [f32; 2],
}

impl StaticImage {
    pub fn draw(&self, ctx: &mut Context, position_on_screen: [f32; 2]) -> GameResult {
        let pos = [
            position_on_screen[0] - self.origin[0],
            position_on_screen[1] - self.origin[1],
        ];
        self.image.draw(ctx, DrawParam::new().dest(pos))
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
        sprite_batches.insert(
            (EntityType::Barracks, team),
            Animation::Static(StaticImage {
                image,
                origin: [0.0, 0.0],
            }),
        );
    }
    Ok(())
}

fn create_tech_lab(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let image = Image::new(ctx, "/images/tech_lab.png")?;
    let rgba = image.to_rgba8(ctx)?;
    for (team, color_family) in TEAM_COLOR_FAMILIES {
        let team_image = recolor(ctx, [image.width(), image.height()], &rgba, &color_family)?;
        sprite_batches.insert(
            (EntityType::TechLab, team),
            Animation::Static(StaticImage {
                image: team_image,
                origin: [0.0, 0.0],
            }),
        );
    }
    Ok(())
}

fn create_fuel_rift(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let image = Image::new(ctx, "/images/fuel_rift.png")?;

    sprite_batches.insert(
        (EntityType::FuelRift, Team::Neutral),
        Animation::Static(StaticImage {
            image,
            origin: [8.0, 8.0],
        }),
    );
    Ok(())
}

fn recolor(
    ctx: &mut Context,
    size: [u16; 2],
    rgba: &[u8],
    color_family: &EntityColorFamily,
) -> GameResult<Image> {
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
    Image::from_rgba8(ctx, size[0], size[1], &recolored[..])
}
