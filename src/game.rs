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
use crate::data::{self, EntityType, MapType, WorldInitData};
use crate::enemy_ai::EnemyPlayerAi;
use crate::entities::{
    Action, Entity, EntityId, EntityState, PhysicalType, Team, TrainingConfig,
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
    SelectAttackTarget,
    SelectMovementDestination,
    PlaceStructure(EntityType),
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
            CursorAction::SelectAttackTarget => mouse::set_cursor_type(ctx, CursorIcon::Crosshair),
            CursorAction::SelectMovementDestination => {
                mouse::set_cursor_type(ctx, CursorIcon::Move)
            }
            CursorAction::PlaceStructure(_) => mouse::set_cursor_type(ctx, CursorIcon::Grabbing),
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
    structure_sizes: HashMap<EntityType, [u32; 2]>,
}

impl Game {
    fn new(ctx: &mut Context, map_type: MapType) -> Result<Self, GameError> {
        let WorldInitData {
            dimensions: map_dimensions,
            entities,
        } = WorldInitData::new(map_type);

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

        let structure_sizes = data::structure_sizes();

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
            structure_sizes,
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
                    self.add_entity(entity_type, [x, y], team);
                    return Some([x, y]);
                }
            }
        }
        None
    }

    fn add_entity(&mut self, entity_type: EntityType, position: [u32; 2], team: Team) {
        let new_entity = data::create_entity(entity_type, position, team);
        let size = new_entity.size();
        self.entities.push(new_entity);
        self.entity_grid.set_area(&position, &size, true);
    }

    fn handle_player_entity_action(
        &mut self,
        ctx: &mut Context,
        actor_id: EntityId,
        action: Action,
    ) {
        match action {
            Action::Train(unit_type, training_config) => {
                self.issue_command(
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
                self.issue_command(Command::Heal(actor_id), Team::Player);
            }
            Action::Attack => {
                self.player_state
                    .set_cursor_action(ctx, CursorAction::SelectAttackTarget);
            }
        }
    }

    fn issue_command(&mut self, command: Command, issuing_team: Team) {
        match command {
            Command::Train(trainer_id, trained_entity_type, config) => {
                let resources = self.teams.get(&issuing_team).unwrap().resources;
                let trainer = self.entity_mut(trainer_id);
                assert_eq!(trainer.team, issuing_team);
                let training = trainer
                    .training
                    .as_mut()
                    .expect("Training command was issued for entity that can't train");
                if resources >= config.cost {
                    if let TrainingPerformStatus::NewTrainingStarted =
                        training.try_start(trained_entity_type)
                    {
                        trainer.state = EntityState::TrainingUnit(trained_entity_type);
                        self.teams.get_mut(&issuing_team).unwrap().resources -= config.cost;
                    }
                }
            }
            Command::Construct(builder_id, construction_position, construction_type) => {
                let builder = self.entity_mut(builder_id);
                assert_eq!(builder.team, issuing_team);
                let builder_pos = builder.position;
                builder.state = EntityState::Constructing(construction_type);
                builder
                    .unit_mut()
                    .pathfinder
                    .find_path(&builder_pos, construction_position);
            }
            Command::Move(mover_id, destination) => {
                let mover = self.entity_mut(mover_id);
                assert_eq!(mover.team, issuing_team);
                let current_pos = mover.position;
                mover.state = EntityState::Moving;
                mover
                    .unit_mut()
                    .pathfinder
                    .find_path(&current_pos, destination);
            }
            Command::Heal(healer_id) => {
                let healer = self.entity_mut(healer_id);
                assert_eq!(healer.team, issuing_team);
                healer
                    .actions
                    .iter()
                    .find(|action| **action == Some(Action::Heal))
                    .expect("Heal command was issued for entity that doesn't have a Heal action");
                let health = healer.health.as_mut().unwrap();
                health.receive_healing(1);
            }
            Command::Attack(attacker_id, victim_id) => {
                let victim = self.entity_mut(victim_id);
                assert_ne!(victim.team, issuing_team);
                let victim_pos = victim.position;
                let attacker = self.entity_mut(attacker_id);
                assert_eq!(attacker.team, issuing_team);
                attacker.state = EntityState::Attacking(victim_id);
                let attacker_pos = attacker.position;
                attacker
                    .unit_mut()
                    .pathfinder
                    .find_path(&attacker_pos, victim_pos);
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

        //-------------------------------
        //             AI
        //-------------------------------
        let enemy_commands = self
            .enemy_player_ai
            .run(dt, &self.entities[..], &mut self.rng);
        if !enemy_commands.is_empty() {
            println!("Issuing {} AI commands:", enemy_commands.len());
            for command in enemy_commands {
                println!("  {:?}", command);
                self.issue_command(command, Team::Enemy);
            }
        }

        //-------------------------------
        //          MOVEMENT
        //-------------------------------
        for entity in &mut self.entities {
            if let PhysicalType::Unit(unit) = &mut entity.physical_type {
                unit.sub_cell_movement.update(dt, entity.position);
                if unit.sub_cell_movement.is_ready() {
                    if let Some(next_pos) = unit.pathfinder.peek_path() {
                        let occupied = self.entity_grid.get(next_pos);
                        if !occupied {
                            let old_pos = entity.position;
                            let new_pos = unit.pathfinder.advance_path();
                            self.entity_grid.set(&old_pos, false);
                            unit.sub_cell_movement.set_moving(old_pos, new_pos);
                            entity.position = new_pos;
                            self.entity_grid.set(&new_pos, true);
                        }
                    } else if entity.state == EntityState::Moving {
                        entity.state = EntityState::Idle;
                    }
                }
            }
        }

        //-------------------------------
        //           COMBAT
        //-------------------------------
        let mut attacks = vec![];
        for entity in &mut self.entities {
            if let EntityState::Attacking(victim_id) = entity.state {
                let attacker_id = entity.id;
                let combat = entity
                    .unit_mut()
                    .combat
                    .as_mut()
                    .expect("non-combat attacker");
                if combat.count_down_cooldown(dt) {
                    attacks.push((attacker_id, 1, victim_id));
                }
            }
        }
        for (attacker_id, damage_amount, victim_id) in attacks {
            let attacker_pos = self.entity_mut(attacker_id).position;
            if let Some(victim) = self.entities.iter_mut().find(|e| e.id == victim_id) {
                let victim_pos = victim.position;
                // TODO: This doesn't work for structures that are larger than 1x1
                //       Distance calculation is only done from upper-left corner
                let within_range = square_distance(attacker_pos, victim_pos) <= 2;
                if within_range {
                    let health = victim.health.as_mut().expect("victim without health");
                    health.receive_damage(damage_amount);
                    println!(
                        "{:?} --[{} dmg]--> {:?}",
                        attacker_id, damage_amount, victim_id
                    );
                    self.entity_mut(attacker_id)
                        .unit_mut()
                        .combat
                        .as_mut()
                        .unwrap()
                        .start_cooldown();
                } else {
                    let attacker = self.entity_mut(attacker_id).unit_mut();
                    if attacker.pathfinder.peek_path().is_none() {
                        attacker.pathfinder.find_path(&attacker_pos, victim_pos);
                    }
                }
            } else {
                let attacker = self.entity_mut(attacker_id);
                attacker.state = EntityState::Idle;
                attacker.unit_mut().pathfinder.clear();
                println!(
                    "{:?} doesn't exist so {:?} went back to idling",
                    victim_id, attacker_id
                );
            }
        }

        //-------------------------------
        //       CONSTRUCTION 1
        //-------------------------------
        let mut builders_to_remove = Vec::new();
        let mut structures_to_add = Vec::new();
        for entity in &mut self.entities {
            if let EntityState::Constructing(structure_type) = entity.state {
                if entity.unit_mut().pathfinder.peek_path().is_none() {
                    let position = entity.position;
                    let size = self.structure_sizes.get(&structure_type).unwrap();
                    let mut sufficient_space = true;
                    for x in position[0]..position[0] + size[0] {
                        for y in position[1]..position[1] + size[1] {
                            if [x, y] != position {
                                // Don't check for collision on the cell that the builder stands on,
                                // since it will be removed when structure is added.
                                if self.entity_grid.get(&[x, y]) {
                                    sufficient_space = false;
                                }
                            }
                        }
                    }
                    if sufficient_space {
                        builders_to_remove.push(entity.id);
                        structures_to_add.push((entity.team, position, structure_type));
                    } else {
                        println!("There's not enough space for the structure, so builder goes back to idling");
                        entity.state = EntityState::Idle;
                    }
                }
            }
        }

        //-------------------------------
        //       ENTITY REMOVAL
        //-------------------------------
        self.entities.retain(|entity| {
            let is_dead = entity
                .health
                .as_ref()
                .map(|health| health.current == 0)
                .unwrap_or(false);
            let is_transforming_into_structure = builders_to_remove.contains(&entity.id);
            if is_transforming_into_structure {
                println!("{:?} is transforming into a structure", entity.id);
            }
            let should_be_removed = is_dead || is_transforming_into_structure;

            if should_be_removed {
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

            !should_be_removed
        });

        //-------------------------------
        //       CONSTRUCTION 2
        //-------------------------------
        // Now that the builder has been removed, and no longer occupies a cell, the structure can
        // be placed.
        for (team, position, structure_type) in structures_to_add {
            self.add_entity(structure_type, position, team);
        }

        //-------------------------------
        //          TRAINING
        //-------------------------------
        let mut completed_trainings = Vec::new();
        for entity in &mut self.entities {
            if let EntityState::TrainingUnit(trained_entity_type) = entity.state {
                let status = entity.training.as_mut().map(|training| training.update(dt));
                if let Some(TrainingUpdateStatus::Done) = status {
                    entity.state = EntityState::Idle;
                    completed_trainings.push((
                        trained_entity_type,
                        entity.team,
                        entity.position,
                        entity.size(),
                    ));
                }
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

        if let Some(hovered_world_pos) =
            self.screen_to_grid_coordinates(ggez::input::mouse::position(ctx).into())
        {
            if self.player_state.cursor_action == CursorAction::Default {
                if self.entities.iter().any(|e| e.contains(hovered_world_pos)) {
                    mouse::set_cursor_type(ctx, CursorIcon::Hand);
                } else {
                    mouse::set_cursor_type(ctx, CursorIcon::Default);
                }
            }
        }

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
                let size = *self.structure_sizes.get(&structure_type).unwrap();
                let pixel_pos = grid_to_pixel_position(hovered_world_pos);
                let screen_coords = [offset[0] + pixel_pos[0], offset[1] + pixel_pos[1]];
                // TODO: Draw transparent filled rect instead of selection outline
                self.assets
                    .draw_selection(ctx, size, Team::Player, screen_coords)?;
            }
        }

        for entity in &self.entities {
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
                            .find(|e| e.contains(clicked_world_pos))
                            .map(|e| e.id);
                    } else if let Some(entity) = self.selected_entity_mut() {
                        if entity.team == Team::Player {
                            match &mut entity.physical_type {
                                PhysicalType::Unit(unit) => {
                                    let entity_id = entity.id;

                                    if unit.combat.is_some() {
                                        if let Some(victim) = self.entities.iter().find(|e| {
                                            e.contains(clicked_world_pos)
                                                && e.health.is_some()
                                                && e.team == Team::Enemy
                                        }) {
                                            let victim_id = victim.id;
                                            // TODO: highlight attacked entity temporarily
                                            self.issue_command(
                                                Command::Attack(entity_id, victim_id),
                                                Team::Player,
                                            );
                                            return;
                                        }
                                    }

                                    self.issue_command(
                                        Command::Move(entity_id, clicked_world_pos),
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

                CursorAction::SelectMovementDestination => {
                    let entity = self
                        .selected_entity_mut()
                        .expect("Cannot issue movement without selected entity");
                    let entity_id = entity.id;
                    self.issue_command(Command::Move(entity_id, clicked_world_pos), Team::Player);
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }

                CursorAction::PlaceStructure(structure_type) => {
                    let entity = self
                        .selected_entity_mut()
                        .expect("Cannot issue construction without selected entity");
                    let entity_id = entity.id;
                    self.issue_command(
                        Command::Construct(entity_id, clicked_world_pos, structure_type),
                        Team::Player,
                    );
                    self.player_state
                        .set_cursor_action(ctx, CursorAction::Default);
                }

                CursorAction::SelectAttackTarget => {
                    if let Some(victim) = self.entities.iter_mut().find(|e| {
                        e.contains(clicked_world_pos) && e.health.is_some() && e.team == Team::Enemy
                    }) {
                        let victim_id = victim.id;
                        let attacker_id = self
                            .selected_entity_mut()
                            .expect("Cannot attack without selected entity")
                            .id;
                        // TODO: highlight attacked entity temporarily
                        self.issue_command(Command::Attack(attacker_id, victim_id), Team::Player);
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
                    if let Some(action) = self.hud.on_mouse_click([x, y], entity) {
                        let entity_id = entity.id;
                        self.handle_player_entity_action(ctx, entity_id, action);
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
                        if let Some(action) = self.hud.on_button_click(keycode, entity) {
                            let entity_id = entity.id;
                            self.handle_player_entity_action(ctx, entity_id, action);
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
    Construct(EntityId, [u32; 2], EntityType),
    Move(EntityId, [u32; 2]),
    Heal(EntityId),
    Attack(EntityId, EntityId),
}

fn square_distance(a: [u32; 2], b: [u32; 2]) -> u32 {
    ((a[0] as i32 - b[0] as i32).pow(2) + (a[1] as i32 - b[1] as i32).pow(2)) as u32
}
