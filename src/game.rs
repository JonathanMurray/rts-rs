use ggez;
use ggez::conf::{NumSamples, WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, FilterMode, Font, MeshBuilder, Rect};
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::input::mouse::{self, CursorIcon, MouseButton};
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};

use rand::rngs::ThreadRng;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashSet;

use crate::assets::Assets;
use crate::camera::Camera;
use crate::core::{
    AttackCommand, Command, CommandError, ConstructCommand, Core, GatherResourceCommand,
    MoveCommand, ReturnResourceCommand, StartActivityCommand, StopCommand, UpdateOutcome,
};
use crate::data::EntityType;
use crate::entities::{
    Action, Entity, EntityCategory, EntityId, EntityState, Team, NUM_ENTITY_ACTIONS,
};
use crate::hud_graphics::{HudGraphics, PlayerInput};
use crate::map::{MapConfig, WorldInitData};
use crate::player::{CursorState, EntityHighlight, HighlightType, PlayerState};
use crate::team_ai::TeamAi;
use crate::text::SharpFont;

pub const COLOR_FG: Color = Color::new(0.3, 0.3, 0.4, 1.0);
pub const COLOR_BG: Color = Color::new(0.2, 0.2, 0.3, 1.0);

const GAME_SIZE: [f32; 2] = [800.0, 450.0];
const WORLD_X: f32 = 225.0;
const WORLD_Y: f32 = 35.0;
pub const WORLD_VIEWPORT: Rect = Rect {
    x: WORLD_X,
    y: WORLD_Y,
    w: GAME_SIZE[0] - WORLD_X - 12.5,
    h: GAME_SIZE[1] - WORLD_Y - 35.0,
};
pub const CELL_PIXEL_SIZE: [f32; 2] = [32.0, 32.0];
const ENTITY_VISIBILITY_RECT: Rect = Rect {
    x: WORLD_VIEWPORT.x - CELL_PIXEL_SIZE[0] * 4.0,
    y: WORLD_VIEWPORT.y - CELL_PIXEL_SIZE[1] * 4.0,
    w: WORLD_VIEWPORT.w + CELL_PIXEL_SIZE[0] * 5.0,
    h: WORLD_VIEWPORT.h + CELL_PIXEL_SIZE[1] * 5.0,
};

const SHOW_GRID: bool = false;

pub const MAX_NUM_SELECTED_ENTITIES: usize = 8;

const TITLE: &str = "RTS";

pub fn run(map_config: MapConfig) -> GameResult {
    const GAME_SCALE: f32 = 3.0;
    let window_setup = WindowSetup::default().title(TITLE).samples(NumSamples::One);
    let window_mode =
        WindowMode::default().dimensions(GAME_SIZE[0] * GAME_SCALE, GAME_SIZE[1] * GAME_SCALE);
    let (mut ctx, event_loop) = ContextBuilder::new("rts", "jm")
        .window_setup(window_setup)
        .window_mode(window_mode)
        .add_resource_path("resources")
        .build()
        .expect("Creating ggez context");

    graphics::set_default_filter(&mut ctx, FilterMode::Nearest);
    graphics::set_screen_coordinates(&mut ctx, Rect::new(0.0, 0.0, GAME_SIZE[0], GAME_SIZE[1]))
        .unwrap();

    let game = Game::new(&mut ctx, map_config)?;
    ggez::event::run(ctx, event_loop, game)
}

struct Game {
    assets: Assets,
    hud: RefCell<HudGraphics>,
    player_state: PlayerState,
    enemy_team_ais: Vec<TeamAi>,
    rng: ThreadRng,
    core: Core,
}

impl Game {
    fn new(ctx: &mut Context, map_config: MapConfig) -> Result<Self, GameError> {
        let WorldInitData {
            dimensions: world_dimensions,
            entities,
            water_grid,
            tile_grid,
        } = WorldInitData::load(ctx, map_config);

        println!("Created {} entities", entities.len());

        let assets = Assets::new(ctx, [WORLD_VIEWPORT.w, WORLD_VIEWPORT.h], &tile_grid)?;

        let rng = rand::thread_rng();

        let mut teams = HashSet::new();
        for entity in &entities {
            teams.insert(entity.team);
        }
        let mut enemy_team_ais = vec![];
        if teams.contains(&Team::Enemy1) {
            let opponent = if teams.contains(&Team::Player) {
                Team::Player
            } else {
                Team::Enemy2
            };
            enemy_team_ais.push(TeamAi::new(Team::Enemy1, opponent));
        }
        if teams.contains(&Team::Enemy2) {
            let opponent = if teams.contains(&Team::Player) {
                Team::Player
            } else {
                Team::Enemy1
            };
            enemy_team_ais.push(TeamAi::new(Team::Enemy2, opponent));
        }

        let font = Font::new(ctx, "/fonts/Merchant Copy.ttf")?;
        // let font = Font::new(ctx, "/fonts/Retro Gaming.ttf")?;
        let font = SharpFont::new(font);

        let max_camera_position = [
            world_dimensions[0] as f32 * CELL_PIXEL_SIZE[0] - WORLD_VIEWPORT.w,
            world_dimensions[1] as f32 * CELL_PIXEL_SIZE[1] - WORLD_VIEWPORT.h,
        ];
        let camera = Camera::new([0.0, 0.0], max_camera_position);
        let player_state = PlayerState::new(camera);

        let hud_pos = [12.5, 12.5];
        let tooltip_pos = [WORLD_VIEWPORT.x, GAME_SIZE[1] - 25.0];
        let hud = HudGraphics::new(ctx, hud_pos, font, world_dimensions, tooltip_pos)?;
        let hud = RefCell::new(hud);

        let mut water_cells = vec![];
        for x in 0..world_dimensions[0] {
            for y in 0..world_dimensions[1] {
                if water_grid.get(&[x, y]).unwrap() {
                    water_cells.push([x, y]);
                }
            }
        }

        let core = Core::new(entities, world_dimensions, water_cells);

        Ok(Self {
            assets,
            hud,
            player_state,
            enemy_team_ais,
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

    fn resource_at_position(&self, world_pixel_coords: [f32; 2]) -> Option<&RefCell<Entity>> {
        self.core.entities().iter().find_map(|(_id, entity)| {
            if entity.borrow().entity_type == EntityType::FuelRift
                && entity.borrow().pixel_rect().contains(world_pixel_coords)
            {
                Some(entity)
            } else {
                None
            }
        })
    }

    fn enemy_at_position(&self, world_pixel_coords: [f32; 2]) -> Option<&RefCell<Entity>> {
        self.core.entities().iter().find_map(|(_id, entity)| {
            let entity_ref = entity.borrow();
            if (entity_ref.team == Team::Enemy1 || entity_ref.team == Team::Enemy2)
                && entity_ref.pixel_rect().contains(world_pixel_coords)
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
            if let EntityCategory::Structure { .. } = &entity_ref.category {
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
        self.update_hud_for_selection();
    }

    fn update_hud_for_selection(&self) {
        let mut actions = [None; NUM_ENTITY_ACTIONS];

        let mut player_entities = self.selected_player_entities();
        if let Some(first) = player_entities.next() {
            let first = first.borrow();
            // TODO standardize how this sort of thing should work.
            //      There are many situations where certain actions shouldn't be shown:
            //      building under construction, research already complete / in progress,
            //      having a cursor action tied to some selected action (?), etc.
            let is_under_construction = matches!(first.state, EntityState::UnderConstruction(..));
            if !is_under_construction {
                for (i, action_slot) in first.action_slots.iter().enumerate() {
                    actions[i] = action_slot
                        .filter(|slot| slot.enabled)
                        .map(|slot| slot.action);
                }
            }
        }

        for additional in player_entities {
            let additional = additional.borrow();

            let is_under_construction =
                matches!(additional.state, EntityState::UnderConstruction(..));
            for (i, action_slot) in additional.action_slots.iter().enumerate() {
                if is_under_construction || actions[i] != action_slot.map(|slot| slot.action) {
                    // Since not all selected entities have this action, it should not
                    // be shown in HUD.
                    actions[i] = None;
                }
            }
        }

        let mut hud = self.hud.borrow_mut();
        hud.set_entity_actions(actions);
        hud.set_num_selected_entities(self.player_state.selected_entity_ids.len());
    }

    fn handle_player_input(&mut self, ctx: &mut Context, player_input: PlayerInput) {
        match player_input {
            PlayerInput::UseEntityAction(action) => {
                for entity in self.selected_player_entities() {
                    let entity = entity.borrow_mut();
                    if entity.has_enabled_action(action) {
                        self.handle_player_use_entity_action(ctx, entity, action);
                    }
                }
            }
            PlayerInput::SetCameraPositionRelativeToWorldDimension([x_ratio, y_ratio]) => {
                self.set_camera_position(x_ratio, y_ratio);
            }
            PlayerInput::LimitSelectionToIndex(i) => {
                self.set_selected_entities(vec![self.player_state.selected_entity_ids[i]])
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
            Action::StartActivity(target, _config) => {
                self.player_issue_command(Command::StartActivity(StartActivityCommand {
                    structure: actor,
                    target,
                }));
            }
            Action::Construct(structure_type, _) => {
                let resources = self
                    .core
                    .team_state_unchecked(&Team::Player)
                    .borrow()
                    .resources;
                let construction_options = actor.unit().construction_options.as_ref().unwrap();
                let cost = construction_options.get(&structure_type).unwrap().cost;
                if resources >= cost {
                    self.set_player_cursor_state(
                        ctx,
                        CursorState::PlacingStructure(structure_type),
                    );
                } else {
                    self.hud
                        .borrow_mut()
                        .set_error_message("Not enough resources".to_owned());
                }
            }
            Action::Stop => {
                self.player_issue_command(Command::Stop(StopCommand { entity: actor }));
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

    fn player_issue_command(&self, command: Command) {
        match self.core.issue_command(command, Team::Player) {
            Ok(success) => {
                if success.did_research_state_change {
                    println!("Research state changed after issuing command. Updating HUD.");
                    self.update_hud_for_selection();
                }
            }
            Err(error) => {
                let message = match error {
                    CommandError::NotEnoughResources => "Not enough resources".to_owned(),
                    CommandError::NoPathFound => "Can't go there".to_owned(),
                    CommandError::NotCarryingResource => {
                        "Not carrying any fuel to return".to_owned()
                    }
                    CommandError::NotEnoughSpaceForStructure => {
                        "Not enough space for structure".to_owned()
                    }
                    CommandError::EntityIsBusy => "Can't do that right now".to_owned(),
                };
                self.hud.borrow_mut().set_error_message(message);
            }
        }
    }

    fn handle_right_click_world(&mut self, world_pixel_coords: [f32; 2]) {
        let world_pos = world_to_grid(world_pixel_coords);
        for entity in self.selected_player_entities() {
            let entity_ref = entity.borrow();
            match &entity_ref.category {
                EntityCategory::Unit(unit) => {
                    if unit.combat.is_some() {
                        if let Some(victim) = self.enemy_at_position(world_pixel_coords) {
                            drop(entity_ref);
                            self._player_issue_attack(entity.borrow_mut(), victim.borrow());
                            continue;
                        }
                    }
                    if entity_ref.has_enabled_action(Action::GatherResource) {
                        if let Some(resource) = self.resource_at_position(world_pixel_coords) {
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
                EntityCategory::Structure { .. } => {
                    println!("Structures have no right-click functionality yet")
                }
                EntityCategory::Resource { .. } => {}
            }
        }
    }

    fn player_issue_return_resource(
        &self,
        gatherer: RefMut<Entity>,
        structure: Option<Ref<Entity>>,
    ) {
        if let Some(structure) = structure.as_ref() {
            self.player_state
                .timed_entity_highlights
                .borrow_mut()
                .push(EntityHighlight::new(structure.id, HighlightType::Friendly));
        }
        self.player_issue_command(Command::ReturnResource(ReturnResourceCommand {
            gatherer,
            structure,
        }));
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
        self.player_issue_command(Command::Construct(ConstructCommand {
            builder,
            structure_position: clicked_world_pos,
            structure_type,
        }));
    }

    fn player_issue_all_selected_attack(&mut self, world_pixel_coords: [f32; 2]) {
        if let Some(victim) = self.enemy_at_position(world_pixel_coords) {
            for attacker in self.selected_player_entities() {
                self._player_issue_attack(attacker.borrow_mut(), victim.borrow());
            }
        } else {
            self.hud
                .borrow_mut()
                .set_error_message("Invalid attack target".to_owned());
        }
    }

    fn _player_issue_attack(&self, attacker: RefMut<Entity>, victim: Ref<Entity>) {
        self.player_state
            .timed_entity_highlights
            .borrow_mut()
            .push(EntityHighlight::new(victim.id, HighlightType::Hostile));
        self.player_issue_command(Command::Attack(AttackCommand { attacker, victim }));
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
        self.player_issue_command(Command::Move(MoveCommand {
            unit: entity,
            destination,
        }));
    }

    fn player_issue_all_selected_gather_resource(&self, world_pos: [f32; 2]) {
        if let Some(resource) = self.resource_at_position(world_pos) {
            for gatherer in self.selected_player_entities() {
                self._player_issue_gather_resource(gatherer.borrow_mut(), resource.borrow());
            }
        } else {
            self.hud
                .borrow_mut()
                .set_error_message("Invalid resource target".to_owned());
        }
    }

    fn _player_issue_gather_resource(&self, gatherer: RefMut<Entity>, resource: Ref<Entity>) {
        self.player_state
            .timed_entity_highlights
            .borrow_mut()
            .push(EntityHighlight::new(resource.id, HighlightType::Friendly));
        self.player_issue_command(Command::GatherResource(GatherResourceCommand {
            gatherer,
            resource,
        }));
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
        // let [x, y]: [f32; 2] = mouse_position(ctx);
        // graphics::set_window_title(ctx, &format!("{} ({}, {})", TITLE, x, y));
        let fps = ggez::timer::fps(ctx) as u32;
        graphics::set_window_title(ctx, &format!("{} (fps={})", TITLE, fps));

        let dt = ggez::timer::delta(ctx);

        for ai in &mut self.enemy_team_ais {
            if let Some(command) = ai.run(dt, &self.core, &mut self.rng) {
                println!("[{:?}] Issuing AI command", ai.team());
                let _ = self.core.issue_command(command, ai.team());
            }
        }

        let UpdateOutcome {
            removed_entities,
            finished_structures,
            did_research_state_change,
        } = self.core.update(dt);

        let num_selected_before = self.player_state.selected_entity_ids.len();
        self.player_state
            .selected_entity_ids
            .retain(|entity_id| !removed_entities.contains(entity_id));
        let mut should_update_hud = did_research_state_change;
        if num_selected_before != self.player_state.selected_entity_ids.len() {
            // TODO: what if you still have some selected entity, but it doesn't
            //       have any action corresponding to the cursor state?
            self.set_player_cursor_state(ctx, CursorState::Default);
            should_update_hud = true;
        }
        for selected_entity_id in &self.player_state.selected_entity_ids {
            if finished_structures.contains(selected_entity_id) {
                should_update_hud = true;
                break;
            }
        }
        if should_update_hud {
            self.update_hud_for_selection();
        }

        self.player_state.update(ctx, dt);

        let mouse_pos = mouse_position(ctx);
        match self.player_state.cursor_state() {
            CursorState::Default => {
                let hovered_entity =
                    if let Some(pixel_coords) = self.player_state.screen_to_world(mouse_pos) {
                        self.core
                            .entities()
                            .iter()
                            .find(|(_id, e)| e.borrow().pixel_rect().contains(pixel_coords))
                    } else {
                        None
                    };

                if let Some((entity_id, _entity)) = hovered_entity {
                    mouse::set_cursor_type(ctx, CursorIcon::Hand);
                    self.player_state.hovered_entity_highlight =
                        Some((*entity_id, HighlightType::Neutral));
                } else {
                    mouse::set_cursor_type(ctx, CursorIcon::Default);
                    self.player_state.hovered_entity_highlight = None;
                }
            }

            CursorState::SelectingAttackTarget => {
                if let Some(world_pixel_coords) = self.player_state.screen_to_world(mouse_pos) {
                    self.player_state.hovered_entity_highlight = self
                        .enemy_at_position(world_pixel_coords)
                        .map(|enemy| (enemy.borrow().id, HighlightType::Hostile));
                }
            }

            CursorState::SelectingResourceTarget => {
                if let Some(world_pixel_coords) = self.player_state.screen_to_world(mouse_pos) {
                    self.player_state.hovered_entity_highlight = self
                        .resource_at_position(world_pixel_coords)
                        .map(|enemy| (enemy.borrow().id, HighlightType::Friendly));
                }
            }
            _ => {}
        }

        self.hud.borrow_mut().update(dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, COLOR_FG);

        let camera_pos_in_world = self.player_state.camera.borrow().position_in_world;
        self.assets.draw_world_background(
            ctx,
            WORLD_VIEWPORT.point().into(),
            camera_pos_in_world,
        )?;

        if SHOW_GRID {
            self.assets
                .draw_grid(ctx, WORLD_VIEWPORT.point().into(), camera_pos_in_world)?;
        }

        let indicator = &self.player_state.movement_command_indicator;
        if let Some((world_pixel_position, scale)) = indicator.borrow().graphics() {
            let screen_coords = self.player_state.world_to_screen(world_pixel_position);
            self.assets
                .draw_movement_command_indicator(ctx, screen_coords, scale)?;
        }

        let mut entities_to_draw = vec![];
        for (entity_id, entity) in self.core.entities() {
            let entity = entity.borrow();
            let screen_coords = self
                .player_state
                .world_to_screen(entity.world_pixel_position());

            if self.player_state.selected_entity_ids.contains(entity_id) {
                if let EntityState::MovingToConstruction(structure_type, grid_pos) = entity.state {
                    let screen_coords = self.player_state.world_to_screen(grid_to_world(grid_pos));
                    let size = *self.core.structure_size(&structure_type);
                    self.assets
                        .draw_construction_outline(ctx, size, screen_coords)?;
                }
            }

            if ENTITY_VISIBILITY_RECT.contains(screen_coords) {
                entities_to_draw.push((screen_coords, entity));
            }
        }

        for (screen_coords, entity) in &entities_to_draw {
            if matches!(entity.category, EntityCategory::Structure { .. }) {
                self.assets.draw_entity(ctx, entity, *screen_coords)?;
            }
        }
        for (screen_coords, entity) in &entities_to_draw {
            if matches!(entity.category, EntityCategory::Resource { .. }) {
                self.assets.draw_entity(ctx, entity, *screen_coords)?;
            }
        }
        for (screen_coords, entity) in &entities_to_draw {
            if matches!(entity.category, EntityCategory::Unit { .. }) {
                self.assets.draw_entity(ctx, entity, *screen_coords)?;
            }
        }
        for (screen_coords, entity) in &entities_to_draw {
            if self.player_state.selected_entity_ids.contains(&entity.id) {
                self.assets
                    .draw_selection(ctx, entity.size(), entity.team, *screen_coords)?;
            }
            if let Some((hovered_id, highlight_type)) = self.player_state.hovered_entity_highlight {
                if hovered_id == entity.id {
                    Assets::draw_highlight(ctx, entity.size(), *screen_coords, highlight_type)?;
                }
            }
            if let Some(highlight) = self
                .player_state
                .timed_entity_highlights
                .borrow()
                .iter()
                .find(|highlight| highlight.entity_id == entity.id && highlight.is_visible())
            {
                Assets::draw_highlight(
                    ctx,
                    entity.size(),
                    *screen_coords,
                    highlight.highlight_type,
                )?;
            }
        }

        let mouse_position: [f32; 2] = mouse_position(ctx);
        match self.player_state.cursor_state() {
            CursorState::PlacingStructure(structure_type) => {
                if let Some(hovered_world_pos) = self.screen_to_grid(mouse_position) {
                    let size = *self.core.structure_size(&structure_type);
                    let screen_coords = self
                        .player_state
                        .world_to_screen(grid_to_world(hovered_world_pos));
                    self.assets
                        .draw_construction_outline(ctx, size, screen_coords)?;
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

        let player_resources = self
            .core
            .team_state(&Team::Player)
            .map(|team_state| team_state.borrow().resources);
        self.hud.borrow_mut().draw(
            ctx,
            player_resources,
            selected_entities,
            &self.player_state,
            self.core.obstacle_grid(),
        )?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        let [x, y] = physical_to_logical(ctx, [x, y]);
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
                    self.player_issue_all_selected_attack(clicked_world_pixel_coords);
                    self.set_player_cursor_state(ctx, CursorState::Default);
                }
                CursorState::SelectingResourceTarget => {
                    self.player_issue_all_selected_gather_resource(clicked_world_pixel_coords);
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
        let [x, y] = physical_to_logical(ctx, [x, y]);
        if let CursorState::DraggingSelectionArea(start_world_pixel_coords) =
            self.player_state.cursor_state()
        {
            self.set_player_cursor_state(ctx, CursorState::Default);

            let released_world_pixel_coords = self.player_state.screen_to_world_clamped([x, y]);
            let selection_rect =
                Game::rect_from_points(start_world_pixel_coords, released_world_pixel_coords);

            println!("SELECTION RECT: {:?}", selection_rect);

            // TODO: prioritize units
            if button == MouseButton::Left {
                // Only player-owned entities can be selected in groups.
                // Player-owned entities are prioritized when drag-selecting.

                let mut player_entities = vec![];
                let mut non_player_entity = None;

                for (id, entity) in self.core.entities() {
                    let entity = entity.borrow();
                    if entity.team == Team::Player {
                        if entity.pixel_rect().overlaps(&selection_rect) {
                            player_entities.push(*id);
                            if player_entities.len() == MAX_NUM_SELECTED_ENTITIES {
                                break;
                            }
                        }
                    } else if non_player_entity.is_none()
                        && entity.pixel_rect().overlaps(&selection_rect)
                    {
                        non_player_entity = Some(*id);
                    }
                }

                let new_selection = if !player_entities.is_empty() {
                    player_entities
                } else if let Some(other) = non_player_entity {
                    vec![other]
                } else {
                    vec![]
                };

                println!("Selected {:?} by releasing mouse button", new_selection);
                self.set_selected_entities(new_selection);
            }
        }

        self.hud.borrow_mut().on_mouse_button_up(button);
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        let [x, y] = physical_to_logical(ctx, [x, y]);
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
            KeyCode::Key0 => {
                if let Some(selected) = self.selected_entities().next() {
                    // Dump selected entity for debugging
                    println!("\n--------------------------------");
                    println!("{:?}", selected.borrow());
                    println!("--------------------------------\n");
                }
            }
            _ => {
                let mut hud = self.hud.borrow_mut();
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

fn mouse_position(ctx: &mut Context) -> [f32; 2] {
    physical_to_logical(ctx, ggez::input::mouse::position(ctx).into())
}

fn physical_to_logical(ctx: &mut Context, coordinates: [f32; 2]) -> [f32; 2] {
    let screen_rect = graphics::screen_coordinates(ctx);
    let size = graphics::window(ctx).inner_size();
    [
        screen_rect.x + coordinates[0] / size.width as f32 * screen_rect.w,
        screen_rect.y + coordinates[1] / size.height as f32 * screen_rect.h,
    ]
}
