use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Mesh, MeshBuilder, Rect};
use ggez::input::mouse::MouseButton;
use ggez::{graphics, Context, ContextBuilder, GameError};

use rand::rngs::ThreadRng;
use rand::Rng;
use std::time::Duration;

use crate::entities::{Entity, EntitySprite, MovementComponent, Team};
use crate::images;

const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);
const COLOR_GRID: Color = Color::new(0.3, 0.3, 0.4, 1.0);

const WINDOW_DIMENSIONS: (f32, f32) = (1600.0, 1200.0);
pub const CELL_PIXEL_SIZE: (f32, f32) = (100.0, 100.0);
pub const WORLD_PIXEL_OFFSET: (f32, f32) = (20.0, 20.0);
const GRID_DIMENSIONS: (u32, u32) = (8, 8);

const TITLE: &str = "RTS";

pub fn run() -> Result<(), GameError> {
    let (mut ctx, event_loop) = ContextBuilder::new("rts", "jm")
        .window_setup(WindowSetup::default().title(TITLE))
        .window_mode(WindowMode::default().dimensions(WINDOW_DIMENSIONS.0, WINDOW_DIMENSIONS.1))
        //.add_resource_path("resources")
        .build()
        .expect("Creating ggez context");

    let game = Game::new(&mut ctx)?;
    ggez::event::run(ctx, event_loop, game)
}

struct EnemyPlayerAi {
    timer_s: f32,
}

impl EnemyPlayerAi {
    fn new() -> Self {
        Self { timer_s: 0.0 }
    }

    fn run(&mut self, dt: Duration, entities: &mut [Entity], rng: &mut ThreadRng) {
        self.timer_s -= dt.as_secs_f32();

        // TODO Instead of mutating game state, return commands
        if self.timer_s <= 0.0 {
            self.timer_s = 2.0;
            for enemy in entities {
                if enemy.team == Team::Ai && rng.gen_bool(0.7) {
                    let x: u32 = rng.gen_range(0..GRID_DIMENSIONS.0);
                    let y: u32 = rng.gen_range(0..GRID_DIMENSIONS.1);
                    enemy.set_destination([x, y]);
                }
            }
        }
    }
}

struct EntityGrid(Vec<bool>);

impl EntityGrid {
    fn new() -> Self {
        Self(vec![
            false;
            (GRID_DIMENSIONS.0 * GRID_DIMENSIONS.1) as usize
        ])
    }

    fn set(&mut self, position: [u32; 2], occupied: bool) {
        self.0[EntityGrid::index(position)] = occupied;
    }

    fn get(&self, position: [u32; 2]) -> bool {
        self.0[EntityGrid::index(position)]
    }

    fn index(position: [u32; 2]) -> usize {
        let [x, y] = position;
        (y * GRID_DIMENSIONS.0 + x) as usize
    }
}

struct Game {
    grid_mesh: Mesh,
    player_mesh: Mesh,
    enemy_sprite_batch: SpriteBatch,
    entities: Vec<Entity>,
    entity_grid: EntityGrid,
    enemy_player_ai: EnemyPlayerAi,
    rng: ThreadRng,
}

impl Game {
    fn new(ctx: &mut Context) -> Result<Self, GameError> {
        let grid_mesh = Self::build_grid(ctx)?;

        let player_size = (CELL_PIXEL_SIZE.0 * 0.7, CELL_PIXEL_SIZE.1 * 0.8);
        let player_mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    (CELL_PIXEL_SIZE.0 - player_size.0) / 2.0,
                    (CELL_PIXEL_SIZE.1 - player_size.1) / 2.0,
                    player_size.0,
                    player_size.1,
                ),
                Color::new(0.6, 0.8, 0.5, 1.0),
            )?
            .build(ctx)?;
        let player_entity = Entity::new(
            MovementComponent::new([0, 0], Duration::from_millis(400)),
            Team::Player,
            EntitySprite::Player,
        );
        let mut entities = vec![player_entity];

        let enemy_mesh = MeshBuilder::new()
            .circle(
                DrawMode::fill(),
                [CELL_PIXEL_SIZE.0 / 2.0, CELL_PIXEL_SIZE.1 / 2.0],
                CELL_PIXEL_SIZE.0 * 0.25,
                0.05,
                Color::new(0.8, 0.4, 0.4, 1.0),
            )?
            .build(ctx)?;
        let enemy_sprite_batch = SpriteBatch::new(images::mesh_into_image(ctx, enemy_mesh)?);

        fn enemy_entity(position: [u32; 2]) -> Entity {
            Entity::new(
                MovementComponent::new(position, Duration::from_millis(800)),
                Team::Ai,
                EntitySprite::Enemy,
            )
        }

        // for y in 1..GRID_DIMENSIONS.1 {
        //     for x in 0..GRID_DIMENSIONS.0 {
        //         entities.push(enemy_entity([x, y]));
        //     }
        // }

        entities.push(enemy_entity([5, 2]));
        entities.push(enemy_entity([3, 0]));
        entities.push(enemy_entity([0, 4]));
        entities.push(enemy_entity([3, 4]));

        println!("Created {} entities", entities.len());

        let rng = rand::thread_rng();

        let mut entity_grid = EntityGrid::new();
        for entity in &entities {
            entity_grid.set(entity.movement_component.position(), true);
        }

        Ok(Self {
            grid_mesh,
            player_mesh,
            enemy_sprite_batch,
            entities,
            entity_grid,
            enemy_player_ai: EnemyPlayerAi::new(),
            rng,
        })
    }

    fn build_grid(ctx: &mut Context) -> Result<Mesh, GameError> {
        let mut builder = MeshBuilder::new();
        const LINE_WIDTH: f32 = 2.0;

        let x0 = WORLD_PIXEL_OFFSET.0;
        let x1 = x0 + GRID_DIMENSIONS.0 as f32 * CELL_PIXEL_SIZE.0;
        let y0 = WORLD_PIXEL_OFFSET.1;
        let y1 = y0 + GRID_DIMENSIONS.1 as f32 * CELL_PIXEL_SIZE.1;

        // Horizontal lines
        for i in 0..GRID_DIMENSIONS.1 + 1 {
            let y = y0 + i as f32 * CELL_PIXEL_SIZE.1;
            builder.line(&[[x0, y], [x1, y]], LINE_WIDTH, COLOR_GRID)?;
        }

        // Vertical lines
        for i in 0..GRID_DIMENSIONS.0 + 1 {
            let x = x0 + i as f32 * CELL_PIXEL_SIZE.0;
            builder.line(&[[x, y0], [x, y1]], LINE_WIDTH, COLOR_GRID)?;
        }

        builder.build(ctx)
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        let fps = ggez::timer::fps(ctx) as u32;
        graphics::set_window_title(ctx, &format!("{} (fps={})", TITLE, fps));

        let dt = ggez::timer::delta(ctx);

        self.enemy_player_ai
            .run(dt, &mut self.entities[..], &mut self.rng);

        for entity in &mut self.entities {
            if entity.movement_component.movement_timer.is_zero() {
                if let Some(target_pos) = entity.movement_plan.pop() {
                    let occupied = self.entity_grid.get(target_pos);
                    if occupied {
                        entity.movement_plan.push(target_pos);
                    } else {
                        self.entity_grid
                            .set(entity.movement_component.position(), false);
                        entity.movement_component.move_to(target_pos);
                        self.entity_grid.set(target_pos, true);
                    }
                }
            }
        }

        for entity in &mut self.entities {
            entity.movement_component.update(dt);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        graphics::clear(ctx, COLOR_BG);

        graphics::draw(ctx, &self.grid_mesh, DrawParam::new())?;

        for entity in &self.entities {
            let screen_coords = entity.movement_component.screen_coords();
            match &entity.team {
                Team::Player => {
                    graphics::draw(ctx, &self.player_mesh, DrawParam::new().dest(screen_coords))?;
                }
                Team::Ai => {
                    self.enemy_sprite_batch
                        .add(DrawParam::new().dest(screen_coords));
                }
            }
        }
        graphics::draw(ctx, &self.enemy_sprite_batch, DrawParam::default())?;
        self.enemy_sprite_batch.clear();

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        x: f32,
        y: f32,
    ) {
        if let Some(clicked_pos) = screen_to_grid_coordinates([x, y]) {
            let player_entity = self
                .entities
                .iter_mut()
                .filter(|e| e.team == Team::Player)
                .next()
                .expect("player entity");
            player_entity.set_destination(clicked_pos);
        }
    }
}

fn screen_to_grid_coordinates(coordinates: [f32; 2]) -> Option<[u32; 2]> {
    let [x, y] = coordinates;
    if x < WORLD_PIXEL_OFFSET.0 || y < WORLD_PIXEL_OFFSET.1 {
        return None;
    }
    let grid_x = ((x - WORLD_PIXEL_OFFSET.0) / CELL_PIXEL_SIZE.0) as u32;
    let grid_y = ((y - WORLD_PIXEL_OFFSET.1) / CELL_PIXEL_SIZE.1) as u32;
    if grid_x < GRID_DIMENSIONS.0 && grid_y < GRID_DIMENSIONS.1 {
        Some([grid_x as u32, grid_y as u32])
    } else {
        None
    }
}

pub fn grid_to_screen_coords(coordinates: [u32; 2]) -> [f32; 2] {
    [
        WORLD_PIXEL_OFFSET.0 + CELL_PIXEL_SIZE.0 * coordinates[0] as f32,
        WORLD_PIXEL_OFFSET.1 + CELL_PIXEL_SIZE.1 * coordinates[1] as f32,
    ]
}
