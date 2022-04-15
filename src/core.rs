use std::cell::{Ref, RefCell, RefMut};
use std::cmp::min;
use std::collections::HashMap;
use std::time::Duration;

use crate::data::{self, EntityType};
use crate::entities::{
    Entity, EntityId, EntityState, PhysicalType, Team, TrainingConfig, TrainingPerformStatus,
    TrainingUpdateStatus,
};
use crate::grid::{CellRect, Grid};
use crate::pathfind::{self, Destination};
use std::borrow::BorrowMut;

pub struct Core {
    teams: HashMap<Team, RefCell<TeamState>>,
    entities: Vec<(EntityId, RefCell<Entity>)>,
    obstacle_grid: Grid<ObstacleType>,
    structure_sizes: HashMap<EntityType, [u32; 2]>,
}

impl Core {
    pub fn new(
        entities: Vec<Entity>,
        world_dimensions: [u32; 2],
        water_cells: Vec<[u32; 2]>,
    ) -> Self {
        let mut teams = HashMap::new();
        teams.insert(Team::Player, RefCell::new(TeamState { resources: 5 }));
        teams.insert(Team::Enemy, RefCell::new(TeamState { resources: 5 }));

        let mut obstacle_grid = Grid::new(world_dimensions);
        for water_cell in water_cells {
            obstacle_grid.set(water_cell, Some(ObstacleType::Water));
        }
        for entity in &entities {
            if entity.is_solid {
                // TODO Store EntityId's instead, to get constant position->entity_id lookup?
                //      (although entity_id->entity is still not constant currently)
                obstacle_grid.set_area(entity.cell_rect(), Some(ObstacleType::Entity(entity.team)));
            }
        }
        let entities = entities
            .into_iter()
            .map(|entity| (entity.id, RefCell::new(entity)))
            .collect();
        let structure_sizes = data::structure_sizes();
        Self {
            teams,
            entities,
            obstacle_grid,
            structure_sizes,
        }
    }

    pub fn update(&mut self, dt: Duration) -> UpdateOutcome {
        //-------------------------------
        //          MOVEMENT
        //-------------------------------
        for (_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            let pos = entity.position;
            if let PhysicalType::Unit(unit) = &mut entity.physical_type {
                unit.sub_cell_movement.update(dt, pos);
                let mut is_moving = false;
                if unit.sub_cell_movement.is_ready() {
                    if let Some(next_pos) = unit.movement_plan.peek() {
                        let obstacle = self.obstacle_grid.get(next_pos);
                        if obstacle.is_none() {
                            is_moving = true;
                            let old_pos = pos;
                            let new_pos = unit.movement_plan.advance();
                            unit.move_to_adjacent_cell(old_pos, new_pos);
                            entity.position = new_pos;
                            self.obstacle_grid.set(old_pos, None);
                            self.obstacle_grid
                                .set(new_pos, Some(ObstacleType::Entity(entity.team)));
                        }
                    } else if entity.state == EntityState::Moving {
                        entity.state = EntityState::Idle;
                    }
                } else {
                    is_moving = true;
                }

                if is_moving {
                    entity.animation.ms_counter = entity
                        .animation
                        .ms_counter
                        .wrapping_add(dt.as_millis() as u16);
                }
            }
        }

        //-------------------------------
        //           COMBAT
        //-------------------------------
        for (_entity_id, entity) in &self.entities {
            let entity = entity.borrow_mut();
            if let EntityState::Attacking(victim_id) = entity.state {
                let mut attacker = entity;
                let combat = attacker
                    .unit_mut()
                    .combat
                    .as_mut()
                    .expect("non-combat attacker");
                if combat.count_down_cooldown(dt) {
                    if let Some(victim) = self.find_entity(victim_id) {
                        let mut victim = victim.borrow_mut();
                        if is_unit_within_melee_range_of(attacker.position, victim.cell_rect()) {
                            let health = victim.health.as_mut().expect("victim without health");
                            let damage_amount = 1;
                            health.receive_damage(damage_amount);
                            println!(
                                "{:?} --[{} dmg]--> {:?}",
                                attacker.id, damage_amount, victim_id
                            );
                            attacker
                                .unit_mut()
                                .combat
                                .as_mut()
                                .unwrap()
                                .start_cooldown();
                        } else if attacker.unit_mut().movement_plan.peek().is_none() {
                            if let Some(plan) = pathfind::find_path(
                                attacker.position,
                                Destination::AdjacentToEntity(victim.cell_rect()),
                                &self.obstacle_grid,
                            ) {
                                attacker.unit_mut().movement_plan.set(plan);
                            }
                        }
                    } else {
                        attacker.state = EntityState::Idle;
                        attacker.unit_mut().movement_plan.clear();
                    }
                }
            }
        }

        //-------------------------------
        //     GATHERING RESOURCES
        //-------------------------------
        for (_entity_id, entity) in &self.entities {
            let entity = entity.borrow_mut();
            if let EntityState::GatheringResource(resource_id) = entity.state {
                let mut gatherer = entity;
                if gatherer.unit_mut().sub_cell_movement.is_ready() {
                    let resource = self
                        .find_entity(resource_id)
                        .unwrap_or_else(|| panic!("Resource not found: {:?}", resource_id));
                    let resource = resource.borrow();
                    if is_unit_within_melee_range_of(gatherer.position, resource.cell_rect()) {
                        let gathering = gatherer.unit_mut().gathering.as_mut().unwrap();
                        gathering.pick_up_resource(resource_id);
                        self.unit_return_resource(gatherer, None);
                    }
                }
            }
        }

        //-------------------------------
        //     RETURNING RESOURCES
        //-------------------------------
        for (_entity_id, entity) in &self.entities {
            let entity = entity.borrow_mut();
            if let EntityState::ReturningResource(structure_id) = entity.state {
                let mut returner = entity;
                if returner.unit_mut().sub_cell_movement.is_ready() {
                    if let Some(structure) = self.find_entity(structure_id) {
                        let structure = structure.borrow();
                        if is_unit_within_melee_range_of(returner.position, structure.cell_rect()) {
                            self.team_state(&returner.team).borrow_mut().resources += 1;
                            // Unit goes back out to gather more
                            let gathering = returner.unit_mut().gathering.as_mut().unwrap();
                            let resource_id = gathering.drop_resource();
                            let resource = self.entity(resource_id).borrow();
                            if let Some(plan) = pathfind::find_path(
                                returner.position,
                                Destination::AdjacentToEntity(resource.cell_rect()),
                                &self.obstacle_grid,
                            ) {
                                returner.unit_mut().movement_plan.set(plan);
                                returner.state = EntityState::GatheringResource(resource_id);
                            } else {
                                returner.state = EntityState::Idle;
                            }
                        }
                    } else {
                        println!(
                            "Tried to return resource to structure that doesn't exist anymore. Idling."
                        );
                        returner.state = EntityState::Idle;
                    };
                }
            }
        }

        //-------------------------------
        //     PREPARE CONSTRUCTION
        //-------------------------------
        let mut builders_to_remove = Vec::new();
        let mut structures_to_add = Vec::new();
        for (entity_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            if let EntityState::Constructing(structure_type, structure_position) = entity.state {
                let has_arrived = entity.unit_mut().movement_plan.peek().is_none()
                    && entity.unit_mut().sub_cell_movement.is_ready();
                if has_arrived {
                    let size = self.structure_sizes.get(&structure_type).unwrap();
                    let mut sufficient_space = true;
                    println!(
                        "Check if structure can fit. Worker pos: {:?}, Structure pos: {:?}, Structure size: {:?}",
                        entity.position, structure_position, size
                    );
                    for x in structure_position[0]..structure_position[0] + size[0] {
                        for y in structure_position[1]..structure_position[1] + size[1] {
                            if [x, y] != entity.position {
                                // Don't check for collision on the cell that the builder stands on,
                                // since it will be removed when structure is added.
                                if self.obstacle_grid.get(&[x, y]).is_some() {
                                    sufficient_space = false;
                                    println!("Not enough space. Occupied cell: {:?}", [x, y]);
                                }
                            }
                        }
                    }
                    if sufficient_space {
                        builders_to_remove.push(*entity_id);
                        structures_to_add.push((entity.team, structure_position, structure_type));
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
        let mut removed_entities = vec![];
        self.entities.retain(|(entity_id, entity)| {
            let entity = entity.borrow();
            let is_dead = entity
                .health
                .as_ref()
                .map(|health| health.current == 0)
                .unwrap_or(false);
            let is_transforming_into_structure = builders_to_remove.contains(entity_id);
            if is_dead || is_transforming_into_structure {
                if entity.is_solid {
                    self.obstacle_grid.set_area(entity.cell_rect(), None);
                }
                removed_entities.push(*entity_id);
                false
            } else {
                true
            }
        });

        //-------------------------------
        //     START CONSTRUCTION
        //-------------------------------
        // Now that the builder has been removed, and no longer occupies a cell, the structure can
        // be placed.
        for (team, position, structure_type) in structures_to_add {
            let mut new_structure = data::create_entity(structure_type, position, team);
            let construction_time = Duration::from_secs(4); //TODO should come from entity config
            new_structure.state =
                EntityState::UnderConstruction(construction_time, construction_time);
            self.add_entity(new_structure);
        }

        //-------------------------------
        //     CONSTRUCTION
        //-------------------------------
        let mut finished_structures = vec![];
        for (id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            if let EntityState::UnderConstruction(remaining, total) = entity.state {
                let remaining = remaining.saturating_sub(dt);
                if remaining.is_zero() {
                    entity.state = EntityState::Idle;
                    finished_structures.push(*id);
                } else {
                    entity.state = EntityState::UnderConstruction(remaining, total);
                }
            }
        }

        //-------------------------------
        //          TRAINING
        //-------------------------------
        let mut completed_trainings = Vec::new();
        for (_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            if let EntityState::TrainingUnit(trained_entity_type) = entity.state {
                let status = entity.training.as_mut().map(|training| training.update(dt));
                if let Some(TrainingUpdateStatus::Done) = status {
                    entity.state = EntityState::Idle;
                    completed_trainings.push((
                        trained_entity_type,
                        entity.team,
                        entity.cell_rect(),
                    ));
                }
            }
        }
        for (entity_type, team, source_rect) in completed_trainings {
            if self
                .try_add_trained_entity(entity_type, team, source_rect)
                .is_none()
            {
                eprintln!("Failed to create entity around {:?}", source_rect);
            }
        }

        UpdateOutcome {
            removed_entities,
            finished_structures,
        }
    }

    pub fn issue_command(&self, command: Command, issuing_team: Team) {
        match command {
            Command::Train(TrainCommand {
                mut trainer,
                trained_unit_type,
                config,
            }) => {
                assert_eq!(trainer.team, issuing_team);
                let mut team_state = self.teams.get(&issuing_team).unwrap().borrow_mut();
                let training = trainer
                    .training
                    .as_mut()
                    .expect("Training command was issued for entity that can't train");
                if team_state.resources >= config.cost {
                    if let TrainingPerformStatus::NewTrainingStarted =
                        training.try_start(trained_unit_type)
                    {
                        trainer.state = EntityState::TrainingUnit(trained_unit_type);
                        team_state.resources -= config.cost;
                    }
                }
            }

            Command::Construct(ConstructCommand {
                mut builder,
                structure_position,
                structure_type,
            }) => {
                assert_eq!(builder.team, issuing_team);
                builder.state = EntityState::Constructing(structure_type, structure_position);
                let structure_rect = CellRect {
                    position: structure_position,
                    size: *self.structure_sizes.get(&structure_type).unwrap(),
                };
                if let Some(plan) = pathfind::find_path(
                    builder.position,
                    Destination::AdjacentToEntity(structure_rect),
                    &self.obstacle_grid,
                ) {
                    builder.unit_mut().movement_plan.set(plan);
                }
            }

            Command::Stop(StopCommand {
                entity: mut stopper,
            }) => {
                assert_eq!(stopper.team, issuing_team);
                stopper.state = EntityState::Idle;
                if let PhysicalType::Unit(unit) = stopper.physical_type.borrow_mut() {
                    unit.movement_plan.clear();
                }
            }

            Command::Move(MoveCommand {
                unit: mut mover,
                destination,
            }) => {
                assert_eq!(mover.team, issuing_team);
                if let Some(plan) = pathfind::find_path(
                    mover.position,
                    Destination::Point(destination),
                    &self.obstacle_grid,
                ) {
                    mover.state = EntityState::Moving;
                    mover.unit_mut().movement_plan.set(plan);
                }
            }

            Command::Attack(AttackCommand {
                mut attacker,
                victim,
            }) => {
                assert_eq!(attacker.team, issuing_team);
                assert_ne!(victim.team, issuing_team);
                attacker.state = EntityState::Attacking(victim.id);
                if let Some(plan) = pathfind::find_path(
                    attacker.position,
                    Destination::AdjacentToEntity(victim.cell_rect()),
                    &self.obstacle_grid,
                ) {
                    attacker.unit_mut().movement_plan.set(plan);
                }
            }

            Command::GatherResource(GatherResourceCommand {
                mut gatherer,
                resource,
            }) => {
                assert_eq!(gatherer.team, issuing_team);
                assert_eq!(resource.team, Team::Neutral);
                let is_carrying_resource = gatherer
                    .unit_mut()
                    .gathering
                    .as_ref()
                    .unwrap()
                    .is_carrying();
                if is_carrying_resource {
                    // TODO improve UI so that no player input leads to this situation
                    eprintln!(
                        "WARN: {:?} was issued to gather a resource, but they already carry some",
                        gatherer.id
                    );
                    return;
                }
                gatherer.state = EntityState::GatheringResource(resource.id);
                if let Some(plan) = pathfind::find_path(
                    gatherer.position,
                    Destination::AdjacentToEntity(resource.cell_rect()),
                    &self.obstacle_grid,
                ) {
                    gatherer.unit_mut().movement_plan.set(plan);
                }
            }

            Command::ReturnResource(ReturnResourceCommand {
                mut gatherer,
                structure,
            }) => {
                assert_eq!(gatherer.team, issuing_team);
                let is_carrying_resource = gatherer
                    .unit_mut()
                    .gathering
                    .as_ref()
                    .unwrap()
                    .is_carrying();
                if is_carrying_resource {
                    self.unit_return_resource(gatherer, structure);
                } else {
                    // TODO improve UI so that no player input leads to this situation
                    eprintln!(
                        "WARN: {:?} was issued to return a resource, but they don't carry any",
                        gatherer.id
                    );
                }
            }
        }
    }

    fn unit_return_resource(&self, mut gatherer: RefMut<Entity>, structure: Option<Ref<Entity>>) {
        let structure = structure.or_else(|| {
            // No specific structure was selected as the destination, so we pick one
            for (_entity_id, entity) in &self.entities {
                match entity.try_borrow() {
                    Ok(entity) if entity.team == gatherer.team => {
                        // For now, resources can be returned to any friendly structure
                        if let PhysicalType::Structure { .. } = entity.physical_type {
                            //TODO find the closest structure
                            return Some(entity);
                        }
                    }
                    _ => {}
                };
            }
            None
        });

        if let Some(structure) = structure {
            gatherer.state = EntityState::ReturningResource(structure.id);

            if let Some(plan) = pathfind::find_path(
                gatherer.position,
                Destination::AdjacentToEntity(structure.cell_rect()),
                &self.obstacle_grid,
            ) {
                gatherer.unit_mut().movement_plan.set(plan);
            }
        } else {
            gatherer.state = EntityState::Idle;
            eprintln!("WARN: Couldn't return resource. No structure found?");
        }
    }

    pub fn team_state(&self, team: &Team) -> &RefCell<TeamState> {
        self.teams.get(team).expect("Unknown team")
    }

    pub fn entities(&self) -> &[(EntityId, RefCell<Entity>)] {
        &self.entities
    }

    pub fn dimensions(&self) -> [u32; 2] {
        self.obstacle_grid.dimensions
    }

    pub fn structure_size(&self, structure_type: &EntityType) -> &[u32; 2] {
        self.structure_sizes
            .get(structure_type)
            .expect("Unknown structure type")
    }

    pub fn obstacle_grid(&self) -> &Grid<ObstacleType> {
        &self.obstacle_grid
    }

    fn try_add_trained_entity(
        &mut self,
        entity_type: EntityType,
        team: Team,
        source_rect: CellRect,
    ) -> Option<[u32; 2]> {
        let left = source_rect.position[0].saturating_sub(1);
        let top = source_rect.position[1].saturating_sub(1);
        let right = min(
            source_rect.position[0] + source_rect.size[0],
            self.obstacle_grid.dimensions[0] - 1,
        );
        let bot = min(
            source_rect.position[1] + source_rect.size[1],
            self.obstacle_grid.dimensions[1] - 1,
        );
        for x in left..right + 1 {
            for y in top..bot + 1 {
                if self.obstacle_grid.get(&[x, y]).is_none() {
                    let new_unit = data::create_entity(entity_type, [x, y], team);
                    self.add_entity(new_unit);
                    return Some([x, y]);
                }
            }
        }
        None
    }

    fn add_entity(&mut self, new_entity: Entity) {
        let rect = new_entity.cell_rect();
        let team = new_entity.team;
        self.entities
            .push((new_entity.id, RefCell::new(new_entity)));
        self.obstacle_grid
            .set_area(rect, Some(ObstacleType::Entity(team)));
    }

    fn entity(&self, id: EntityId) -> &RefCell<Entity> {
        self.find_entity(id)
            .unwrap_or_else(|| panic!("Entity not found: {:?}", id))
    }

    fn find_entity(&self, id: EntityId) -> Option<&RefCell<Entity>> {
        //println!("find_entity({:?})", id);
        self.entities.iter().find_map(
            |(entity_id, entity)| {
                if entity_id == &id {
                    Some(entity)
                } else {
                    None
                }
            },
        )
    }
}

fn is_unit_within_melee_range_of(unit_position: [u32; 2], rect: CellRect) -> bool {
    let mut is_attacker_within_range = false;
    for x in rect.position[0]..rect.position[0] + rect.size[0] {
        for y in rect.position[1]..rect.position[1] + rect.size[1] {
            if square_distance(unit_position, [x, y]) <= 2 {
                is_attacker_within_range = true;
            }
        }
    }
    is_attacker_within_range
}

fn square_distance(a: [u32; 2], b: [u32; 2]) -> u32 {
    ((a[0] as i32 - b[0] as i32).pow(2) + (a[1] as i32 - b[1] as i32).pow(2)) as u32
}

#[derive(Debug)]
pub enum Command<'a> {
    Train(TrainCommand<'a>),
    Construct(ConstructCommand<'a>),
    Stop(StopCommand<'a>),
    Move(MoveCommand<'a>),
    Attack(AttackCommand<'a>),
    GatherResource(GatherResourceCommand<'a>),
    ReturnResource(ReturnResourceCommand<'a>),
}

#[derive(Debug)]
pub struct TrainCommand<'a> {
    pub trainer: RefMut<'a, Entity>,
    pub trained_unit_type: EntityType,
    pub config: TrainingConfig,
}

#[derive(Debug)]
pub struct ConstructCommand<'a> {
    pub builder: RefMut<'a, Entity>,
    pub structure_position: [u32; 2],
    pub structure_type: EntityType,
}

#[derive(Debug)]
pub struct StopCommand<'a> {
    pub entity: RefMut<'a, Entity>,
}

#[derive(Debug)]
pub struct MoveCommand<'a> {
    pub unit: RefMut<'a, Entity>,
    pub destination: [u32; 2],
}

#[derive(Debug)]
pub struct AttackCommand<'a> {
    pub attacker: RefMut<'a, Entity>,
    pub victim: Ref<'a, Entity>,
}

#[derive(Debug)]
pub struct GatherResourceCommand<'a> {
    pub gatherer: RefMut<'a, Entity>,
    pub resource: Ref<'a, Entity>,
}

#[derive(Debug)]
pub struct ReturnResourceCommand<'a> {
    pub gatherer: RefMut<'a, Entity>,
    pub structure: Option<Ref<'a, Entity>>,
}

pub struct TeamState {
    pub resources: u32,
}

pub struct UpdateOutcome {
    pub removed_entities: Vec<EntityId>,
    pub finished_structures: Vec<EntityId>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ObstacleType {
    Entity(Team),
    Water,
}
