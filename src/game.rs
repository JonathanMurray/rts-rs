use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, Font, Rect};
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
    ActionType, Entity, EntityId, EntityState, PhysicalType, Team, TrainingConfig,
    TrainingPerformStatus, TrainingUpdateStatus,
};
use crate::grid::EntityGrid;
use crate::hud_graphics::{HudGraphics, MinimapGraphics};

pub const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);

const WINDOW_DIMENSIONS: [f32; 2] = [1600.0, 1200.0];
pub const CELL_PIXEL_SIZE: [f32; 2] = [50.0, 50.0];
pub const WORLD_VIEWPORT: Rect = Rect {
    x: 50.0,
    y: 50.0,
    w: WINDOW_DIMENSIONS[0] - 100.0,
    h: 650.0,
};

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

#[derive(PartialEq, Copy, Clone)]
pub enum CursorAction {
    Default,
    DealDamage,
    IssueMovement,
}

struct PlayerState {
    selected_entity_id: Option<EntityId>,
    cursor_action: CursorAction,
    camera: Camera,
}

impl PlayerState {
    fn set_cursor_action(&mut self, ctx: &mut Context, cursor_action: CursorAction) {
        match cursor_action {
            CursorAction::Default => mouse::set_cursor_type(ctx, CursorIcon::Default),
            CursorAction::DealDamage => mouse::set_cursor_type(ctx, CursorIcon::Crosshair),
            CursorAction::IssueMovement => mouse::set_cursor_type(ctx, CursorIcon::Move),
        }
        self.cursor_action = cursor_action;
    }
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

        let assets = assets::create_assets(ctx, [WORLD_VIEWPORT.w, WORLD_VIEWPORT.h])?;

        let rng = rand::thread_rng();

        let mut entity_grid = EntityGrid::new(map_dimensions);
        for entity in &entities {
            if entity.is_solid {
                entity_grid.set_area(&entity.position, &entity.size(), true);
            }
        }

        let enemy_player_ai = EnemyPlayerAi::new(map_dimensions);

        let font = Font::new(ctx, "/fonts/Merchant Copy.ttf")?;

        let mut teams = HashMap::new();
        teams.insert(Team::Player, TeamState { resources: 5 });
        teams.insert(Team::Enemy, TeamState { resources: 5 });

        let max_camera_position = [
            map_dimensions[0] as f32 * CELL_PIXEL_SIZE[0] - WORLD_VIEWPORT.w,
            map_dimensions[1] as f32 * CELL_PIXEL_SIZE[1] - WORLD_VIEWPORT.h,
        ];
        let camera = Camera::new([0.0, 0.0], max_camera_position);
        let player_state = PlayerState {
            selected_entity_id: None,
            cursor_action: CursorAction::Default,
            camera,
        };

        let hud_pos = [WORLD_VIEWPORT.x, WORLD_VIEWPORT.y + WORLD_VIEWPORT.h + 25.0];
        let minimap_pos = [900.0, hud_pos[1] + 100.0];
        let hud = HudGraphics::new(ctx, hud_pos, font)?;
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
        if x < WORLD_VIEWPORT.x || y < WORLD_VIEWPORT.y {
            return None;
        }
        if x >= WORLD_VIEWPORT.x + WORLD_VIEWPORT.w || y >= WORLD_VIEWPORT.y + WORLD_VIEWPORT.h {
            return None;
        }

        let camera_pos = self.player_state.camera.position_in_world;
        let grid_x = (x - WORLD_VIEWPORT.x + camera_pos[0]) / CELL_PIXEL_SIZE[0];
        let grid_y = (y - WORLD_VIEWPORT.y + camera_pos[1]) / CELL_PIXEL_SIZE[1];
        let grid_x = grid_x as u32;
        let grid_y = grid_y as u32;
        if grid_x < self.entity_grid.dimensions[0] && grid_y < self.entity_grid.dimensions[1] {
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
        self.player_state
            .selected_entity_id
            .map(|id| self.entity_mut(id))
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
            self.entity_grid.dimensions[0] - 1,
        );
        let bot = min(
            source_position[1] + source_size[1],
            self.entity_grid.dimensions[1] - 1,
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

    fn try_perform_player_action(&mut self, ctx: &mut Context, action_type: ActionType) {
        match action_type {
            ActionType::Train(trained_entity_type, training_config) => {
                let entity_id = self
                    .player_state
                    .selected_entity_id
                    .expect("Need selected entity to train");
                self.apply_command(
                    Command::Train(entity_id, trained_entity_type, training_config),
                    Team::Player,
                );
            }
            ActionType::Move => {
                self.player_state
                    .set_cursor_action(ctx, CursorAction::IssueMovement);
            }
            ActionType::Heal => {
                let entity_id = self
                    .player_state
                    .selected_entity_id
                    .expect("Need selected entity to health");
                self.apply_command(Command::Heal(entity_id), Team::Player);
            }
            ActionType::Harm => {
                self.player_state
                    .set_cursor_action(ctx, CursorAction::DealDamage);
            }
        }
    }

    fn apply_command(&mut self, command: Command, issuing_team: Team) {
        match command {
            Command::Train(active_entity_id, trained_entity_type, config) => {
                let resources = self.teams.get(&issuing_team).unwrap().resources;
                let entity = self.entity_mut(active_entity_id);
                assert_eq!(entity.team, issuing_team);
                let training = entity
                    .training
                    .as_mut()
                    .expect("Training command was issued for entity that can't train");
                if resources >= config.cost {
                    if let TrainingPerformStatus::NewTrainingStarted =
                        training.start(trained_entity_type)
                    {
                        entity.state = EntityState::TrainingUnit(trained_entity_type);
                        self.teams.get_mut(&issuing_team).unwrap().resources -= config.cost;
                    }
                }
            }
            Command::Move(active_entity_id, current_pos, destination) => {
                let entity = self.entity_mut(active_entity_id);
                assert_eq!(entity.team, issuing_team);
                match &mut entity.physical_type {
                    PhysicalType::Mobile(movement) => {
                        movement.pathfinder.find_path(&current_pos, destination);
                    }
                    PhysicalType::Structure { .. } => {
                        panic!("Move command was issued for structure")
                    }
                }
            }
            Command::Heal(active_entity_id) => {
                let entity = self.entity_mut(active_entity_id);
                assert_eq!(entity.team, issuing_team);
                entity
                    .actions
                    .iter()
                    .find(|action| **action == Some(ActionType::Heal))
                    .expect("Heal command was issued for entity that doesn't have a Heal action");
                let health = entity.health.as_mut().unwrap();
                health.receive_healing(1);
            }
            Command::DealDamage(active_entity_id, target_entity_id) => {
                let dealer_entity = self.entity_mut(active_entity_id);
                assert_eq!(dealer_entity.team, issuing_team);
                dealer_entity
                    .actions
                    .iter()
                    .find(|action| **action == Some(ActionType::Harm))
                    .expect("Heal command was issued for entity that doesn't have a Harm action");
                let target_entity = self.entity_mut(target_entity_id);
                let health = target_entity
                    .health
                    .as_mut()
                    .expect("Damage command was targeted at entity that has no health");
                health.receive_damage(1);
                println!("Reduced health down to {}/{}", health.current, health.max)
            }
        }
    }

    fn entity_mut(&mut self, id: EntityId) -> &mut Entity {
        self.entities
            .iter_mut()
            .find(|e| e.id == id)
            .expect("entity must exist")
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let fps = ggez::timer::fps(ctx) as u32;
        graphics::set_window_title(ctx, &format!("{} (fps={})", TITLE, fps));

        let dt = ggez::timer::delta(ctx);

        let enemy_commands = self
            .enemy_player_ai
            .run(dt, &self.entities[..], &mut self.rng);
        if !enemy_commands.is_empty() {
            println!("Applying {} AI commands:", enemy_commands.len());
            for command in enemy_commands {
                println!("  {:?}", command);
                self.apply_command(command, Team::Enemy);
            }
        }

        // Remove dead entities
        self.entities.retain(|entity| {
            let is_dead = entity
                .health
                .as_ref()
                .map(|health| health.current == 0)
                .unwrap_or(false);
            if is_dead {
                if entity.is_solid {
                    self.entity_grid
                        .set_area(&entity.position, &entity.size(), false);
                }
                if self.player_state.selected_entity_id == Some(entity.id) {
                    self.player_state.selected_entity_id = None;
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }
            }

            !is_dead
        });

        for entity in &mut self.entities {
            if let PhysicalType::Mobile(movement) = &mut entity.physical_type {
                if movement.sub_cell_movement.is_ready() {
                    if let Some(next_pos) = movement.pathfinder.peek_path() {
                        entity.state = EntityState::Moving;
                        let occupied = self.entity_grid.get(next_pos);
                        if !occupied {
                            let old_pos = entity.position;
                            let new_pos = movement.pathfinder.advance_path();
                            self.entity_grid.set(&old_pos, false);
                            movement.sub_cell_movement.set_moving(old_pos, new_pos);
                            entity.position = new_pos;
                            self.entity_grid.set(&new_pos, true);
                        }
                    } else {
                        entity.state = EntityState::Idle;
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
            let status = entity.training.as_mut().map(|training| training.update(dt));
            if let Some(TrainingUpdateStatus::Done(trained_entity_type)) = status {
                entity.state = EntityState::Idle;
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
            WORLD_VIEWPORT.point().into(),
            self.player_state.camera.position_in_world,
        )?;

        let offset = [
            WORLD_VIEWPORT.x - self.player_state.camera.position_in_world[0],
            WORLD_VIEWPORT.y - self.player_state.camera.position_in_world[1],
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
                    .draw_selection(ctx, entity.size(), entity.team, screen_coords)?;
            }

            self.assets
                .draw_entity(ctx, entity.sprite, entity.team, screen_coords)?;
        }
        self.assets.flush_entity_sprite_batch(ctx)?;

        self.assets
            .draw_background_around_grid(ctx, WORLD_VIEWPORT.point().into())?;

        let selected_entity = self.selected_entity();
        self.hud.draw(
            ctx,
            self.teams.get(&Team::Player).unwrap(),
            selected_entity,
            self.player_state.cursor_action,
            ggez::input::mouse::position(ctx).into(),
        )?;
        self.minimap
            .draw(ctx, self.player_state.camera.position_in_world)?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let Some(clicked_world_pos) = self.screen_to_grid_coordinates([x, y]) {
            match self.player_state.cursor_action {
                CursorAction::Default => {
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
                                PhysicalType::Mobile(..) => {
                                    let entity_id = entity.id;
                                    let current_pos = entity.position;
                                    self.apply_command(
                                        Command::Move(entity_id, current_pos, clicked_world_pos),
                                        Team::Player,
                                    );
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
                CursorAction::DealDamage => {
                    // TODO this only works for structures' top-left corner
                    if let Some(target_entity) = self
                        .entities
                        .iter_mut()
                        .find(|e| e.position == clicked_world_pos && e.health.is_some())
                    {
                        let target_entity_id = target_entity.id;
                        let dealer_entity_id = self
                            .player_state
                            .selected_entity_id
                            .expect("Can't deal damage without selected entity");
                        self.apply_command(
                            Command::DealDamage(dealer_entity_id, target_entity_id),
                            Team::Player,
                        );
                    }
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }
                CursorAction::IssueMovement => {
                    let entity = self
                        .selected_entity_mut()
                        .expect("Cannot issue movement without selected entity");
                    assert_eq!(entity.team, Team::Player);
                    match &mut entity.physical_type {
                        PhysicalType::Mobile(..) => {
                            let entity_id = entity.id;
                            let current_pos = entity.position;
                            self.apply_command(
                                Command::Move(entity_id, current_pos, clicked_world_pos),
                                Team::Player,
                            );
                        }
                        PhysicalType::Structure { .. } => {
                            panic!("Cannot issue movement for structure")
                        }
                    }
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }
            }
        } else {
            let minimap = self.minimap.rect();
            if minimap.contains([x, y]) {
                self.player_state.camera.position_in_world = [
                    ((x - minimap.x) / minimap.w)
                        * self.entity_grid.dimensions[0] as f32
                        * CELL_PIXEL_SIZE[0]
                        - WORLD_VIEWPORT.w / 2.0,
                    ((y - minimap.y) / minimap.h)
                        * self.entity_grid.dimensions[1] as f32
                        * CELL_PIXEL_SIZE[1]
                        - WORLD_VIEWPORT.h / 2.0,
                ];
            }

            if let Some(entity) = self.selected_entity() {
                if entity.team == Team::Player {
                    if let Some(action_type) = self.hud.on_mouse_click([x, y], entity) {
                        self.try_perform_player_action(ctx, action_type);
                    }
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
            _ => {
                if let Some(entity) = self.selected_entity() {
                    if entity.team == Team::Player {
                        if let Some(action_type) = self.hud.on_button_click(keycode, entity) {
                            self.try_perform_player_action(ctx, action_type);
                        }
                    }
                }
            }
        }
    }
}

pub fn grid_to_pixel_position(grid_position: [u32; 2]) -> [f32; 2] {
    [
        grid_position[0] as f32 * CELL_PIXEL_SIZE[0],
        grid_position[1] as f32 * CELL_PIXEL_SIZE[1],
    ]
}

#[derive(Debug)]
pub enum Command {
    Train(EntityId, EntityType, TrainingConfig),
    Move(EntityId, [u32; 2], [u32; 2]),
    Heal(EntityId),
    DealDamage(EntityId, EntityId),
}
