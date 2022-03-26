use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, Font};
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};

use rand::rngs::ThreadRng;
use std::cmp::min;
use std::collections::HashMap;

use crate::assets::{self, Assets};
use crate::camera::Camera;
use crate::data::{self, EntityType, Map, MapType};
use crate::enemy_ai::EnemyPlayerAi;
use crate::entities::{
    Entity, EntityId, PhysicalType, Team, TrainingPerformStatus, TrainingUpdateStatus,
};
use crate::hud_graphics::{HudGraphics, MinimapGraphics};

pub const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);

const WINDOW_DIMENSIONS: [f32; 2] = [1600.0, 1200.0];
pub const CELL_PIXEL_SIZE: [f32; 2] = [50.0, 50.0];
const WORLD_POSITION_ON_SCREEN: [f32; 2] = [100.0, 100.0];
pub const CAMERA_SIZE: [f32; 2] = [
    WINDOW_DIMENSIONS[0] - WORLD_POSITION_ON_SCREEN[0] * 2.0,
    700.0,
];

const TITLE: &str = "RTS";

pub fn run(map_type: MapType) -> GameResult {
    let (mut ctx, event_loop) = ContextBuilder::new("rts", "jm")
        .window_setup(WindowSetup::default().title(TITLE))
        .window_mode(WindowMode::default().dimensions(WINDOW_DIMENSIONS[0], WINDOW_DIMENSIONS[1]))
        .add_resource_path("resources")
        .build()
        .expect("Creating ggez context");

    let game = Game::new(&mut ctx, map_type)?;
    ggez::event::run(ctx, event_loop, game)
}

struct EntityGrid {
    grid: Vec<bool>,
    map_dimensions: [u32; 2],
}

impl EntityGrid {
    fn new(map_dimensions: [u32; 2]) -> Self {
        let grid = vec![false; (map_dimensions[0] * map_dimensions[1]) as usize];
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
        (y * self.map_dimensions[0] + x) as usize
    }
}

enum MouseState {
    Default,
    DealingDamage,
}

struct PlayerState {
    selected_entity_id: Option<EntityId>,
    mouse_state: MouseState,
    camera: Camera,
}

pub struct TeamState {
    pub resources: u32,
}

struct Game {
    assets: Assets,
    hud: HudGraphics,
    minimap: MinimapGraphics,
    teams: HashMap<Team, TeamState>,
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

        let assets = assets::create_assets(ctx, CAMERA_SIZE)?;

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

        let mut teams = HashMap::new();
        teams.insert(Team::Player, TeamState { resources: 5 });
        teams.insert(Team::Enemy, TeamState { resources: 5 });

        let max_camera_position = [
            map_dimensions[0] as f32 * CELL_PIXEL_SIZE[0] - CAMERA_SIZE[0],
            map_dimensions[1] as f32 * CELL_PIXEL_SIZE[1] - CAMERA_SIZE[1],
        ];
        let camera = Camera::new([0.0, 0.0], max_camera_position);
        let player_state = PlayerState {
            selected_entity_id: None,
            mouse_state: MouseState::Default,
            camera,
        };

        let hud_pos = [
            WORLD_POSITION_ON_SCREEN[0],
            WORLD_POSITION_ON_SCREEN[1] + CAMERA_SIZE[1] + 25.0,
        ];
        let minimap_pos = [900.0, hud_pos[1] + 100.0];
        let hud = HudGraphics::new(hud_pos, font);
        let minimap = MinimapGraphics::new(ctx, minimap_pos, map_dimensions)?;

        Ok(Self {
            assets,
            hud,
            minimap,
            teams,
            player_state,
            entities,
            entity_grid,
            enemy_player_ai,
            rng,
        })
    }

    fn screen_to_grid_coordinates(&self, coordinates: [f32; 2]) -> Option<[u32; 2]> {
        let [x, y] = coordinates;
        if x < WORLD_POSITION_ON_SCREEN[0] || y < WORLD_POSITION_ON_SCREEN[1] {
            println!("Top/left of the game area on screen");
            return None;
        }
        if x >= WORLD_POSITION_ON_SCREEN[0] + CAMERA_SIZE[0]
            || y >= WORLD_POSITION_ON_SCREEN[1] + CAMERA_SIZE[1]
        {
            println!("Bot/right of the game area on screen");
            return None;
        }

        let camera_pos = self.player_state.camera.position_in_world;
        println!("Camera pos: {:?}", camera_pos);
        let grid_x = (x - WORLD_POSITION_ON_SCREEN[0] + camera_pos[0]) / CELL_PIXEL_SIZE[0];
        let grid_y = (y - WORLD_POSITION_ON_SCREEN[1] + camera_pos[1]) / CELL_PIXEL_SIZE[1];
        let grid_x = grid_x as u32;
        let grid_y = grid_y as u32;
        if grid_x < self.entity_grid.map_dimensions[0]
            && grid_y < self.entity_grid.map_dimensions[1]
        {
            Some([grid_x, grid_y])
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
        entity_type: EntityType,
        team: Team,
        source_position: [u32; 2],
        source_size: [u32; 2],
    ) -> Option<[u32; 2]> {
        let left = source_position[0].saturating_sub(1);
        let top = source_position[1].saturating_sub(1);
        let right = min(
            source_position[0] + source_size[0],
            self.entity_grid.map_dimensions[0] - 1,
        );
        let bot = min(
            source_position[1] + source_size[1],
            self.entity_grid.map_dimensions[1] - 1,
        );
        for x in left..right + 1 {
            for y in top..bot + 1 {
                if !self.entity_grid.get(&[x, y]) {
                    let new_entity = data::create_entity(entity_type, [x, y], team);
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

        self.enemy_player_ai.run(
            dt,
            &mut self.entities[..],
            &mut self.rng,
            self.teams.get_mut(&Team::Enemy).unwrap(),
        );

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
            if let PhysicalType::Mobile(movement) = &mut entity.physical_type {
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
            if let PhysicalType::Mobile(movement) = &mut entity.physical_type {
                movement.sub_cell_movement.update(dt, entity.position);
            }
        }

        let mut completed_trainings = Vec::new();
        for entity in &mut self.entities {
            let status = entity
                .training_action
                .as_mut()
                .map(|training_action| training_action.update(dt));
            if let Some(TrainingUpdateStatus::Done(trained_entity_type)) = status {
                completed_trainings.push((
                    trained_entity_type,
                    entity.team,
                    entity.position,
                    entity.size(),
                ));
            }
        }

        for (entity_type, team, source_position, source_size) in completed_trainings {
            if self
                .try_add_trained_entity(entity_type, team, source_position, source_size)
                .is_none()
            {
                eprintln!(
                    "Failed to create entity around {:?}, {:?}",
                    source_position, source_size
                );
            }
        }

        self.player_state.camera.update(ctx, dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, COLOR_BG);

        self.assets.draw_grid(
            ctx,
            WORLD_POSITION_ON_SCREEN,
            self.player_state.camera.position_in_world,
        )?;

        let offset = [
            WORLD_POSITION_ON_SCREEN[0] - self.player_state.camera.position_in_world[0],
            WORLD_POSITION_ON_SCREEN[1] - self.player_state.camera.position_in_world[1],
        ];

        for entity in &self.entities {
            let pixel_pos = match &entity.physical_type {
                PhysicalType::Mobile(movement) => {
                    movement.sub_cell_movement.pixel_position(entity.position)
                }
                PhysicalType::Structure { .. } => grid_to_pixel_position(entity.position),
            };

            let screen_coords = [offset[0] + pixel_pos[0], offset[1] + pixel_pos[1]];

            if self.player_state.selected_entity_id.as_ref() == Some(&entity.id) {
                self.assets
                    .draw_selection(ctx, entity.size(), screen_coords)?;
            }

            self.assets
                .draw_entity(ctx, &entity.sprite, screen_coords)?;
        }
        self.assets.flush_entity_sprite_batch(ctx)?;

        self.assets
            .draw_background_around_grid(ctx, WORLD_POSITION_ON_SCREEN)?;

        let selected_entity = self.selected_entity();
        self.hud
            .draw(ctx, self.teams.get(&Team::Player).unwrap(), selected_entity)?;
        self.minimap
            .draw(ctx, self.player_state.camera.position_in_world)?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let Some(clicked_world_pos) = self.screen_to_grid_coordinates([x, y]) {
            match self.player_state.mouse_state {
                MouseState::Default => {
                    if button == MouseButton::Left {
                        // TODO (bug) Don't select neutral entity when player unit is on top of it
                        self.player_state.selected_entity_id = self
                            .entities
                            .iter()
                            .find(|e| {
                                let [w, h] = e.size();
                                clicked_world_pos[0] >= e.position[0]
                                    && clicked_world_pos[0] < e.position[0] + w
                                    && clicked_world_pos[1] >= e.position[1]
                                    && clicked_world_pos[1] < e.position[1] + h
                            })
                            .map(|e| e.id);
                        println!(
                            "Selected entity index: {:?}",
                            self.player_state.selected_entity_id
                        );
                    } else if let Some(entity) = self.selected_entity_mut() {
                        if entity.team == Team::Player {
                            match &mut entity.physical_type {
                                PhysicalType::Mobile(movement) => {
                                    movement
                                        .pathfinder
                                        .find_path(&entity.position, clicked_world_pos);
                                }
                                PhysicalType::Structure { .. } => {
                                    println!("Selected entity is immobile")
                                }
                            }
                        }
                    } else {
                        println!("No entity is selected");
                    }
                }
                MouseState::DealingDamage => {
                    // TODO this only works for structures' top-left corner
                    if let Some(mut health) = self
                        .entities
                        .iter_mut()
                        .filter(|e| e.position == clicked_world_pos)
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
        } else {
            let minimap = self.minimap.rect();
            if minimap.contains([x, y]) {
                self.player_state.camera.position_in_world = [
                    ((x - minimap.x) / minimap.w)
                        * self.entity_grid.map_dimensions[0] as f32
                        * CELL_PIXEL_SIZE[0]
                        - CAMERA_SIZE[0] / 2.0,
                    ((y - minimap.y) / minimap.h)
                        * self.entity_grid.map_dimensions[1] as f32
                        * CELL_PIXEL_SIZE[1]
                        - CAMERA_SIZE[1] / 2.0,
                ];
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
                let resources = self.teams.get(&Team::Player).unwrap().resources;
                if let Some(entity) = self.selected_entity_mut() {
                    if entity.team == Team::Player {
                        if let Some(training_action) = &mut entity.training_action {
                            let cost = training_action.cost();
                            if resources >= cost {
                                if training_action.perform()
                                    == TrainingPerformStatus::NewTrainingStarted
                                {
                                    self.teams.get_mut(&Team::Player).unwrap().resources -= cost;
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

pub fn grid_to_pixel_position(grid_position: [u32; 2]) -> [f32; 2] {
    [
        grid_position[0] as f32 * CELL_PIXEL_SIZE[0],
        grid_position[1] as f32 * CELL_PIXEL_SIZE[1],
    ]
}
