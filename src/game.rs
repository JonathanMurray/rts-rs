use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Mesh, MeshBuilder, Rect};
use ggez::input::mouse::MouseButton;
use ggez::{graphics, Context, ContextBuilder, GameError};
use std::time::Duration;

use crate::images;

const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);
const COLOR_GRID: Color = Color::new(0.8, 0.8, 0.8, 1.0);

const WINDOW_DIMENSIONS: (f32, f32) = (1600.0, 1200.0);
const CELL_PIXEL_SIZE: (f32, f32) = (32.0, 32.0);
const WORLD_PIXEL_OFFSET: (f32, f32) = (20.0, 20.0);
const GRID_DIMENSIONS: (u32, u32) = (40, 32);

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

struct Entity {
    previous_position: [u32; 2],
    position: [u32; 2],
    movement_timer: Duration,
    movement_cooldown: Duration,
}

impl Entity {
    fn new(position: [u32; 2], movement_cooldown: Duration) -> Self {
        Self {
            previous_position: position,
            position,
            movement_timer: Duration::ZERO,
            movement_cooldown,
        }
    }

    fn update(&mut self, dt: Duration) {
        if self.movement_timer < dt {
            self.movement_timer = Duration::ZERO;
        } else {
            self.movement_timer -= dt;
        }
        if self.movement_timer.is_zero() {
            self.previous_position = self.position;
        }
    }

    fn sprite_screen_coords(&self) -> [f32; 2] {
        let prev_pos = grid_to_screen_coords(self.previous_position);
        let pos = grid_to_screen_coords(self.position);
        let interpolation =
            self.movement_timer.as_secs_f32() / self.movement_cooldown.as_secs_f32();
        [
            pos[0] - interpolation * (pos[0] - prev_pos[0]),
            pos[1] - interpolation * (pos[1] - prev_pos[1]),
        ]
    }

    fn move_to(&mut self, new_position: [u32; 2]) {
        assert!(self.movement_timer.is_zero());
        self.position = new_position;
        self.movement_timer = self.movement_cooldown;
    }
}

struct Game {
    grid_mesh: Mesh,
    player_mesh: Mesh,
    player_entity: Entity,
    enemy_sprite_batch: SpriteBatch,
    enemy_entities: Vec<Entity>,
}

impl Game {
    fn new(ctx: &mut Context) -> Result<Self, GameError> {
        let grid_mesh = Self::build_grid(ctx)?;

        let player_mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(0.0, 0.0, CELL_PIXEL_SIZE.0 - 2.0, CELL_PIXEL_SIZE.1 - 2.0),
                Color::new(0.6, 0.8, 0.5, 1.0),
            )?
            .build(ctx)?;
        let player_entity = Entity::new([0, 0], Duration::from_millis(400));

        let enemy_mesh = MeshBuilder::new()
            .circle(
                DrawMode::fill(),
                [CELL_PIXEL_SIZE.0 / 2.0, CELL_PIXEL_SIZE.1 / 2.0],
                CELL_PIXEL_SIZE.0 * 0.25,
                0.05,
                Color::new(0.8, 0.4, 0.4, 1.0),
            )?
            .build(ctx)?;
        let enemy_sprite_batch = images::mesh_into_image(ctx, enemy_mesh)?;
        let mut enemy_entities = vec![];
        for y in 1..GRID_DIMENSIONS.1 {
            for x in 0..GRID_DIMENSIONS.0 {
                enemy_entities.push(Entity::new([x, y], Duration::from_millis(400)));
            }
        }
        println!("Created {} enemy entities", enemy_entities.len());

        Ok(Self {
            grid_mesh,
            player_mesh,
            player_entity,
            enemy_sprite_batch,
            enemy_entities,
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
        self.player_entity.update(dt);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        graphics::clear(ctx, COLOR_BG);

        graphics::draw(ctx, &self.grid_mesh, DrawParam::new())?;

        graphics::draw(
            ctx,
            &self.player_mesh,
            DrawParam::new().dest(self.player_entity.sprite_screen_coords()),
        )?;

        for enemy_entity in &self.enemy_entities {
            let draw_param = DrawParam::new().dest(enemy_entity.sprite_screen_coords());
            self.enemy_sprite_batch.add(draw_param);
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
        if let Some(pos) = screen_to_grid_coordinates([x, y]) {
            if self.player_entity.movement_timer.is_zero() {
                let dx = (pos[0] as i32 - self.player_entity.position[0] as i32).abs();
                let dy = (pos[1] as i32 - self.player_entity.position[1] as i32).abs();
                // No diagonal movement for now
                if (dx == 0 && dy == 1) || (dx == 1 && dy == 0) {
                    self.player_entity.move_to(pos);
                }
            }
        }
    }
}

fn grid_to_screen_coords(coordinates: [u32; 2]) -> [f32; 2] {
    [
        WORLD_PIXEL_OFFSET.0 + CELL_PIXEL_SIZE.0 * coordinates[0] as f32,
        WORLD_PIXEL_OFFSET.1 + CELL_PIXEL_SIZE.1 * coordinates[1] as f32,
    ]
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
