use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, DrawParam, Font, Text};
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};

use rand::rngs::ThreadRng;

use crate::assets::{self, Assets};
use crate::enemy_ai::EnemyPlayerAi;
use crate::entities::{Entity, EntityId, Team};
use crate::maps::{Map, MapType};

const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);

const WINDOW_DIMENSIONS: (f32, f32) = (1600.0, 1200.0);
pub const CELL_PIXEL_SIZE: (f32, f32) = (50.0, 50.0);
pub const WORLD_PIXEL_OFFSET: (f32, f32) = (20.0, 20.0);

const TITLE: &str = "RTS";

pub fn run(map_type: MapType) -> GameResult {
    let (mut ctx, event_loop) = ContextBuilder::new("rts", "jm")
        .window_setup(WindowSetup::default().title(TITLE))
        .window_mode(WindowMode::default().dimensions(WINDOW_DIMENSIONS.0, WINDOW_DIMENSIONS.1))
        .add_resource_path("resources")
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
    font: Font,
    assets: Assets,
    selected_entity: Option<EntityId>,
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

        println!("Created {} entities", entities.len());

        let assets = assets::create_assets(ctx, map_dimensions)?;

        let rng = rand::thread_rng();

        let mut entity_grid = EntityGrid::new(map_dimensions);
        for entity in &entities {
            if entity.is_solid {
                entity_grid.set(&entity.position, true);
            }
        }

        let enemy_player_ai = EnemyPlayerAi::new(map_dimensions);
        let mouse_state = MouseState::Default;

        let font = Font::new(ctx, "/fonts/Merchant Copy.ttf")?;

        Ok(Self {
            font,
            assets,
            selected_entity: None,
            entities,
            entity_grid,
            enemy_player_ai,
            rng,
            mouse_state,
        })
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

    fn draw_debug_ui(&self, ctx: &mut Context) -> GameResult {
        let mut lines = vec![];
        lines.push(format!("Selected: {:?}", self.selected_entity));
        lines.push(format!("Total entities: {:?}", self.entities.len()));

        let x = WORLD_PIXEL_OFFSET.0
            + self.entity_grid.map_dimensions.0 as f32 * CELL_PIXEL_SIZE.0
            + 40.0;
        let mut y = 25.0;
        for line in lines {
            let text = Text::new((line, self.font, 25.0));
            graphics::draw(ctx, &text, DrawParam::new().dest([x, y]))?;
            y += 25.0;
        }
        Ok(())
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
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
            if is_dead && e.is_solid {
                self.entity_grid.set(&e.position, false);
            }
            !is_dead
        });

        for entity in &mut self.entities {
            if let Some(movement) = &mut entity.movement {
                if movement.is_ready() {
                    if let Some(next_pos) = entity.pathfind.peek_path() {
                        let occupied = self.entity_grid.get(next_pos);
                        if !occupied {
                            let old_pos = entity.position;
                            let new_pos = entity.pathfind.advance_path();
                            self.entity_grid.set(&old_pos, false);
                            movement.set_moving(old_pos, new_pos);
                            entity.position = new_pos;
                            self.entity_grid.set(&new_pos, true);
                        }
                    }
                }
            }
        }

        for entity in &mut self.entities {
            if let Some(movement) = entity.movement.as_mut() {
                movement.update(dt, entity.position);
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, COLOR_BG);

        graphics::draw(ctx, &self.assets.grid, DrawParam::new())?;

        for entity in &self.entities {
            let screen_coords = entity
                .movement
                .as_ref()
                .map(|movement| movement.screen_coords(entity.position))
                .unwrap_or_else(|| grid_to_screen_coords(entity.position));

            if self.selected_entity.as_ref() == Some(&entity.id) {
                graphics::draw(
                    ctx,
                    &self.assets.selection,
                    DrawParam::new().dest(screen_coords),
                )?;
            }

            self.assets
                .draw_entity(ctx, &entity.sprite, screen_coords)?;
        }
        self.assets.flush_entity_sprite_batch(ctx)?;

        self.draw_debug_ui(ctx)?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let Some(clicked_pos) = self.screen_to_grid_coordinates([x, y]) {
            match self.mouse_state {
                MouseState::Default => {
                    if button == MouseButton::Left {
                        self.selected_entity = self
                            .entities
                            .iter()
                            .find(|e| e.team == Team::Player && e.position == clicked_pos)
                            .map(|e| e.id);
                        println!("Selected entity index: {:?}", self.selected_entity);
                    } else {
                        if let Some(selected_entity) = &self.selected_entity {
                            let player_entity = self
                                .entities
                                .iter_mut()
                                .find(|e| &e.id == selected_entity)
                                .expect("selected entity must exist");
                            player_entity
                                .pathfind
                                .find_path(&player_entity.position, clicked_pos);
                        } else {
                            println!("No entity is selected");
                        }
                    }
                }
                MouseState::DealingDamage => {
                    // TODO
                    if let Some(mut health) = self
                        .entities
                        .iter_mut()
                        .filter(|e| e.position == clicked_pos)
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
