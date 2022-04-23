use std::collections::HashMap;
use std::time::Duration;

use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Image, Mesh, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

use crate::entities::{
    Action, AnimationState, CategoryConfig, ConstructionConfig, Direction, Entity, EntityConfig,
    EntityState, Team, TrainingConfig, NUM_ENTITY_ACTIONS,
};
use crate::hud_graphics::entity_portrait::PORTRAIT_DIMENSIONS;

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum EntityType {
    FuelRift,
    Enforcer,
    Engineer,
    BattleAcademy,
    TechLab,
}

pub fn create_entity(entity_type: EntityType, position: [u32; 2], team: Team) -> Entity {
    let config = entity_config(entity_type);
    Entity::new(entity_type, config, position, team)
}

pub fn structure_sizes() -> HashMap<EntityType, [u32; 2]> {
    let mut map: HashMap<EntityType, [u32; 2]> = Default::default();
    let structure_types = [EntityType::BattleAcademy, EntityType::TechLab];
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
        EntityType::Enforcer => EntityConfig {
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
        EntityType::Engineer => EntityConfig {
            max_health: Some(5),
            category: CategoryConfig::UnitMovementCooldown(Duration::from_millis(900)),
            actions: [
                Some(Action::Move),
                Some(Action::Stop),
                Some(Action::GatherResource),
                Some(Action::ReturnResource),
                Some(Action::Construct(
                    EntityType::BattleAcademy,
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
        EntityType::BattleAcademy => EntityConfig {
            max_health: Some(3),
            category: CategoryConfig::StructureSize([3, 3]),
            actions: [
                Some(Action::Train(
                    EntityType::Enforcer,
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
                    EntityType::Engineer,
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
    enforcer: EntityHudConfig,
    engineer: EntityHudConfig,
    battle_academy: EntityHudConfig,
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

        let engineer_icon = Image::new(ctx, "/images/icons/engineer.png")?;
        let enforcer_icon = Image::new(ctx, "/images/icons/enforcer.png")?;
        let tech_lab_icon = Image::new(ctx, "/images/icons/tech_lab.png")?;

        Ok(Self {
            enforcer: EntityHudConfig {
                name: "Enforcer".to_string(),
                portrait: Picture::Image(enforcer_icon),
            },
            engineer: EntityHudConfig {
                name: "Engineer".to_string(),
                portrait: Picture::Image(engineer_icon),
            },
            battle_academy: EntityHudConfig {
                name: "Battle Academy".to_string(),
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
            EntityType::Enforcer => &self.enforcer,
            EntityType::Engineer => &self.engineer,
            EntityType::BattleAcademy => &self.battle_academy,
            EntityType::TechLab => &self.tech_lab,
            EntityType::FuelRift => &self.fuel_rift,
        }
    }

    pub fn action(&self, action: Action) -> ActionHudConfig {
        match action {
            Action::Train(entity_type, training_config) => {
                let unit_config = self.entity(entity_type);
                let keycode = match entity_type {
                    EntityType::Engineer => KeyCode::E,
                    EntityType::Enforcer => KeyCode::F,
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
                    EntityType::BattleAcademy => KeyCode::B,
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

pub fn create_entity_animations(
    ctx: &mut Context,
) -> GameResult<HashMap<(EntityType, Team), Animation>> {
    let mut animations = Default::default();
    create_enforcer(ctx, &mut animations)?;
    create_engineer(ctx, &mut animations)?;
    create_battle_academy(ctx, &mut animations)?;
    create_tech_lab(ctx, &mut animations)?;
    create_fuel_rift(ctx, &mut animations)?;

    Ok(animations)
}

fn create_enforcer(
    ctx: &mut Context,
    animations: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let moving = Image::new(ctx, "/images/enforcer_sheet.png")?;
    let attacking = Image::new(ctx, "/images/enforcer_attacking_sheet.png")?;
    create_unit_tilesheets(
        ctx,
        animations,
        EntityType::Enforcer,
        moving,
        Some(attacking),
    )
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

#[derive(Copy, Clone)]
struct EntityColorFamily {
    light: [u8; 4],
    dark: [u8; 4],
}

fn create_engineer(
    ctx: &mut Context,
    animations: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let moving = Image::new(ctx, "/images/engineer_sheet.png")?;
    create_unit_tilesheets(ctx, animations, EntityType::Engineer, moving, None)
}

fn create_unit_tilesheets(
    ctx: &mut Context,
    animations: &mut HashMap<(EntityType, Team), Animation>,
    entity_type: EntityType,
    moving_image: Image,
    attacking_image: Option<Image>,
) -> GameResult {
    let moving_size = [moving_image.width(), moving_image.height()];
    let moving_rgba = moving_image.to_rgba8(ctx)?;

    for (team, color_family) in TEAM_COLOR_FAMILIES {
        let moving_tilesheet = tilesheet(
            ctx,
            moving_size,
            &moving_rgba[..],
            color_family,
            AnimationType::Moving,
        )?;

        let idle_tilesheet = tilesheet(
            ctx,
            moving_size,
            &moving_rgba[..],
            color_family,
            AnimationType::Idle,
        )?;

        let attacking_tilesheet = if let Some(image) = attacking_image.as_ref() {
            let rgba = image.to_rgba8(ctx)?;
            Some(tilesheet(
                ctx,
                [image.width(), image.height()],
                &rgba[..],
                color_family,
                AnimationType::Attacking,
            )?)
        } else {
            None
        };

        animations.insert(
            (entity_type, team),
            Animation::Tilesheets(UnitTilesheets {
                idle: idle_tilesheet,
                moving: moving_tilesheet,
                attacking: attacking_tilesheet,
            }),
        );
    }
    Ok(())
}

fn tilesheet(
    ctx: &mut Context,
    size: [u16; 2],
    rgba: &[u8],
    color_family: EntityColorFamily,
    animation_type: AnimationType,
) -> GameResult<Tilesheet> {
    let image = recolor(ctx, size, rgba, &color_family)?;
    let mut frames_by_direction = HashMap::new();
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
        // Different sheets are laid out differently
        // Animations with more frames use more columns per row
        let frames = match animation_type {
            AnimationType::Idle => vec![Frame::new(
                1.0 / 3.0,
                row as f32 / 8.0,
                1.0 / 3.0,
                1.0 / 8.0,
            )],
            AnimationType::Moving => vec![
                Frame::new(1.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
                Frame::new(0.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
                Frame::new(1.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
                Frame::new(2.0 / 3.0, row as f32 / 8.0, 1.0 / 3.0, 1.0 / 8.0),
            ],
            AnimationType::Attacking => vec![
                Frame::new(0.0 / 2.0, row as f32 / 8.0, 1.0 / 2.0, 1.0 / 8.0),
                Frame::new(1.0 / 2.0, row as f32 / 8.0, 1.0 / 2.0, 1.0 / 8.0),
            ],
        };

        frames_by_direction.insert(direction, frames);
    }

    Ok(Tilesheet {
        sheet: image,
        origin: [0.0, 16.0],
        frames: frames_by_direction,
    })
}

pub enum Animation {
    Tilesheets(UnitTilesheets),
    Static(StaticImage),
}

impl Animation {
    pub fn draw(
        &self,
        ctx: &mut Context,
        entity_state: &EntityState,
        animation: &AnimationState,
        direction: Direction,
        position_on_screen: [f32; 2],
    ) -> GameResult {
        match self {
            Animation::Tilesheets(tilesheets) => {
                tilesheets.draw(ctx, entity_state, animation, direction, position_on_screen)
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

pub struct UnitTilesheets {
    idle: Tilesheet,
    moving: Tilesheet,
    attacking: Option<Tilesheet>,
}

impl UnitTilesheets {
    pub fn draw(
        &self,
        ctx: &mut Context,
        entity_state: &EntityState,
        animation: &AnimationState,
        direction: Direction,
        position_on_screen: [f32; 2],
    ) -> GameResult {
        let tilesheet = match entity_state {
            EntityState::Idle => &self.idle,
            EntityState::Moving => &self.moving,
            EntityState::Attacking(_) => self.attacking.as_ref().unwrap(),
            EntityState::MovingToResource(_) => &self.moving,
            // TODO gathering animation
            EntityState::GatheringResource(_) => &self.idle,
            EntityState::ReturningResource(_) => &self.moving,
            unhandled => panic!("No animation for entity state: {:?}", unhandled),
        };
        tilesheet.draw(ctx, animation, direction, position_on_screen)
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

fn create_battle_academy(
    ctx: &mut Context,
    animations: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let image = Image::new(ctx, "/images/battle_academy.png")?;
    structure_sprite(ctx, EntityType::BattleAcademy, animations, image)
}

fn structure_sprite(
    ctx: &mut Context,
    entity_type: EntityType,
    animations: &mut HashMap<(EntityType, Team), Animation>,
    image: Image,
) -> GameResult {
    let rgba = image.to_rgba8(ctx)?;
    for (team, color_family) in TEAM_COLOR_FAMILIES {
        let team_image = recolor(ctx, [image.width(), image.height()], &rgba, &color_family)?;
        animations.insert(
            (entity_type, team),
            Animation::Static(StaticImage {
                image: team_image,
                origin: [0.0, 0.0],
            }),
        );
    }
    Ok(())
}

fn create_tech_lab(
    ctx: &mut Context,
    animations: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let image = Image::new(ctx, "/images/tech_lab.png")?;
    structure_sprite(ctx, EntityType::TechLab, animations, image)
}

fn create_fuel_rift(
    ctx: &mut Context,
    animations: &mut HashMap<(EntityType, Team), Animation>,
) -> GameResult {
    let image = Image::new(ctx, "/images/fuel_rift.png")?;

    animations.insert(
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

#[derive(Debug, Copy, Clone)]
enum AnimationType {
    Idle,
    Moving,
    Attacking,
}
