use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{EventHandler};
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Mesh, MeshBuilder, Rect};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::input::keyboard::{KeyMods, KeyCode};
use ggez::{graphics, Context, ContextBuilder, GameError};

use crate::enemy_ai::EnemyPlayerAi;

use rand::rngs::ThreadRng;
use crate::entities::{Entity, EntitySprite, Team};
use crate::images;
use crate::maps::{Map, MapType};

const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);
const COLOR_GRID: Color = Color::new(0.3, 0.3, 0.4, 1.0);

const WINDOW_DIMENSIONS: (f32, f32) = (1600.0, 1200.0);
pub const CELL_PIXEL_SIZE: (f32, f32) = (50.0, 50.0);
pub const WORLD_PIXEL_OFFSET: (f32, f32) = (20.0, 20.0);

const TITLE: &str = "RTS";

pub fn run(map_type: MapType) -> Result<(), GameError> {
    let (mut ctx, event_loop) = ContextBuilder::new("rts", "jm")
        .window_setup(WindowSetup::default().title(TITLE))
        .window_mode(WindowMode::default().dimensions(WINDOW_DIMENSIONS.0, WINDOW_DIMENSIONS.1))
        //.add_resource_path("resources")
        .build()
        .expect("Creating ggez context");

    let game = Game::new(&mut ctx, map_type)?;
    ggez::event::run(ctx, event_loop, game)
}

struct EntityGrid {
    grid: Vec<bool>,
    map_dimensions: (u32, u32),
}

impl EntityGrid {
    fn new(map_dimensions: (u32, u32)) -> Self {
        let grid = vec![false; (map_dimensions.0 * map_dimensions.1) as usize];
        Self {
            grid,
            map_dimensions,
        }
    }

    fn set(&mut self, position: &[u32; 2], occupied: bool) {
        let i = self.index(position);
        self.grid[i] = occupied;
    }

    fn get(&self, position: &[u32; 2]) -> bool {
        self.grid[self.index(position)]
    }

    fn index(&self, position: &[u32; 2]) -> usize {
        let [x, y] = position;
        (y * self.map_dimensions.0 + x) as usize
    }
}

enum MouseState {
    Default,
    DealingDamage,
}

struct Game {
    grid_mesh: Mesh,
    player_mesh: Mesh,
    neutral_mesh: Mesh,
    enemy_sprite_batch: SpriteBatch,
    entities: Vec<Entity>,
    entity_grid: EntityGrid,
    enemy_player_ai: EnemyPlayerAi,
    rng: ThreadRng,
    mouse_state: MouseState,
}

impl Game {
    fn new(ctx: &mut Context, map_type: MapType) -> Result<Self, GameError> {
        let Map {
            dimensions: map_dimensions,
            entities,
        } = Map::new(map_type);

        let grid_mesh = Self::build_grid(ctx, map_dimensions)?;

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

        let neutral_size = (CELL_PIXEL_SIZE.0 * 0.7, CELL_PIXEL_SIZE.1 * 0.6);
        let neutral_mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    (CELL_PIXEL_SIZE.0 - player_size.0) / 2.0,
                    (CELL_PIXEL_SIZE.1 - player_size.1) / 2.0,
                    neutral_size.0,
                    neutral_size.1,
                ),
                Color::new(0.8, 0.6, 0.2, 1.0),
            )?
            .build(ctx)?;

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

        println!("Created {} entities", entities.len());

        let rng = rand::thread_rng();

        let mut entity_grid = EntityGrid::new(map_dimensions);
        for entity in &entities {
            entity_grid.set(&entity.physics.position(), true);
        }

        let enemy_player_ai = EnemyPlayerAi::new(map_dimensions);
        let mouse_state = MouseState::Default;

        Ok(Self {
            grid_mesh,
            player_mesh,
            neutral_mesh,
            enemy_sprite_batch,
            entities,
            entity_grid,
            enemy_player_ai,
            rng,
            mouse_state,
        })
    }

    fn build_grid(ctx: &mut Context, map_dimensions: (u32, u32)) -> Result<Mesh, GameError> {
        let mut builder = MeshBuilder::new();
        const LINE_WIDTH: f32 = 2.0;

        let x0 = WORLD_PIXEL_OFFSET.0;
        let x1 = x0 + map_dimensions.0 as f32 * CELL_PIXEL_SIZE.0;
        let y0 = WORLD_PIXEL_OFFSET.1;
        let y1 = y0 + map_dimensions.1 as f32 * CELL_PIXEL_SIZE.1;

        // Horizontal lines
        for i in 0..map_dimensions.1 + 1 {
            let y = y0 + i as f32 * CELL_PIXEL_SIZE.1;
            builder.line(&[[x0, y], [x1, y]], LINE_WIDTH, COLOR_GRID)?;
        }

        // Vertical lines
        for i in 0..map_dimensions.0 + 1 {
            let x = x0 + i as f32 * CELL_PIXEL_SIZE.0;
            builder.line(&[[x, y0], [x, y1]], LINE_WIDTH, COLOR_GRID)?;
        }

        builder.build(ctx)
    }

    fn screen_to_grid_coordinates(&self, coordinates: [f32; 2]) -> Option<[u32; 2]> {
        let [x, y] = coordinates;
        if x < WORLD_PIXEL_OFFSET.0 || y < WORLD_PIXEL_OFFSET.1 {
            return None;
        }
        let grid_x = ((x - WORLD_PIXEL_OFFSET.0) / CELL_PIXEL_SIZE.0) as u32;
        let grid_y = ((y - WORLD_PIXEL_OFFSET.1) / CELL_PIXEL_SIZE.1) as u32;
        if grid_x < self.entity_grid.map_dimensions.0 && grid_y < self.entity_grid.map_dimensions.1
        {
            Some([grid_x as u32, grid_y as u32])
        } else {
            None
        }
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        let fps = ggez::timer::fps(ctx) as u32;
        graphics::set_window_title(ctx, &format!("{} (fps={})", TITLE, fps));

        let dt = ggez::timer::delta(ctx);

        self.enemy_player_ai
            .run(dt, &mut self.entities[..], &mut self.rng);

        // Remove dead entities
        self.entities.retain(|e| {
            let is_dead = e
                .health
                .as_ref()
                .map(|health| health.current == 0)
                .unwrap_or(false);
            if is_dead {
                self.entity_grid.set(&e.physics.position(), false);
            }
            !is_dead
        });

        for entity in &mut self.entities {
            if entity.physics.is_ready_for_movement() {
                if let Some(next_pos) = entity.pathfind.peek_path() {
                    let occupied = self.entity_grid.get(next_pos);
                    if !occupied {
                        let new_pos = entity.pathfind.advance_path();
                        self.entity_grid.set(&entity.physics.position(), false);
                        entity.physics.move_to(new_pos);
                        self.entity_grid.set(&new_pos, true);
                    }
                }
            }
        }

        for entity in &mut self.entities {
            entity.physics.update(dt);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        graphics::clear(ctx, COLOR_BG);

        graphics::draw(ctx, &self.grid_mesh, DrawParam::new())?;

        for entity in &self.entities {
            let screen_coords = entity.physics.screen_coords();
            match &entity.sprite {
                EntitySprite::Player => {
                    graphics::draw(ctx, &self.player_mesh, DrawParam::new().dest(screen_coords))?;
                }
                EntitySprite::Enemy => {
                    self.enemy_sprite_batch
                        .add(DrawParam::new().dest(screen_coords));
                }
                EntitySprite::Neutral => {
                    graphics::draw(
                        ctx,
                        &self.neutral_mesh,
                        DrawParam::new().dest(screen_coords),
                    )?;
                }
            }
        }
        graphics::draw(ctx, &self.enemy_sprite_batch, DrawParam::default())?;
        self.enemy_sprite_batch.clear();

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, _button: MouseButton, x: f32, y: f32) {
        if let Some(clicked_pos) = self.screen_to_grid_coordinates([x, y]) {

            match self.mouse_state {
                MouseState::Default => {
                    let player_entity = self
                        .entities
                        .iter_mut()
                        .find(|e| e.team == Team::Player)
                        .expect("player entity");
                    let current_pos = &player_entity.physics.position();
                    player_entity.pathfind.find_path(current_pos, clicked_pos);
                }
                MouseState::DealingDamage => {
                    // TODO
                    if let Some(mut health) = self
                        .entities
                        .iter_mut()
                        .filter(|e| e.physics.position() == clicked_pos)
                        .filter_map(|e| e.health.as_mut())
                        .next()
                    {
                        health.current -= 1;
                        println!("Reduced health down to {}/{}", health.current, health.max)
                    }
                    self.mouse_state = MouseState::Default;
                    mouse::set_cursor_type(ctx, CursorIcon::Default);
                }
            }
        }
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        if keycode == KeyCode::Escape {
            ggez::event::quit(ctx);
        } else {
            self.mouse_state = MouseState::DealingDamage;
            mouse::set_cursor_type(ctx, CursorIcon::Crosshair);
        }
    }
}

pub fn grid_to_screen_coords(coordinates: [u32; 2]) -> [f32; 2] {
    [
        WORLD_PIXEL_OFFSET.0 + CELL_PIXEL_SIZE.0 * coordinates[0] as f32,
        WORLD_PIXEL_OFFSET.1 + CELL_PIXEL_SIZE.1 * coordinates[1] as f32,
    ]
}
