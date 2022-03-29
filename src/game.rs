use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, Font, Rect};
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};

use rand::rngs::ThreadRng;

use crate::assets::{self, Assets};
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
    SelectAttackTarget,
    SelectMovementDestination,
    PlaceStructure(EntityType),
}

pub struct PlayerState {
    selected_entity_id: Option<EntityId>,
    pub cursor_action: CursorAction, //TODO
    pub camera: Camera,              //TODO
}

impl PlayerState {
    fn set_cursor_action(&mut self, ctx: &mut Context, cursor_action: CursorAction) {
        match cursor_action {
            CursorAction::Default => mouse::set_cursor_type(ctx, CursorIcon::Default),
            CursorAction::SelectAttackTarget => mouse::set_cursor_type(ctx, CursorIcon::Crosshair),
            CursorAction::SelectMovementDestination => {
                mouse::set_cursor_type(ctx, CursorIcon::Move)
            }
            CursorAction::PlaceStructure(_) => mouse::set_cursor_type(ctx, CursorIcon::Grabbing),
        }
        self.cursor_action = cursor_action;
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

        let assets = assets::create_assets(ctx, [WORLD_VIEWPORT.w, WORLD_VIEWPORT.h])?;

        let rng = rand::thread_rng();

        let enemy_player_ai = EnemyPlayerAi::new(world_dimensions);

        let font = Font::new(ctx, "/fonts/Merchant Copy.ttf")?;

        let max_camera_position = [
            world_dimensions[0] as f32 * CELL_PIXEL_SIZE[0] - WORLD_VIEWPORT.w,
            world_dimensions[1] as f32 * CELL_PIXEL_SIZE[1] - WORLD_VIEWPORT.h,
        ];
        let camera = Camera::new([0.0, 0.0], max_camera_position);
        let player_state = PlayerState {
            selected_entity_id: None,
            cursor_action: CursorAction::Default,
            camera,
        };

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

    fn screen_to_grid_coordinates(&self, coordinates: [f32; 2]) -> Option<[u32; 2]> {
        let [x, y] = coordinates;
        if !WORLD_VIEWPORT.contains(coordinates) {
            return None;
        }

        let camera_pos = self.player_state.camera.position_in_world;
        let grid_x = (x - WORLD_VIEWPORT.x + camera_pos[0]) / CELL_PIXEL_SIZE[0];
        let grid_y = (y - WORLD_VIEWPORT.y + camera_pos[1]) / CELL_PIXEL_SIZE[1];
        let grid_x = grid_x as u32;
        let grid_y = grid_y as u32;
        if grid_x < self.core.dimensions()[0] && grid_y < self.core.dimensions()[1] {
            Some([grid_x, grid_y])
        } else {
            None
        }
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

    fn enemy_at_position(&mut self, clicked_world_pos: [u32; 2]) -> Option<EntityId> {
        self.core
            .entities()
            .iter()
            .find(|e| e.contains(clicked_world_pos) && e.health.is_some() && e.team == Team::Enemy)
            .map(|e| e.id)
    }

    fn set_selected_entity(&mut self, entity_id: Option<EntityId>) {
        self.player_state.selected_entity_id = entity_id;
        if let Some(entity) = self.selected_entity() {
            let actions = entity.actions;
            self.hud.set_entity_actions(actions);
        }
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

        self.player_state.camera.update(ctx, dt);

        if let Some(hovered_world_pos) =
            self.screen_to_grid_coordinates(ggez::input::mouse::position(ctx).into())
        {
            if self.player_state.cursor_action == CursorAction::Default {
                if self
                    .core
                    .entities()
                    .iter()
                    .any(|e| e.contains(hovered_world_pos))
                {
                    mouse::set_cursor_type(ctx, CursorIcon::Hand);
                } else {
                    mouse::set_cursor_type(ctx, CursorIcon::Default);
                }
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

        let offset = [
            WORLD_VIEWPORT.x - self.player_state.camera.position_in_world[0],
            WORLD_VIEWPORT.y - self.player_state.camera.position_in_world[1],
        ];

        if let CursorAction::PlaceStructure(structure_type) = self.player_state.cursor_action {
            if let Some(hovered_world_pos) =
                self.screen_to_grid_coordinates(ggez::input::mouse::position(ctx).into())
            {
                let size = *self.core.structure_size(&structure_type);
                let pixel_pos = grid_to_pixel_position(hovered_world_pos);
                let screen_coords = [offset[0] + pixel_pos[0], offset[1] + pixel_pos[1]];
                // TODO: Draw transparent filled rect instead of selection outline
                self.assets
                    .draw_selection(ctx, size, Team::Player, screen_coords)?;
            }
        }

        for entity in self.core.entities() {
            let pixel_pos = match &entity.physical_type {
                PhysicalType::Unit(unit) => unit.sub_cell_movement.pixel_position(entity.position),
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
        if let Some(clicked_world_pos) = self.screen_to_grid_coordinates([x, y]) {
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

                                self.core.issue_command(
                                    Command::Move(entity_id, clicked_world_pos),
                                    Team::Player,
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
                    self.core
                        .issue_command(Command::Move(entity_id, clicked_world_pos), Team::Player);
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
            }
        } else {
            self.player_state
                .set_cursor_action(ctx, CursorAction::Default);

            if let Some(player_input) = self.hud.on_mouse_button_down(button, x, y) {
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
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        self.hud.on_mouse_button_up(button);
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        if let Some(player_input) = self.hud.on_mouse_motion(x, y) {
            match player_input {
                PlayerInput::SetCameraPositionRelativeToWorldDimension([x_ratio, y_ratio]) => {
                    self.set_player_camera_position(x_ratio, y_ratio);
                }
                _ => panic!("Unhandled player input: {:?}", player_input),
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
                if let Some(player_input) = self.hud.on_key_down(keycode) {
                    match player_input {
                        PlayerInput::UseEntityAction(i) => {
                            if let Some(entity) = self.selected_player_entity() {
                                if let Some(action) = entity.actions[i] {
                                    let entity_id = entity.id;
                                    self.handle_player_entity_action(ctx, entity_id, action);
                                }
                            }
                        }
                        _ => panic!("Unhandled player input: {:?}", player_input),
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
