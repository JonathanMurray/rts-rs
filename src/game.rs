use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, DrawParam, Font};
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};

use rand::rngs::ThreadRng;

use crate::assets::{self, Assets};
use crate::data::{self, Map, MapType};
use crate::enemy_ai::EnemyPlayerAi;
use crate::entities::{Entity, EntityId, Team, TrainingUpdateStatus};
use crate::hud_graphics::HudGraphics;
use std::cmp::min;

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

struct PlayerState {
    selected_entity_id: Option<EntityId>,
    mouse_state: MouseState,
}

struct Game {
    assets: Assets,
    hud: HudGraphics,
    player_state: PlayerState,
    entities: Vec<Entity>,
    entity_grid: EntityGrid,
    enemy_player_ai: EnemyPlayerAi,
    rng: ThreadRng,
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

        let font = Font::new(ctx, "/fonts/Merchant Copy.ttf")?;

        let player_state = PlayerState {
            selected_entity_id: None,
            mouse_state: MouseState::Default,
        };

        let hud_pos = [
            WORLD_PIXEL_OFFSET.0 + entity_grid.map_dimensions.0 as f32 * CELL_PIXEL_SIZE.0 + 40.0,
            25.0,
        ];
        let hud = HudGraphics::new(hud_pos, font);

        Ok(Self {
            assets,
            hud,
            player_state,
            entities,
            entity_grid,
            enemy_player_ai,
            rng,
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

    fn selected_entity(&self) -> Option<&Entity> {
        self.player_state.selected_entity_id.map(|id| {
            self.entities
                .iter()
                .find(|e| e.id == id)
                .expect("selected entity must exist")
        })
    }

    fn selected_entity_mut(&mut self) -> Option<&mut Entity> {
        self.player_state.selected_entity_id.map(|id| {
            self.entities
                .iter_mut()
                .find(|e| e.id == id)
                .expect("selected entity must exist")
        })
    }

    fn add_entity(&mut self, target_position: [u32; 2]) -> Option<[u32; 2]> {
        let left = target_position[0].saturating_sub(1);
        let top = target_position[1].saturating_sub(1);
        let right = min(
            target_position[0] + 1,
            self.entity_grid.map_dimensions.0 - 1,
        );
        let bot = min(
            target_position[1] + 1,
            self.entity_grid.map_dimensions.1 - 1,
        );
        for x in left..right + 1 {
            for y in top..bot + 1 {
                if !self.entity_grid.get(&[x, y]) {
                    let new_entity = data::create_player_entity_1([x, y]);
                    self.entities.push(new_entity);
                    self.entity_grid.set(&[x, y], true);
                    return Some([x, y]);
                }
            }
        }
        None
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
                if movement.sub_cell_movement.is_ready() {
                    if let Some(next_pos) = movement.pathfinder.peek_path() {
                        let occupied = self.entity_grid.get(next_pos);
                        if !occupied {
                            let old_pos = entity.position;
                            let new_pos = movement.pathfinder.advance_path();
                            self.entity_grid.set(&old_pos, false);
                            movement.sub_cell_movement.set_moving(old_pos, new_pos);
                            entity.position = new_pos;
                            self.entity_grid.set(&new_pos, true);
                        }
                    }
                }
            }
        }

        for entity in &mut self.entities {
            if let Some(movement) = entity.movement.as_mut() {
                movement.sub_cell_movement.update(dt, entity.position);
            }
        }

        let mut requested_entity_creations = Vec::new();
        for entity in &mut self.entities {
            let status = entity
                .training_action
                .as_mut()
                .map(|training_action| training_action.update(dt));
            if status == Some(TrainingUpdateStatus::Done) {
                requested_entity_creations.push(entity.position);
            }
        }

        for target_position in requested_entity_creations {
            let actual_pos = self.add_entity(target_position);
            println!("Created entity at: {:?}", actual_pos);
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
                .map(|movement| movement.sub_cell_movement.screen_coords(entity.position))
                .unwrap_or_else(|| grid_to_screen_coords(entity.position));

            if self.player_state.selected_entity_id.as_ref() == Some(&entity.id) {
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

        let num_entities = self.entities.len();
        let selected_entity = self.selected_entity();
        self.hud.draw(ctx, selected_entity, num_entities)?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let Some(clicked_pos) = self.screen_to_grid_coordinates([x, y]) {
            match self.player_state.mouse_state {
                MouseState::Default => {
                    if button == MouseButton::Left {
                        self.player_state.selected_entity_id = self
                            .entities
                            .iter()
                            .find(|e| e.team == Team::Player && e.position == clicked_pos)
                            .map(|e| e.id);
                        println!(
                            "Selected entity index: {:?}",
                            self.player_state.selected_entity_id
                        );
                    } else if let Some(player_entity) = self.selected_entity_mut() {
                        if let Some(movement) = player_entity.movement.as_mut() {
                            movement
                                .pathfinder
                                .find_path(&player_entity.position, clicked_pos);
                        } else {
                            println!("Selected entity is immobile")
                        }
                    } else {
                        println!("No entity is selected");
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
                    self.player_state.mouse_state = MouseState::Default;
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
        match keycode {
            KeyCode::Escape => ggez::event::quit(ctx),
            KeyCode::A => {
                self.player_state.mouse_state = MouseState::DealingDamage;
                mouse::set_cursor_type(ctx, CursorIcon::Crosshair);
            }
            KeyCode::B => {
                if let Some(player_entity) = self.selected_entity_mut() {
                    if let Some(training_action) = &mut player_entity.training_action {
                        training_action.perform();
                    } else {
                        println!("Selected entity has no such action")
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn grid_to_screen_coords(coordinates: [u32; 2]) -> [f32; 2] {
    [
        WORLD_PIXEL_OFFSET.0 + CELL_PIXEL_SIZE.0 * coordinates[0] as f32,
        WORLD_PIXEL_OFFSET.1 + CELL_PIXEL_SIZE.1 * coordinates[1] as f32,
    ]
}
