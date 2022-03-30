use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, Font, Rect};
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};

use rand::rngs::ThreadRng;
use std::time::Duration;

use crate::assets::Assets;
use crate::camera::Camera;
use crate::core::{Command, Core};
use crate::data::{EntityType, MapType, WorldInitData};
use crate::enemy_ai::EnemyPlayerAi;
use crate::entities::{Action, Entity, EntityId, PhysicalType, Team};
use crate::hud_graphics::{HudGraphics, PlayerInput};

pub const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);

const WINDOW_DIMENSIONS: [f32; 2] = [1600.0, 1200.0];
pub const CELL_PIXEL_SIZE: [f32; 2] = [50.0, 50.0];
pub const WORLD_VIEWPORT: Rect = Rect {
    x: 50.0,
    y: 70.0,
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
    SelectAttackTarget,
    SelectMovementDestination,
    PlaceStructure(EntityType),
    SelectResourceTarget,
}

struct MovementCommandIndicator {
    world_pixel_position: [f32; 2],
    remaining: Duration,
}

impl MovementCommandIndicator {
    fn new() -> Self {
        Self {
            world_pixel_position: Default::default(),
            remaining: Default::default(),
        }
    }

    fn update(&mut self, dt: Duration) {
        self.remaining = self.remaining.checked_sub(dt).unwrap_or(Duration::ZERO);
    }

    fn set(&mut self, world_pixel_position: [f32; 2]) {
        self.world_pixel_position = world_pixel_position;
        self.remaining = Duration::from_secs_f32(0.5);
    }

    fn graphics(&self) -> Option<([f32; 2], f32)> {
        if !self.remaining.is_zero() {
            let scale = self.remaining.as_secs_f32() / 0.5;
            return Some((self.world_pixel_position, scale));
        }
        None
    }
}

pub struct PlayerState {
    selected_entity_id: Option<EntityId>,
    pub cursor_action: CursorAction, //TODO
    pub camera: Camera,              //TODO
    movement_command_indicator: MovementCommandIndicator,
}

impl PlayerState {
    fn new(camera: Camera) -> Self {
        Self {
            selected_entity_id: None,
            cursor_action: CursorAction::Default,
            camera,
            movement_command_indicator: MovementCommandIndicator::new(),
        }
    }

    fn set_cursor_action(&mut self, ctx: &mut Context, cursor_action: CursorAction) {
        match cursor_action {
            CursorAction::Default => mouse::set_cursor_type(ctx, CursorIcon::Default),
            CursorAction::SelectAttackTarget => mouse::set_cursor_type(ctx, CursorIcon::Crosshair),
            CursorAction::SelectMovementDestination => {
                mouse::set_cursor_type(ctx, CursorIcon::Move)
            }
            CursorAction::PlaceStructure(_) => mouse::set_cursor_type(ctx, CursorIcon::Grabbing),
            CursorAction::SelectResourceTarget => mouse::set_cursor_type(ctx, CursorIcon::Grab),
        }
        self.cursor_action = cursor_action;
    }

    fn screen_to_world(&self, coordinates: [f32; 2]) -> Option<[f32; 2]> {
        let [x, y] = coordinates;
        if !WORLD_VIEWPORT.contains(coordinates) {
            return None;
        }

        let camera_pos = self.camera.position_in_world;
        Some([
            x - WORLD_VIEWPORT.x + camera_pos[0],
            y - WORLD_VIEWPORT.y + camera_pos[1],
        ])
    }

    fn world_to_screen(&self, world_pixel_position: [f32; 2]) -> [f32; 2] {
        let [x, y] = world_pixel_position;
        let camera_pos = self.camera.position_in_world;
        [
            WORLD_VIEWPORT.x + x - camera_pos[0],
            WORLD_VIEWPORT.y + y - camera_pos[1],
        ]
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.camera.update(ctx, dt);
        self.movement_command_indicator.update(dt);
    }
}

struct Game {
    assets: Assets,
    hud: HudGraphics,
    player_state: PlayerState,
    enemy_player_ai: EnemyPlayerAi,
    rng: ThreadRng,
    core: Core,
}

impl Game {
    fn new(ctx: &mut Context, map_type: MapType) -> Result<Self, GameError> {
        let WorldInitData {
            dimensions: world_dimensions,
            entities,
        } = WorldInitData::new(map_type);

        println!("Created {} entities", entities.len());

        let assets = Assets::new(ctx, [WORLD_VIEWPORT.w, WORLD_VIEWPORT.h])?;

        let rng = rand::thread_rng();

        let enemy_player_ai = EnemyPlayerAi::new(world_dimensions);

        let font = Font::new(ctx, "/fonts/Merchant Copy.ttf")?;

        let max_camera_position = [
            world_dimensions[0] as f32 * CELL_PIXEL_SIZE[0] - WORLD_VIEWPORT.w,
            world_dimensions[1] as f32 * CELL_PIXEL_SIZE[1] - WORLD_VIEWPORT.h,
        ];
        let camera = Camera::new([0.0, 0.0], max_camera_position);
        let player_state = PlayerState::new(camera);

        let hud_pos = [WORLD_VIEWPORT.x, WORLD_VIEWPORT.y + WORLD_VIEWPORT.h + 25.0];
        let hud = HudGraphics::new(ctx, hud_pos, font, world_dimensions)?;

        let core = Core::new(entities, world_dimensions);

        Ok(Self {
            assets,
            hud,
            player_state,
            enemy_player_ai,
            rng,
            core,
        })
    }

    fn selected_entity(&self) -> Option<&Entity> {
        self.player_state.selected_entity_id.map(|id| {
            self.core
                .entities()
                .iter()
                .find(|e| e.id == id)
                .expect("selected entity must exist")
        })
    }

    fn selected_player_entity(&self) -> Option<&Entity> {
        self.selected_entity()
            .filter(|entity| entity.team == Team::Player)
    }

    fn handle_player_entity_action(
        &mut self,
        ctx: &mut Context,
        actor_id: EntityId,
        action: Action,
    ) {
        match action {
            Action::Train(unit_type, training_config) => {
                self.core.issue_command(
                    Command::Train(actor_id, unit_type, training_config),
                    Team::Player,
                );
            }
            Action::Construct(structure_type) => {
                self.player_state
                    .set_cursor_action(ctx, CursorAction::PlaceStructure(structure_type));
            }
            Action::Move => {
                self.player_state
                    .set_cursor_action(ctx, CursorAction::SelectMovementDestination);
            }
            Action::Heal => {
                self.core
                    .issue_command(Command::Heal(actor_id), Team::Player);
            }
            Action::Attack => {
                self.player_state
                    .set_cursor_action(ctx, CursorAction::SelectAttackTarget);
            }
            Action::GatherResource => {
                self.player_state
                    .set_cursor_action(ctx, CursorAction::SelectResourceTarget);
            }
        }
    }

    fn set_player_camera_position(&mut self, x_ratio: f32, y_ratio: f32) {
        self.player_state.camera.position_in_world = [
            x_ratio * self.core.dimensions()[0] as f32 * CELL_PIXEL_SIZE[0]
                - WORLD_VIEWPORT.w / 2.0,
            y_ratio * self.core.dimensions()[1] as f32 * CELL_PIXEL_SIZE[1]
                - WORLD_VIEWPORT.h / 2.0,
        ];
    }

    fn enemy_at_position(&self, clicked_world_pos: [u32; 2]) -> Option<EntityId> {
        self.core
            .entities()
            .iter()
            .find(|e| e.contains(clicked_world_pos) && e.health.is_some() && e.team == Team::Enemy)
            .map(|e| e.id)
    }

    fn resource_at_position(&self, clicked_world_pos: [u32; 2]) -> Option<EntityId> {
        // TODO we assume that all neutral entities are resources for now
        self.core
            .entities()
            .iter()
            .find(|e| e.contains(clicked_world_pos) && e.team == Team::Neutral)
            .map(|e| e.id)
    }

    fn set_selected_entity(&mut self, entity_id: Option<EntityId>) {
        self.player_state.selected_entity_id = entity_id;
        if let Some(entity) = self.selected_entity() {
            let actions = entity.actions;
            self.hud.set_entity_actions(actions);
        }
    }

    fn handle_player_input(&mut self, ctx: &mut Context, player_input: PlayerInput) {
        match player_input {
            PlayerInput::UseEntityAction(i) => {
                if let Some(entity) = self.selected_player_entity() {
                    if let Some(action) = entity.actions[i] {
                        let entity_id = entity.id;
                        self.handle_player_entity_action(ctx, entity_id, action);
                    }
                }
            }
            PlayerInput::SetCameraPositionRelativeToWorldDimension([x_ratio, y_ratio]) => {
                self.set_player_camera_position(x_ratio, y_ratio);
            }
        }
    }

    fn issue_player_movement_command(
        &mut self,
        world_pixel_coordinates: [f32; 2],
        entity_id: EntityId,
    ) {
        self.player_state
            .movement_command_indicator
            .set(world_pixel_coordinates);
        let destination = world_to_grid(world_pixel_coordinates);
        self.core
            .issue_command(Command::Move(entity_id, destination), Team::Player);
    }

    fn screen_to_grid(&self, coordinates: [f32; 2]) -> Option<[u32; 2]> {
        self.player_state
            .screen_to_world(coordinates)
            .map(world_to_grid)
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let fps = ggez::timer::fps(ctx) as u32;
        graphics::set_window_title(ctx, &format!("{} (fps={})", TITLE, fps));

        let dt = ggez::timer::delta(ctx);

        let enemy_commands = self
            .enemy_player_ai
            .run(dt, self.core.entities(), &mut self.rng);
        if !enemy_commands.is_empty() {
            println!("Issuing {} AI commands:", enemy_commands.len());
            for command in enemy_commands {
                println!("  {:?}", command);
                self.core.issue_command(command, Team::Enemy);
            }
        }

        let removed_entity_ids = self.core.update(dt);

        for removed_entity_id in removed_entity_ids {
            if self.player_state.selected_entity_id == Some(removed_entity_id) {
                self.set_selected_entity(None);
                self.player_state
                    .set_cursor_action(ctx, CursorAction::Default);
            }
        }

        self.player_state.update(ctx, dt);

        if let Some(hovered_world_pos) =
            self.screen_to_grid(ggez::input::mouse::position(ctx).into())
        {
            if self.player_state.cursor_action == CursorAction::Default {
                let is_hovering_some_entity = self
                    .core
                    .entities()
                    .iter()
                    .any(|e| e.contains(hovered_world_pos));
                let icon = if is_hovering_some_entity {
                    CursorIcon::Hand
                } else {
                    CursorIcon::Default
                };
                mouse::set_cursor_type(ctx, icon);
            }
        }

        self.hud.update(dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, COLOR_BG);

        self.assets.draw_grid(
            ctx,
            WORLD_VIEWPORT.point().into(),
            self.player_state.camera.position_in_world,
        )?;

        let mouse_position: [f32; 2] = ggez::input::mouse::position(ctx).into();
        if let CursorAction::PlaceStructure(structure_type) = self.player_state.cursor_action {
            if let Some(hovered_world_pos) = self.screen_to_grid(mouse_position) {
                let size = *self.core.structure_size(&structure_type);
                let world_coords = grid_to_world(hovered_world_pos);
                let screen_coords = self.player_state.world_to_screen(world_coords);
                // TODO: Draw transparent filled rect instead of selection outline
                self.assets
                    .draw_selection(ctx, size, Team::Player, screen_coords)?;
            }
        }

        let indicator = &self.player_state.movement_command_indicator;
        if let Some((world_pixel_position, scale)) = indicator.graphics() {
            let screen_coords = self.player_state.world_to_screen(world_pixel_position);
            self.assets
                .draw_movement_command_indicator(ctx, screen_coords, scale)?;
        }

        for entity in self.core.entities() {
            let screen_coords = self
                .player_state
                .world_to_screen(entity.world_pixel_position());

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

        self.hud.draw(
            ctx,
            self.core.team_state(&Team::Player),
            self.selected_entity(),
            &self.player_state,
        )?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let Some(clicked_world_pixel_coords) = self.player_state.screen_to_world([x, y]) {
            let clicked_world_pos = world_to_grid(clicked_world_pixel_coords);
            match self.player_state.cursor_action {
                CursorAction::Default => {
                    if button == MouseButton::Left {
                        // TODO (bug) Don't select neutral entity when player unit is on top of it
                        let selected_entity_id = self
                            .core
                            .entities()
                            .iter()
                            .find(|e| e.contains(clicked_world_pos))
                            .map(|e| e.id);
                        self.set_selected_entity(selected_entity_id);
                    } else if let Some(entity) = self.selected_player_entity() {
                        match &entity.physical_type {
                            PhysicalType::Unit(unit) => {
                                let entity_id = entity.id;
                                if unit.combat.is_some() {
                                    if let Some(victim_id) =
                                        self.enemy_at_position(clicked_world_pos)
                                    {
                                        // TODO: highlight attacked entity temporarily
                                        self.core.issue_command(
                                            Command::Attack(entity_id, victim_id),
                                            Team::Player,
                                        );
                                        return;
                                    }
                                }
                                if entity.actions.contains(&Some(Action::GatherResource)) {
                                    if let Some(resource_id) =
                                        self.resource_at_position(clicked_world_pos)
                                    {
                                        self.core.issue_command(
                                            Command::GatherResource(entity_id, resource_id),
                                            Team::Player,
                                        );
                                        return;
                                    }
                                }
                                self.issue_player_movement_command(
                                    clicked_world_pixel_coords,
                                    entity_id,
                                );
                            }
                            PhysicalType::Structure { .. } => {
                                println!("Selected entity is immobile")
                            }
                        }
                    } else {
                        println!("No entity is selected");
                    }
                }

                CursorAction::SelectMovementDestination => {
                    let entity_id = self
                        .player_state
                        .selected_entity_id
                        .expect("Cannot issue movement without selected entity");
                    self.issue_player_movement_command(clicked_world_pixel_coords, entity_id);
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }

                CursorAction::PlaceStructure(structure_type) => {
                    let entity_id = self
                        .player_state
                        .selected_entity_id
                        .expect("Cannot issue construction without selected entity");
                    self.core.issue_command(
                        Command::Construct(entity_id, clicked_world_pos, structure_type),
                        Team::Player,
                    );
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }

                CursorAction::SelectAttackTarget => {
                    if let Some(victim_id) = self.enemy_at_position(clicked_world_pos) {
                        let attacker_id = self
                            .player_state
                            .selected_entity_id
                            .expect("Cannot attack without selected entity");
                        // TODO: highlight attacked entity temporarily
                        self.core
                            .issue_command(Command::Attack(attacker_id, victim_id), Team::Player);
                    } else {
                        println!("Invalid attack target");
                    }
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }
                CursorAction::SelectResourceTarget => {
                    if let Some(resource_id) = self.resource_at_position(clicked_world_pos) {
                        let gatherer_id = self
                            .player_state
                            .selected_entity_id
                            .expect("Cannot gather without selected entity");
                        self.core.issue_command(
                            Command::GatherResource(gatherer_id, resource_id),
                            Team::Player,
                        );
                    } else {
                        println!("Invalid resource target");
                    }
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }
            }
        } else {
            self.player_state
                .set_cursor_action(ctx, CursorAction::Default);

            if let Some(player_input) = self.hud.on_mouse_button_down(button, x, y) {
                self.handle_player_input(ctx, player_input)
            }
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        self.hud.on_mouse_button_up(button);
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        if let Some(player_input) = self.hud.on_mouse_motion(x, y) {
            self.handle_player_input(ctx, player_input);
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
                if let Some(player_input) = self.hud.on_key_down(keycode) {
                    self.handle_player_input(ctx, player_input);
                }
            }
        }
    }
}

pub fn grid_to_world(grid_position: [u32; 2]) -> [f32; 2] {
    [
        grid_position[0] as f32 * CELL_PIXEL_SIZE[0],
        grid_position[1] as f32 * CELL_PIXEL_SIZE[1],
    ]
}

fn world_to_grid(world_coordinates: [f32; 2]) -> [u32; 2] {
    let grid_x = world_coordinates[0] / CELL_PIXEL_SIZE[0];
    let grid_y = world_coordinates[1] / CELL_PIXEL_SIZE[1];
    let grid_x = grid_x as u32;
    let grid_y = grid_y as u32;
    [grid_x, grid_y]
}
