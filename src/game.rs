use ggez;
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Font, MeshBuilder, Rect};
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};

use rand::rngs::ThreadRng;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::time::Duration;

use crate::assets::Assets;
use crate::camera::Camera;
use crate::core::{
    AttackCommand, Command, ConstructCommand, Core, GatherResourceCommand, MoveCommand,
    ReturnResourceCommand, TrainCommand,
};
use crate::data::{EntityType, MapType, WorldInitData};
use crate::enemy_ai::EnemyPlayerAi;
use crate::entities::{Action, Entity, EntityId, PhysicalType, Team, NUM_ENTITY_ACTIONS};
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

const SHOW_GRID: bool = false;

//TODO
const MAX_NUM_SELECTED_ENTITIES: usize = 2;

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
pub enum CursorState {
    Default,
    SelectingAttackTarget,
    SelectingMovementDestination,
    PlacingStructure(EntityType),
    SelectingResourceTarget,
    DraggingSelectionArea([f32; 2]),
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
    selected_entity_ids: Vec<EntityId>,
    cursor_state: Cell<CursorState>,
    camera: RefCell<Camera>,
    movement_command_indicator: RefCell<MovementCommandIndicator>,
}

impl PlayerState {
    fn new(camera: Camera) -> Self {
        Self {
            selected_entity_ids: vec![],
            cursor_state: Cell::new(CursorState::Default),
            camera: RefCell::new(camera),
            movement_command_indicator: RefCell::new(MovementCommandIndicator::new()),
        }
    }

    fn set_cursor_state(&self, ctx: &mut Context, state: CursorState) {
        match state {
            CursorState::Default => mouse::set_cursor_type(ctx, CursorIcon::Default),
            CursorState::SelectingAttackTarget => {
                mouse::set_cursor_type(ctx, CursorIcon::Crosshair)
            }
            CursorState::SelectingMovementDestination => {
                mouse::set_cursor_type(ctx, CursorIcon::Move)
            }
            CursorState::PlacingStructure(..) => mouse::set_cursor_type(ctx, CursorIcon::Grabbing),
            CursorState::SelectingResourceTarget => mouse::set_cursor_type(ctx, CursorIcon::Grab),
            CursorState::DraggingSelectionArea(..) => {
                mouse::set_cursor_type(ctx, CursorIcon::Default)
            }
        }
        self.cursor_state.set(state);
    }

    pub fn cursor_state(&self) -> CursorState {
        self.cursor_state.get()
    }

    fn screen_to_world(&self, coordinates: [f32; 2]) -> Option<[f32; 2]> {
        let [x, y] = coordinates;
        if !WORLD_VIEWPORT.contains(coordinates) {
            return None;
        }

        let camera_pos = self.camera.borrow().position_in_world;
        Some([
            x - WORLD_VIEWPORT.x + camera_pos[0],
            y - WORLD_VIEWPORT.y + camera_pos[1],
        ])
    }

    fn world_to_screen(&self, world_pixel_position: [f32; 2]) -> [f32; 2] {
        let [x, y] = world_pixel_position;
        let camera_pos = self.camera.borrow().position_in_world;
        [
            WORLD_VIEWPORT.x + x - camera_pos[0],
            WORLD_VIEWPORT.y + y - camera_pos[1],
        ]
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.camera.borrow_mut().update(ctx, dt);
        self.movement_command_indicator.borrow_mut().update(dt);
    }

    pub fn camera_position_in_world(&self) -> [f32; 2] {
        self.camera.borrow().position_in_world
    }
}

struct Game {
    assets: Assets,
    hud: RefCell<HudGraphics>,
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

        let hud_pos = [WORLD_VIEWPORT.x, WORLD_VIEWPORT.y + WORLD_VIEWPORT.h + 15.0];
        let hud = HudGraphics::new(ctx, hud_pos, font, world_dimensions)?;
        let hud = RefCell::new(hud);

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

    fn selected_entities(&self) -> impl Iterator<Item = &RefCell<Entity>> {
        self.player_state.selected_entity_ids.iter().map(|id| {
            self.core
                .entities()
                .iter()
                .find_map(|(entity_id, entity)| if entity_id == id { Some(entity) } else { None })
                .expect("selected entity must exist")
        })
    }

    fn selected_player_entities(&self) -> impl Iterator<Item = &RefCell<Entity>> {
        self.selected_entities()
            .filter(|entity| RefCell::borrow(entity).team == Team::Player)
    }

    fn resource_at_position(&self, clicked_world_pos: [u32; 2]) -> Option<&RefCell<Entity>> {
        // TODO we assume that all neutral entities are resources for now
        self.core.entities().iter().find_map(|(_id, entity)| {
            if entity.borrow().cell_rect().contains(clicked_world_pos)
                && entity.borrow().team == Team::Neutral
            {
                Some(entity)
            } else {
                None
            }
        })
    }

    fn enemy_at_position(&self, clicked_world_pos: [u32; 2]) -> Option<&RefCell<Entity>> {
        self.core.entities().iter().find_map(|(_id, entity)| {
            let entity_ref = entity.borrow();
            if entity_ref.cell_rect().contains(clicked_world_pos)
                && entity_ref.health.is_some()
                && entity_ref.team == Team::Enemy
            {
                drop(entity_ref);
                Some(entity)
            } else {
                None
            }
        })
    }

    fn player_structure_at_position(
        &self,
        clicked_world_pos: [u32; 2],
    ) -> Option<&RefCell<Entity>> {
        self.core.entities().iter().find_map(|(_id, entity)| {
            let entity_ref = entity.borrow();
            if let PhysicalType::Structure { .. } = &entity_ref.physical_type {
                if entity_ref.cell_rect().contains(clicked_world_pos)
                    && entity_ref.team == Team::Player
                {
                    drop(entity_ref);
                    return Some(entity);
                }
            }
            None
        })
    }

    fn set_camera_position(&self, x_ratio: f32, y_ratio: f32) {
        self.player_state.camera.borrow_mut().position_in_world = [
            x_ratio * self.core.dimensions()[0] as f32 * CELL_PIXEL_SIZE[0]
                - WORLD_VIEWPORT.w / 2.0,
            y_ratio * self.core.dimensions()[1] as f32 * CELL_PIXEL_SIZE[1]
                - WORLD_VIEWPORT.h / 2.0,
        ];
    }

    fn set_selected_entities(&mut self, entity_ids: Vec<EntityId>) {
        self.player_state.selected_entity_ids = entity_ids;
        let mut actions = [None; NUM_ENTITY_ACTIONS];

        let mut player_entities = self.selected_player_entities();
        if let Some(first) = player_entities.next() {
            actions = first.borrow().actions;
        }

        for additional in player_entities {
            for (i, action) in additional.borrow().actions.iter().enumerate() {
                if actions[i] != *action {
                    // Since not all selected entities have this action, it should not
                    // be shown in HUD.
                    actions[i] = None;
                }
            }
        }

        self.hud.borrow_mut().set_entity_actions(actions);
    }

    fn handle_player_input(&self, ctx: &mut Context, player_input: PlayerInput) {
        match player_input {
            PlayerInput::UseEntityAction(action) => {
                for entity in self.selected_player_entities() {
                    let mut_entity = entity.borrow_mut();
                    if mut_entity.actions.contains(&Some(action)) {
                        self.handle_player_use_entity_action(ctx, mut_entity, action);
                    }
                }
            }
            PlayerInput::SetCameraPositionRelativeToWorldDimension([x_ratio, y_ratio]) => {
                self.set_camera_position(x_ratio, y_ratio);
            }
        }
    }

    fn handle_player_use_entity_action(
        &self,
        ctx: &mut Context,
        actor: RefMut<Entity>,
        action: Action,
    ) {
        match action {
            Action::Train(trained_unit_type, config) => {
                self.core.issue_command(
                    Command::Train(TrainCommand {
                        trainer: actor,
                        trained_unit_type,
                        config,
                    }),
                    Team::Player,
                );
            }
            Action::Construct(structure_type) => {
                self.set_player_cursor_state(ctx, CursorState::PlacingStructure(structure_type));
            }
            Action::Move => {
                self.set_player_cursor_state(ctx, CursorState::SelectingMovementDestination);
            }
            Action::Attack => {
                self.set_player_cursor_state(ctx, CursorState::SelectingAttackTarget);
            }
            Action::GatherResource => {
                self.set_player_cursor_state(ctx, CursorState::SelectingResourceTarget);
            }
            Action::ReturnResource => {
                self.player_issue_return_resource(actor, None);
            }
        }
    }

    fn handle_right_click_world(&mut self, world_pixel_coords: [f32; 2]) {
        let world_pos = world_to_grid(world_pixel_coords);
        for entity in self.selected_player_entities() {
            let entity_ref = entity.borrow();
            match &entity_ref.physical_type {
                PhysicalType::Unit(unit) => {
                    if unit.combat.is_some() {
                        if let Some(victim) = self.enemy_at_position(world_pos) {
                            drop(entity_ref);
                            self._player_issue_attack(entity.borrow_mut(), victim.borrow());
                            continue;
                        }
                    }
                    if entity_ref.actions.contains(&Some(Action::GatherResource)) {
                        if let Some(resource) = self.resource_at_position(world_pos) {
                            drop(entity_ref);
                            self._player_issue_gather_resource(
                                entity.borrow_mut(),
                                resource.borrow(),
                            );
                            continue;
                        }
                        if let Some(structure) = self.player_structure_at_position(world_pos) {
                            drop(entity_ref);
                            self.player_issue_return_resource(
                                entity.borrow_mut(),
                                Some(structure.borrow()),
                            );
                            continue;
                        }
                    }
                    drop(entity_ref);
                    self._player_issue_movement(entity.borrow_mut(), world_pixel_coords);
                }
                PhysicalType::Structure { .. } => {
                    println!("Structures have no right-click functionality yet")
                }
            }
        }
    }

    fn player_issue_return_resource(
        &self,
        gatherer: RefMut<Entity>,
        structure: Option<Ref<Entity>>,
    ) {
        self.core.issue_command(
            Command::ReturnResource(ReturnResourceCommand {
                gatherer,
                structure,
            }),
            Team::Player,
        );
    }

    fn player_issue_first_selected_construct(
        &self,
        _ctx: &mut Context,
        clicked_world_pos: [u32; 2],
        structure_type: EntityType,
    ) {
        let builder = self
            .selected_player_entities()
            .next()
            .expect("Cannot issue construction without selected entity")
            .borrow_mut();
        self.core.issue_command(
            Command::Construct(ConstructCommand {
                builder,
                structure_position: clicked_world_pos,
                structure_type,
            }),
            Team::Player,
        );
    }

    fn player_issue_all_selected_attack(&self, world_pos: [u32; 2]) {
        if let Some(victim) = self.enemy_at_position(world_pos) {
            for attacker in self.selected_player_entities() {
                self._player_issue_attack(attacker.borrow_mut(), victim.borrow());
            }
        } else {
            println!("Invalid attack target");
        }
    }

    fn _player_issue_attack(&self, attacker: RefMut<Entity>, victim: Ref<Entity>) {
        // TODO: highlight attacked entity temporarily
        self.core.issue_command(
            Command::Attack(AttackCommand { attacker, victim }),
            Team::Player,
        );
    }

    fn player_issue_all_selected_movement(&self, world_pixel_coords: [f32; 2]) {
        for entity in self.selected_player_entities() {
            self._player_issue_movement(entity.borrow_mut(), world_pixel_coords);
        }
    }

    fn _player_issue_movement(&self, entity: RefMut<Entity>, world_pixel_coordinates: [f32; 2]) {
        self.player_state
            .movement_command_indicator
            .borrow_mut()
            .set(world_pixel_coordinates);
        let destination = world_to_grid(world_pixel_coordinates);
        self.core.issue_command(
            Command::Move(MoveCommand {
                unit: entity,
                destination,
            }),
            Team::Player,
        );
    }

    fn player_issue_all_selected_gather_resource(&self, world_pos: [u32; 2]) {
        if let Some(resource) = self.resource_at_position(world_pos) {
            for gatherer in self.selected_player_entities() {
                self._player_issue_gather_resource(gatherer.borrow_mut(), resource.borrow());
            }
        } else {
            println!("Invalid resource target");
        }
    }

    fn _player_issue_gather_resource(&self, gatherer: RefMut<Entity>, resource: Ref<Entity>) {
        self.core.issue_command(
            Command::GatherResource(GatherResourceCommand { gatherer, resource }),
            Team::Player,
        );
    }

    fn set_player_cursor_state(&self, ctx: &mut Context, cursor_state: CursorState) {
        self.player_state.set_cursor_state(ctx, cursor_state);
    }

    fn screen_to_grid(&self, coordinates: [f32; 2]) -> Option<[u32; 2]> {
        self.player_state
            .screen_to_world(coordinates)
            .map(world_to_grid)
    }

    // Create a rect with non-negative width and height from two points
    fn rect_from_points(a: [f32; 2], b: [f32; 2]) -> Rect {
        let (x0, x1) = if a[0] < b[0] {
            (a[0], b[0])
        } else {
            (b[0], a[0])
        };
        let (y0, y1) = if a[1] < b[1] {
            (a[1], b[1])
        } else {
            (b[1], a[1])
        };

        Rect::new(x0, y0, x1 - x0, y1 - y0)
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
        }
        for command in enemy_commands {
            //println!("  {:?}", command);
            self.core.issue_command(command, Team::Enemy);
        }

        let removed_entity_ids = self.core.update(dt);

        let had_some_selected = !self.player_state.selected_entity_ids.is_empty();
        self.player_state
            .selected_entity_ids
            .retain(|entity_id| !removed_entity_ids.contains(entity_id));
        if had_some_selected && self.player_state.selected_entity_ids.is_empty() {
            // TODO: what if you still have some selected entity, but it doesn't
            //       have any action corresponding to the cursor state?
            self.set_player_cursor_state(ctx, CursorState::Default);
            self.hud
                .borrow_mut()
                .set_entity_actions([None; NUM_ENTITY_ACTIONS]);
        }

        self.player_state.update(ctx, dt);

        if let Some(pixel_coords) = self
            .player_state
            .screen_to_world(ggez::input::mouse::position(ctx).into())
        {
            if self.player_state.cursor_state() == CursorState::Default {
                let is_hovering_some_entity = self
                    .core
                    .entities()
                    .iter()
                    .any(|(_id, e)| e.borrow().pixel_rect().contains(pixel_coords));
                let icon = if is_hovering_some_entity {
                    CursorIcon::Hand
                } else {
                    CursorIcon::Default
                };
                mouse::set_cursor_type(ctx, icon);
            }
        }

        self.hud.borrow_mut().update(dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, COLOR_BG);

        if SHOW_GRID {
            self.assets.draw_grid(
                ctx,
                WORLD_VIEWPORT.point().into(),
                self.player_state.camera.borrow().position_in_world,
            )?;
        }

        let indicator = &self.player_state.movement_command_indicator;
        if let Some((world_pixel_position, scale)) = indicator.borrow().graphics() {
            let screen_coords = self.player_state.world_to_screen(world_pixel_position);
            self.assets
                .draw_movement_command_indicator(ctx, screen_coords, scale)?;
        }

        for (entity_id, entity) in self.core.entities() {
            let entity = entity.borrow();
            let screen_coords = self
                .player_state
                .world_to_screen(entity.world_pixel_position());

            if self.player_state.selected_entity_ids.contains(entity_id) {
                self.assets
                    .draw_selection(ctx, entity.size(), entity.team, screen_coords)?;
            }

            self.assets
                .draw_entity(ctx, entity.sprite, entity.team, screen_coords)?;
        }
        self.assets.flush_entity_sprite_batch(ctx)?;

        let mouse_position: [f32; 2] = ggez::input::mouse::position(ctx).into();
        match self.player_state.cursor_state() {
            CursorState::PlacingStructure(structure_type) => {
                if let Some(hovered_world_pos) = self.screen_to_grid(mouse_position) {
                    let size = *self.core.structure_size(&structure_type);
                    let world_coords = grid_to_world(hovered_world_pos);
                    let screen_coords = self.player_state.world_to_screen(world_coords);
                    // TODO: Draw transparent filled rect instead of selection outline
                    self.assets
                        .draw_selection(ctx, size, Team::Player, screen_coords)?;
                }
            }
            CursorState::DraggingSelectionArea(start_world_pixel_coords) => {
                let rect = Game::rect_from_points(
                    self.player_state.world_to_screen(start_world_pixel_coords),
                    mouse_position,
                );
                MeshBuilder::new()
                    .rectangle(DrawMode::stroke(2.0), rect, Color::new(0.6, 1.0, 0.6, 1.0))?
                    .build(ctx)?
                    .draw(ctx, DrawParam::default())?;
            }
            _ => {}
        }

        self.assets
            .draw_background_around_grid(ctx, WORLD_VIEWPORT.point().into())?;

        let selected_entities: Vec<Ref<Entity>> = self
            .selected_entities()
            .map(|entity| entity.borrow())
            .collect();

        self.hud.borrow().draw(
            ctx,
            self.core.team_state(&Team::Player).borrow(),
            selected_entities,
            self.player_state.selected_entity_ids.len(), //TODO
            &self.player_state,
        )?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let Some(clicked_world_pixel_coords) = self.player_state.screen_to_world([x, y]) {
            let clicked_world_pos = world_to_grid(clicked_world_pixel_coords);
            match self.player_state.cursor_state() {
                CursorState::Default => {
                    if button == MouseButton::Left {
                        println!("Starting to define selection area...");
                        self.set_player_cursor_state(
                            ctx,
                            CursorState::DraggingSelectionArea(clicked_world_pixel_coords),
                        );
                    } else if button == MouseButton::Right {
                        self.handle_right_click_world(clicked_world_pixel_coords)
                    }
                }
                CursorState::SelectingMovementDestination => {
                    self.player_issue_all_selected_movement(clicked_world_pixel_coords);
                    self.set_player_cursor_state(ctx, CursorState::Default);
                }
                CursorState::PlacingStructure(structure_type) => {
                    self.player_issue_first_selected_construct(
                        ctx,
                        clicked_world_pos,
                        structure_type,
                    );
                    self.set_player_cursor_state(ctx, CursorState::Default);
                }
                CursorState::SelectingAttackTarget => {
                    self.player_issue_all_selected_attack(clicked_world_pos);
                    self.set_player_cursor_state(ctx, CursorState::Default);
                }
                CursorState::SelectingResourceTarget => {
                    self.player_issue_all_selected_gather_resource(clicked_world_pos);
                    self.set_player_cursor_state(ctx, CursorState::Default);
                }
                CursorState::DraggingSelectionArea(..) => {
                    panic!("How did we end up here? When we release button, this cursor action should have been removed.");
                }
            }
        } else {
            self.set_player_cursor_state(ctx, CursorState::Default);

            let mut hud = self.hud.borrow_mut();
            if let Some(player_input) = hud.on_mouse_button_down(button, x, y) {
                drop(hud); // HUD may need to be updated, as part of handling the input
                self.handle_player_input(ctx, player_input)
            }
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if let CursorState::DraggingSelectionArea(start_world_pixel_coords) =
            self.player_state.cursor_state()
        {
            self.set_player_cursor_state(ctx, CursorState::Default);
            // TODO: select even if mouse is released outside of the world view port
            if let Some(released_world_pixel_coords) = self.player_state.screen_to_world([x, y]) {
                let selection_rect =
                    Game::rect_from_points(start_world_pixel_coords, released_world_pixel_coords);

                println!("SELECTION RECT: {:?}", selection_rect);

                // TODO: prioritize player-owned
                // TODO: prioritize units
                if button == MouseButton::Left {
                    let selected_entity_ids = self
                        .core
                        .entities()
                        .iter()
                        .filter_map(|(id, e)| {
                            let e = e.borrow();
                            if e.pixel_rect().overlaps(&selection_rect) {
                                Some(*id)
                            } else {
                                None
                            }
                        })
                        .take(MAX_NUM_SELECTED_ENTITIES)
                        .collect();
                    println!(
                        "Selected {:?} by releasing mouse button",
                        selected_entity_ids
                    );
                    self.set_selected_entities(selected_entity_ids);
                }
            } else {
                println!("Didn't get any targets from the selection area")
            }
        }

        self.hud.borrow_mut().on_mouse_button_up(button);
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        let mut hud = self.hud.borrow_mut();
        if let Some(player_input) = hud.on_mouse_motion(x, y) {
            drop(hud); // HUD may need to be updated, as part of handling the input
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
                let hud = self.hud.borrow();
                if let Some(player_input) = hud.on_key_down(keycode) {
                    drop(hud); // HUD may need to be updated, as part of handling the input
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
