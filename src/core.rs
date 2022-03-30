use std::cmp::min;
use std::collections::HashMap;
use std::time::Duration;

use crate::data::{self, EntityType};
use crate::entities::{
    Action, Entity, EntityId, EntityState, PhysicalType, Team, TrainingConfig,
    TrainingPerformStatus, TrainingUpdateStatus,
};
use crate::grid::EntityGrid;

pub struct Core {
    teams: HashMap<Team, TeamState>,
    entities: Vec<Entity>,
    entity_grid: EntityGrid,
    structure_sizes: HashMap<EntityType, [u32; 2]>,
}

impl Core {
    pub fn new(entities: Vec<Entity>, world_dimensions: [u32; 2]) -> Self {
        let mut teams = HashMap::new();
        teams.insert(Team::Player, TeamState { resources: 5 });
        teams.insert(Team::Enemy, TeamState { resources: 5 });

        let mut entity_grid = EntityGrid::new(world_dimensions);
        for entity in &entities {
            if entity.is_solid {
                entity_grid.set_area(&entity.position, &entity.size(), true);
            }
        }
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
                if is_unit_within_melee_range_of(attacker_pos, victim_pos, victim.size()) {
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
        //     RESOURCE GATHERING
        //-------------------------------
        let mut gatherers = vec![];
        for entity in &mut self.entities {
            if let EntityState::Gathering(resource_id) = entity.state {
                let gatherer_id = entity.id;
                gatherers.push((gatherer_id, resource_id));
            }
        }
        for (gatherer_id, resource_id) in gatherers {
            let gatherer_pos = self.entity_mut(gatherer_id).position;
            let success =
                if let Some(resource) = self.entities.iter_mut().find(|e| e.id == resource_id) {
                    let resource_pos = resource.position;
                    let resource_size = resource.size();
                    is_unit_within_melee_range_of(gatherer_pos, resource_pos, resource_size)
                } else {
                    panic!("Resource doesn't exist");
                };

            if success {
                let gatherer = self.entity_mut(gatherer_id);
                gatherer
                    .unit_mut()
                    .gathering
                    .as_mut()
                    .unwrap()
                    .pick_up_resource();
                //gatherer.state = EntityState::Idle;
                //println!("{:?} gathered some resource and is now idling", gatherer.id );
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
        let mut removed_entity_ids = vec![];
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
                removed_entity_ids.push(entity.id);
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

        removed_entity_ids
    }

    pub fn issue_command(&mut self, command: Command, issuing_team: Team) {
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
            Command::GatherResource(gatherer_id, resource_id) => {
                let resource = self.entity_mut(resource_id);
                assert_eq!(resource.team, Team::Neutral);
                let resource_pos = resource.position;
                let gatherer = self.entity_mut(gatherer_id);
                assert_eq!(gatherer.team, issuing_team);
                gatherer.state = EntityState::Gathering(resource_id);
                let gatherer_pos = gatherer.position;
                gatherer
                    .unit_mut()
                    .pathfinder
                    .find_path(&gatherer_pos, resource_pos);
            }
        }
    }

    pub fn team_state(&self, team: &Team) -> &TeamState {
        self.teams.get(team).expect("Unknown team")
    }

    pub fn entities(&self) -> &[Entity] {
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
        self.entities.push(new_entity);
        self.entity_grid.set_area(&position, &size, true);
    }

    fn entity_mut(&mut self, id: EntityId) -> &mut Entity {
        self.entities
            .iter_mut()
            .find(|e| e.id == id)
            .expect("entity must exist")
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
pub enum Command {
    Train(EntityId, EntityType, TrainingConfig),
    Construct(EntityId, [u32; 2], EntityType),
    Move(EntityId, [u32; 2]),
    Heal(EntityId),
    Attack(EntityId, EntityId),
    GatherResource(EntityId, EntityId),
}

pub struct TeamState {
    pub resources: u32,
}
