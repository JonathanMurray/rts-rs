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
use crate::entities::{
    Entity, EntityId, EntityType, Team, TrainingPerformStatus, TrainingUpdateStatus,
};
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
        // Protect against bugs where two entities occupy same cell or we "double free" a cell
        assert_ne!(
            self.grid[i], occupied,
            "Trying to set grid{:?}={} but it already has that value!",
            position, occupied
        );
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

pub struct TeamState {
    pub resources: u32,
}

struct Game {
    assets: Assets,
    hud: HudGraphics,
    player_team_state: TeamState,
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
                // TODO set area?
                let [w, h] = entity.size();
                for x in entity.position[0]..entity.position[0] + w {
                    for y in entity.position[1]..entity.position[1] + h {
                        entity_grid.set(&[x, y], true);
                    }
                }
            }
        }

        let enemy_player_ai = EnemyPlayerAi::new(map_dimensions);

        let font = Font::new(ctx, "/fonts/Merchant Copy.ttf")?;

        let player_team_state = TeamState { resources: 5 };

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
            player_team_state,
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

    fn try_add_trained_entity(
        &mut self,
        source_position: [u32; 2],
        source_size: [u32; 2],
    ) -> Option<[u32; 2]> {
        let left = source_position[0].saturating_sub(1);
        let top = source_position[1].saturating_sub(1);
        let right = min(
            source_position[0] + source_size[0],
            self.entity_grid.map_dimensions.0 - 1,
        );
        let bot = min(
            source_position[1] + source_size[1],
            self.entity_grid.map_dimensions.1 - 1,
        );
        for x in left..right + 1 {
            for y in top..bot + 1 {
                if !self.entity_grid.get(&[x, y]) {
                    let new_entity = data::create_player_unit([x, y]);
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
        self.entities.retain(|entity| {
            let is_dead = entity
                .health
                .as_ref()
                .map(|health| health.current == 0)
                .unwrap_or(false);
            if is_dead {
                if entity.is_solid {
                    // TODO set area?
                    let [w, h] = entity.size();
                    for x in entity.position[0]..entity.position[0] + w {
                        for y in entity.position[1]..entity.position[1] + h {
                            self.entity_grid.set(&[x, y], false);
                        }
                    }
                }
                if self.player_state.selected_entity_id == Some(entity.id) {
                    self.player_state.selected_entity_id = None;
                }
            }

            !is_dead
        });

        for entity in &mut self.entities {
            if let EntityType::Mobile(movement) = &mut entity.entity_type {
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
            if let EntityType::Mobile(movement) = &mut entity.entity_type {
                movement.sub_cell_movement.update(dt, entity.position);
            }
        }

        let mut completed_trainings = Vec::new();
        for entity in &mut self.entities {
            let status = entity
                .training_action
                .as_mut()
                .map(|training_action| training_action.update(dt));
            if status == Some(TrainingUpdateStatus::Done) {
                completed_trainings.push((entity.position, entity.size()));
            }
        }

        for (source_position, source_size) in completed_trainings {
            if self
                .try_add_trained_entity(source_position, source_size)
                .is_none()
            {
                eprintln!(
                    "Failed to create entity around {:?}, {:?}",
                    source_position, source_size
                );
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, COLOR_BG);

        graphics::draw(ctx, &self.assets.grid, DrawParam::new())?;

        for entity in &self.entities {
            let screen_coords = match &entity.entity_type {
                EntityType::Mobile(movement) => {
                    movement.sub_cell_movement.screen_coords(entity.position)
                }
                EntityType::Structure { .. } => grid_to_screen_coords(entity.position),
            };

            if self.player_state.selected_entity_id.as_ref() == Some(&entity.id) {
                self.assets
                    .draw_selection(ctx, entity.size(), screen_coords)?;
            }

            self.assets
                .draw_entity(ctx, &entity.sprite, screen_coords)?;
        }
        self.assets.flush_entity_sprite_batch(ctx)?;

        let selected_entity = self.selected_entity();
        self.hud
            .draw(ctx, &self.player_team_state, selected_entity)?;

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
                            .find(|e| {
                                let [w, h] = e.size();
                                clicked_pos[0] >= e.position[0]
                                    && clicked_pos[0] < e.position[0] + w
                                    && clicked_pos[1] >= e.position[1]
                                    && clicked_pos[1] < e.position[1] + h
                            })
                            .map(|e| e.id);
                        println!(
                            "Selected entity index: {:?}",
                            self.player_state.selected_entity_id
                        );
                    } else if let Some(entity) = self.selected_entity_mut() {
                        if entity.team == Team::Player {
                            match &mut entity.entity_type {
                                EntityType::Mobile(movement) => {
                                    movement.pathfinder.find_path(&entity.position, clicked_pos);
                                }
                                EntityType::Structure { .. } => {
                                    println!("Selected entity is immobile")
                                }
                            }
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
                let resources = self.player_team_state.resources;
                if let Some(entity) = self.selected_entity_mut() {
                    if entity.team == Team::Player {
                        if let Some(training_action) = &mut entity.training_action {
                            let cost = 1;
                            if resources >= cost {
                                if training_action.perform()
                                    == TrainingPerformStatus::NewTrainingStarted
                                {
                                    self.player_team_state.resources -= cost;
                                };
                            } else {
                                println!("Not enough resources!");
                            }
                        } else {
                            println!("Selected entity has no such action")
                        }
                    }
                }
            }
            KeyCode::X => {
                if let Some(entity) = self.selected_entity_mut() {
                    if entity.team == Team::Player {
                        if let Some(health) = &mut entity.health {
                            health.current = health.current.saturating_sub(1);
                        }
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
