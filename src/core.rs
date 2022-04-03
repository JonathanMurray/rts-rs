use std::cell::{Ref, RefCell, RefMut};
use std::cmp::min;
use std::collections::HashMap;
use std::time::Duration;

use crate::data::{self, EntityType};
use crate::entities::{
    Entity, EntityId, EntityState, PhysicalType, Team, TrainingConfig, TrainingPerformStatus,
    TrainingUpdateStatus,
};
use crate::grid::EntityGrid;
use crate::pathfind::{self, Destination};

pub struct Core {
    teams: HashMap<Team, RefCell<TeamState>>,
    entities: Vec<(EntityId, RefCell<Entity>)>,
    entity_grid: EntityGrid,
    structure_sizes: HashMap<EntityType, [u32; 2]>,
}

impl Core {
    pub fn new(entities: Vec<Entity>, world_dimensions: [u32; 2]) -> Self {
        let mut teams = HashMap::new();
        teams.insert(Team::Player, RefCell::new(TeamState { resources: 5 }));
        teams.insert(Team::Enemy, RefCell::new(TeamState { resources: 5 }));

        let mut entity_grid = EntityGrid::new(world_dimensions);
        for entity in &entities {
            if entity.is_solid {
                entity_grid.set_area(&entity.position, &entity.size(), true);
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
            entity_grid,
            structure_sizes,
        }
    }

    pub fn update(&mut self, dt: Duration) -> Vec<EntityId> {
        //-------------------------------
        //          MOVEMENT
        //-------------------------------
        for (_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            let pos = entity.position;
            if let PhysicalType::Unit(unit) = &mut entity.physical_type {
                unit.sub_cell_movement.update(dt, pos);
                if unit.sub_cell_movement.is_ready() {
                    if let Some(next_pos) = unit.movement_plan.peek() {
                        let occupied = self.entity_grid.get(next_pos);
                        if !occupied {
                            let old_pos = pos;
                            let new_pos = unit.movement_plan.advance();
                            self.entity_grid.set(old_pos, false);
                            unit.sub_cell_movement.set_moving(old_pos, new_pos);
                            entity.position = new_pos;
                            self.entity_grid.set(new_pos, true);
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
                        if is_unit_within_melee_range_of(
                            attacker.position,
                            victim.position,
                            victim.size(),
                        ) {
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
                                Destination::AdjacentToEntity(victim.position, victim.size()),
                                &self.entity_grid,
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
        let mut gathering_candidates = vec![];
        for (entity_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            if let EntityState::GatheringResource(resource_id) = entity.state {
                let gatherer_id = entity_id;
                if entity.unit_mut().sub_cell_movement.is_ready() {
                    gathering_candidates.push((*gatherer_id, resource_id));
                }
            }
        }
        //TODO simplify now that we have RefCell?
        for (gatherer_id, resource_id) in gathering_candidates {
            let mut gatherer = self.entity(gatherer_id).borrow_mut();
            let gatherer_pos = gatherer.position;
            let success = if let Some(resource) = self.find_entity(resource_id) {
                let resource = resource.borrow();
                let resource_pos = resource.position;
                let resource_size = resource.size();
                is_unit_within_melee_range_of(gatherer_pos, resource_pos, resource_size)
            } else {
                panic!("Resource doesn't exist");
            };
            if success {
                gatherer
                    .unit_mut()
                    .gathering
                    .as_mut()
                    .unwrap()
                    .pick_up_resource(resource_id);
                self.unit_return_resource(gatherer, None);
            }
        }

        //-------------------------------
        //     RETURNING RESOURCES
        //-------------------------------
        let mut return_candidates = vec![];
        for (entity_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            if let EntityState::ReturningResource(structure_id) = entity.state {
                if entity.unit_mut().sub_cell_movement.is_ready() {
                    return_candidates.push((*entity_id, entity.team, structure_id));
                }
            }
        }
        //TODO simplify now that we have RefCell?
        for (returner_id, returner_team, structure_id) in return_candidates {
            let mut returner = self.entity(returner_id).borrow_mut();
            let returner_pos = returner.position;
            if let Some(structure) = self.find_entity(structure_id) {
                let structure = structure.borrow();
                if is_unit_within_melee_range_of(returner_pos, structure.position, structure.size())
                {
                    self.team_state(&returner_team).borrow_mut().resources += 1;
                    // Unit goes back out to gather more
                    let gathering = returner.unit_mut().gathering.as_mut().unwrap();
                    let resource_id = gathering.drop_resource();
                    let resource = self.entity(resource_id).borrow();
                    if let Some(plan) = pathfind::find_path(
                        returner.position,
                        Destination::AdjacentToEntity(resource.position, resource.size()),
                        &self.entity_grid,
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

        //-------------------------------
        //       CONSTRUCTION 1
        //-------------------------------
        let mut builders_to_remove = Vec::new();
        let mut structures_to_add = Vec::new();
        for (entity_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
            if let EntityState::Constructing(structure_type, structure_position) = entity.state {
                if entity.unit_mut().movement_plan.peek().is_none() {
                    //TODO Check if we have _fully_ arrived to the target cell
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
                                if self.entity_grid.get(&[x, y]) {
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
        let mut removed_entity_ids = vec![];
        self.entities.retain(|(entity_id, entity)| {
            let entity = entity.borrow();
            let is_dead = entity
                .health
                .as_ref()
                .map(|health| health.current == 0)
                .unwrap_or(false);
            let is_transforming_into_structure = builders_to_remove.contains(entity_id);
            if is_transforming_into_structure {
                println!("{:?} is transforming into a structure", entity_id);
            }
            let should_be_removed = is_dead || is_transforming_into_structure;

            if should_be_removed {
                if entity.is_solid {
                    self.entity_grid
                        .set_area(&entity.position, &entity.size(), false);
                }
                removed_entity_ids.push(*entity_id);
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
        for (_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();
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

        removed_entity_ids
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
                let structure_size = *self.structure_sizes.get(&structure_type).unwrap();
                if let Some(plan) = pathfind::find_path(
                    builder.position,
                    Destination::AdjacentToEntity(structure_position, structure_size),
                    &self.entity_grid,
                ) {
                    builder.unit_mut().movement_plan.set(plan);
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
                    &self.entity_grid,
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
                    Destination::AdjacentToEntity(victim.position, victim.size()),
                    &self.entity_grid,
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
                    Destination::AdjacentToEntity(resource.position, resource.size()),
                    &self.entity_grid,
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
        let structure_id_pos_size = match structure {
            Some(structure) => Some((structure.id, structure.position, structure.size())),
            None => {
                // No specific structure was selected as the destination, so we pick one
                let mut structure_id_pos_size = None;
                for (entity_id, entity) in &self.entities {
                    match entity.try_borrow() {
                        Ok(entity) if entity.team == gatherer.team => {
                            // For now, resources can be returned to any friendly structure
                            if let PhysicalType::Structure { size } = entity.physical_type {
                                //TODO find the closest structure
                                structure_id_pos_size = Some((*entity_id, entity.position, size));
                            }
                        }
                        _ => {}
                    };
                }
                structure_id_pos_size
            }
        };
        // TODO simplify with RefCell?
        if let Some((structure_id, structure_pos, structure_size)) = structure_id_pos_size {
            gatherer.state = EntityState::ReturningResource(structure_id);

            if let Some(plan) = pathfind::find_path(
                gatherer.position,
                Destination::AdjacentToEntity(structure_pos, structure_size),
                &self.entity_grid,
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
        self.entity_grid.dimensions
    }

    pub fn structure_size(&self, structure_type: &EntityType) -> &[u32; 2] {
        self.structure_sizes
            .get(structure_type)
            .expect("Unknown structure type")
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
        self.entities
            .push((new_entity.id, RefCell::new(new_entity)));
        self.entity_grid.set_area(&position, &size, true);
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

fn is_unit_within_melee_range_of(
    unit_position: [u32; 2],
    other_position: [u32; 2],
    other_size: [u32; 2],
) -> bool {
    let mut is_attacker_within_range = false;
    for x in other_position[0]..other_position[0] + other_size[0] {
        for y in other_position[1]..other_position[1] + other_size[1] {
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
