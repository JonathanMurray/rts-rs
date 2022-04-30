use std::cell::{Ref, RefCell, RefMut};
use std::cmp::min;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;

use crate::data::{self, EntityType};
use crate::entities::{
    Direction, Entity, EntityCategory, EntityId, EntityState, GatheringProgress, Team,
    TrainingPerformStatus, TrainingUpdateStatus,
};
use crate::grid::{CellRect, Grid};
use crate::pathfind::{self, Destination};

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
        let mut teams: HashMap<Team, RefCell<TeamState>> = HashMap::new();
        for entity in &entities {
            if let Entry::Vacant(entry) = teams.entry(entity.team) {
                entry.insert(RefCell::new(TeamState { resources: 15 }));
            }
        }

        let mut obstacle_grid = Grid::new(world_dimensions);
        for water_cell in water_cells {
            obstacle_grid.set(water_cell, ObstacleType::Water);
        }
        for entity in &entities {
            // TODO Store EntityId's instead, to get constant position->entity_id lookup?
            //      (although entity_id->entity is still not constant currently)
            obstacle_grid.set_area(entity.cell_rect(), ObstacleType::Entity(entity.team));
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
            if let EntityCategory::Unit(unit) = &mut entity.category {
                unit.sub_cell_movement.update(dt, pos);
                if !unit.sub_cell_movement.is_between_cells() {
                    if let Some(next_pos) = unit.movement_plan.peek() {
                        if self.obstacle_grid.get(&next_pos).unwrap() == ObstacleType::None {
                            let old_pos = pos;
                            let new_pos = unit.movement_plan.advance();
                            unit.move_to_adjacent_cell(old_pos, new_pos);
                            entity.position = new_pos;
                            self.obstacle_grid.set(old_pos, ObstacleType::None);
                            self.obstacle_grid
                                .set(new_pos, ObstacleType::Entity(entity.team));
                        } else {
                            let blocked_for_too_long = unit.movement_plan.on_movement_blocked();
                            if blocked_for_too_long {
                                let destination = unit.movement_plan.destination();
                                if let Some(plan) = pathfind::find_path(
                                    pos,
                                    Destination::Point(destination),
                                    &self.obstacle_grid,
                                ) {
                                    println!("Blocked unit found new path");
                                    unit.movement_plan.set(plan);
                                } else {
                                    println!(
                                        "Blocked unit couldn't find new path. Back to idling."
                                    );
                                    unit.movement_plan.clear();
                                    entity.state = EntityState::Idle;
                                }
                            }
                        }
                    } else if entity.state == EntityState::Moving {
                        // Unit reached its destination
                        entity.state = EntityState::Idle;
                    }
                }

                entity.animation.ms_counter = entity
                    .animation
                    .ms_counter
                    .wrapping_add(dt.as_millis() as u16);
            }
        }

        //-------------------------------
        //      MOVING TO COMBAT
        //-------------------------------
        for (_entity_id, entity) in &self.entities {
            let entity = entity.borrow_mut();

            if let EntityState::MovingToAttackTarget(victim_id) = entity.state {
                let mut attacker = entity;
                if let Some(victim) = self.find_entity(victim_id) {
                    let victim = victim.borrow_mut();
                    if let Some(direction) =
                        unit_melee_direction(attacker.position, victim.cell_rect())
                    {
                        attacker.state = EntityState::Attacking(victim_id);
                        let unit = attacker.unit_mut();
                        if !unit.sub_cell_movement.is_between_cells() {
                            attacker.unit_mut().direction = direction;
                        }
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
                    // Attacked target no longer exists
                    attacker.state = EntityState::Idle;
                    attacker.unit_mut().movement_plan.clear();
                }
            }
        }

        //-------------------------------
        //        ATTACKING
        //-------------------------------
        for (_entity_id, entity) in &self.entities {
            let mut entity = entity.borrow_mut();

            if let EntityCategory::Unit(unit) = &mut entity.category {
                if let Some(combat) = &mut unit.combat {
                    combat.count_down_cooldown(dt);
                }
            }

            if let EntityState::Attacking(victim_id) = entity.state {
                let mut attacker = entity;
                let combat = attacker
                    .unit_mut()
                    .combat
                    .as_mut()
                    .expect("non-combat attacker");
                if combat.is_attack_ready() {
                    if let Some(victim) = self.find_entity(victim_id) {
                        let mut victim = victim.borrow_mut();
                        if let Some(direction) =
                            unit_melee_direction(attacker.position, victim.cell_rect())
                        {
                            let health = victim.health.as_mut().expect("victim without health");
                            // TODO get damage amount from unit config
                            let damage_amount = 1;
                            health.receive_damage(damage_amount);
                            println!(
                                "{:?} --[{} dmg]--> {:?}",
                                attacker.id, damage_amount, victim_id
                            );
                            let unit = attacker.unit_mut();
                            if !unit.sub_cell_movement.is_between_cells() {
                                unit.direction = direction;
                            }
                            unit.combat.as_mut().unwrap().start_cooldown();
                        } else {
                            // Attacked target is not in range
                            attacker.state = EntityState::MovingToAttackTarget(victim_id);
                            if let Some(plan) = pathfind::find_path(
                                attacker.position,
                                Destination::AdjacentToEntity(victim.cell_rect()),
                                &self.obstacle_grid,
                            ) {
                                attacker.unit_mut().movement_plan.set(plan);
                            }
                        }
                    } else {
                        // Attacked target no longer exists
                        attacker.state = EntityState::Idle;
                        attacker.unit_mut().movement_plan.clear();
                    }
                }
            }
        }

        //-------------------------------
        //     MOVING TO RESOURCE
        //-------------------------------
        for (_entity_id, entity) in &self.entities {
            let entity = entity.borrow_mut();
            if let EntityState::MovingToResource(resource_id) = entity.state {
                let mut gatherer = entity;
                if !gatherer.unit_mut().sub_cell_movement.is_between_cells() {
                    if let Some(resource) = self.find_entity(resource_id) {
                        let resource = resource.borrow();
                        if let Some(direction) =
                            unit_melee_direction(gatherer.position, resource.cell_rect())
                        {
                            gatherer.state = EntityState::GatheringResource(resource_id);
                            let unit = gatherer.unit_mut();
                            unit.direction = direction;
                            let gathering = unit.gathering.as_mut().unwrap();
                            gathering.start_gathering();
                        }
                    } else {
                        println!("Arrived at resource, but it's gone");
                        gatherer.state = EntityState::Idle;
                    }
                }
            }
        }

        //-------------------------------
        //     GATHERING RESOURCE
        //-------------------------------
        let mut used_up_resources = vec![];
        for (_entity_id, entity) in &self.entities {
            let entity = entity.borrow_mut();
            if let EntityState::GatheringResource(resource_id) = entity.state {
                let mut gatherer = entity;
                if let Some(resource) = self.find_entity(resource_id) {
                    let mut resource = resource.borrow_mut();
                    let remaining = resource.resource_remaining_mut();
                    if *remaining > 0 {
                        let gathering = gatherer.unit_mut().gathering.as_mut().unwrap();
                        if let GatheringProgress::Done =
                            gathering.make_progress_on_gathering(dt, resource_id)
                        {
                            *remaining = remaining.saturating_sub(1);
                            if *remaining == 0 {
                                used_up_resources.push(resource_id);
                            }
                            self.unit_return_resource(gatherer, None);
                        }
                    }
                } else {
                    println!("Resource disappeared while it was being gathered");
                    gatherer.state = EntityState::Idle;
                }
            }
        }

        //-------------------------------
        //     RETURNING RESOURCE
        //-------------------------------
        for (_entity_id, entity) in &self.entities {
            let entity = entity.borrow_mut();
            if let EntityState::ReturningResource(structure_id) = entity.state {
                let mut returner = entity;
                if !returner.unit_mut().sub_cell_movement.is_between_cells() {
                    if let Some(structure) = self.find_entity(structure_id) {
                        let structure = structure.borrow();
                        if let Some(direction) =
                            unit_melee_direction(returner.position, structure.cell_rect())
                        {
                            self.team_state_unchecked(&returner.team)
                                .borrow_mut()
                                .resources += 1;

                            let unit = returner.unit_mut();
                            unit.direction = direction;
                            let gathering = unit.gathering.as_mut().unwrap();
                            let resource_id = gathering.drop_resource();
                            // Unit goes back out to gather more
                            if let Some(resource) = self.find_entity(resource_id) {
                                if let Some(plan) = pathfind::find_path(
                                    returner.position,
                                    Destination::AdjacentToEntity(resource.borrow().cell_rect()),
                                    &self.obstacle_grid,
                                ) {
                                    returner.unit_mut().movement_plan.set(plan);
                                    returner.state = EntityState::MovingToResource(resource_id);
                                } else {
                                    returner.state = EntityState::Idle;
                                }
                            } else {
                                println!("Can't go back to resource since it's gone");
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
            if let EntityState::MovingToConstruction(structure_type, structure_position) =
                entity.state
            {
                // TODO should movement_plan and sub_cell_movement be turned into one single thing?
                let has_arrived = entity.unit_mut().movement_plan.peek().is_none()
                    && !entity.unit_mut().sub_cell_movement.is_between_cells();
                if has_arrived {
                    let structure_size = *self.structure_sizes.get(&structure_type).unwrap();
                    if self.can_structure_fit(entity.position, structure_position, structure_size) {
                        let constructions_options =
                            entity.unit().construction_options.as_ref().unwrap();
                        let construction_time = constructions_options
                            .get(&structure_type)
                            .unwrap()
                            .construction_time;

                        // Mark worker for removal and free occupied grid cell
                        builders_to_remove.push(*entity_id);
                        self.obstacle_grid
                            .set_area(entity.cell_rect(), ObstacleType::None);

                        // Plan for structure creation and claim occupied grid cells
                        structures_to_add.push((
                            entity.team,
                            structure_position,
                            structure_type,
                            construction_time,
                        ));
                        let structure_rect = CellRect {
                            position: structure_position,
                            size: structure_size,
                        };
                        self.obstacle_grid
                            .set_area(structure_rect, ObstacleType::Entity(entity.team));
                    } else {
                        println!("There's not enough space for the structure, so builder goes back to idling");
                        let construction_options =
                            entity.unit_mut().construction_options.as_ref().unwrap();
                        let config = construction_options.get(&structure_type).unwrap();
                        self.team_state_unchecked(&entity.team)
                            .borrow_mut()
                            .resources += config.cost;
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
            if is_dead {
                Core::maybe_repay_construction_cost(&entity, &self.teams);
            }
            let is_transforming_into_structure = builders_to_remove.contains(entity_id);
            let is_used_up_resource = used_up_resources.contains(entity_id);

            // worker transforming into structure has already cleared its grid cell
            if is_dead || is_used_up_resource {
                let cell_rect = entity.cell_rect();
                self.obstacle_grid.set_area(cell_rect, ObstacleType::None);
            }

            if is_dead || is_transforming_into_structure || is_used_up_resource {
                removed_entities.push(*entity_id);
                false
            } else {
                true
            }
        });

        //-------------------------------
        //     START CONSTRUCTION
        //-------------------------------
        for (team, position, structure_type, construction_time) in structures_to_add {
            let mut new_structure = data::create_entity(structure_type, position, team);
            new_structure.state =
                EntityState::UnderConstruction(construction_time, construction_time);
            self.entities
                .push((new_structure.id, RefCell::new(new_structure)));
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

    pub fn can_structure_fit(
        &self,
        worker_position: [u32; 2],
        structure_position: [u32; 2],
        structure_size: [u32; 2],
    ) -> bool {
        let mut can_fit = true;
        for x in structure_position[0]..structure_position[0] + structure_size[0] {
            for y in structure_position[1]..structure_position[1] + structure_size[1] {
                if [x, y] != worker_position {
                    // Don't check for collision on the cell that the builder stands on,
                    // since it will be removed when structure is added.
                    let is_occupied = self
                        .obstacle_grid
                        .get(&[x, y])
                        .map_or(true, |obstacle| obstacle != ObstacleType::None);
                    if is_occupied {
                        can_fit = false;
                    }
                }
            }
        }
        can_fit
    }

    fn maybe_repay_construction_cost(entity: &Entity, teams: &HashMap<Team, RefCell<TeamState>>) {
        if let EntityState::MovingToConstruction(structure_type, ..) = entity.state {
            let construction_options = entity.unit().construction_options.as_ref().unwrap();
            let config = construction_options.get(&structure_type).unwrap();
            let mut team_state = teams.get(&entity.team).unwrap().borrow_mut();
            team_state.resources += config.cost;
            println!(
                "Repaying {} to {:?} due to cancelled construction",
                config.cost, entity.team
            );
        }
    }

    pub fn issue_command(&self, command: Command, issuing_team: Team) -> Option<CommandError> {
        Core::maybe_repay_construction_cost(command.actor().deref(), &self.teams);

        match command {
            Command::Train(TrainCommand {
                mut trainer,
                trained_unit_type,
            }) => {
                assert_eq!(trainer.team, issuing_team);
                let mut team_state = self.teams.get(&issuing_team).unwrap().borrow_mut();
                let training = trainer
                    .training
                    .as_mut()
                    .expect("Training command was issued for entity that can't train");

                let cost = training.config(&trained_unit_type).cost;

                if team_state.resources >= cost {
                    if let TrainingPerformStatus::NewTrainingStarted =
                        training.try_start(trained_unit_type)
                    {
                        trainer.state = EntityState::TrainingUnit(trained_unit_type);
                        team_state.resources -= cost;
                    }
                } else {
                    return Some(CommandError::NotEnoughResources);
                }
            }

            Command::Construct(ConstructCommand {
                mut builder,
                structure_position,
                structure_type,
            }) => {
                assert_eq!(builder.team, issuing_team);
                let unit = builder.unit_mut();
                let cost = unit
                    .construction_options
                    .as_mut()
                    .unwrap()
                    .get_mut(&structure_type)
                    .unwrap()
                    .cost;
                let mut team_state = self.teams.get(&issuing_team).unwrap().borrow_mut();

                if team_state.resources < cost {
                    return Some(CommandError::NotEnoughResources);
                }

                let structure_size = self.structure_sizes.get(&structure_type).unwrap();
                if !self.can_structure_fit(builder.position, structure_position, *structure_size) {
                    return Some(CommandError::NotEnoughSpaceForStructure);
                }

                let structure_rect = CellRect {
                    position: structure_position,
                    size: *self.structure_sizes.get(&structure_type).unwrap(),
                };
                if let Some(plan) = pathfind::find_path(
                    builder.position,
                    Destination::AdjacentToEntity(structure_rect),
                    &self.obstacle_grid,
                ) {
                    team_state.resources -= cost;
                    builder.unit_mut().movement_plan.set(plan);
                    builder.state =
                        EntityState::MovingToConstruction(structure_type, structure_position);
                } else {
                    return Some(CommandError::NoPathFound);
                }
            }

            Command::Stop(StopCommand {
                entity: mut stopper,
            }) => {
                assert_eq!(stopper.team, issuing_team);
                stopper.state = EntityState::Idle;
                stopper.unit_mut().movement_plan.clear();
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
                } else {
                    return Some(CommandError::NoPathFound);
                }
            }

            Command::Attack(AttackCommand {
                mut attacker,
                victim,
            }) => {
                assert_eq!(attacker.team, issuing_team);
                assert_ne!(victim.team, issuing_team);
                if let Some(plan) = pathfind::find_path(
                    attacker.position,
                    Destination::AdjacentToEntity(victim.cell_rect()),
                    &self.obstacle_grid,
                ) {
                    attacker.state = EntityState::Attacking(victim.id);
                    attacker.unit_mut().movement_plan.set(plan);
                } else {
                    return Some(CommandError::NoPathFound);
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
                    println!("Unit is already carrying resources, reinterpreting gather command as return command.");
                    self.unit_return_resource(gatherer, None);
                } else if let Some(plan) = pathfind::find_path(
                    gatherer.position,
                    Destination::AdjacentToEntity(resource.cell_rect()),
                    &self.obstacle_grid,
                ) {
                    gatherer.state = EntityState::MovingToResource(resource.id);
                    gatherer.unit_mut().movement_plan.set(plan);
                } else {
                    return Some(CommandError::NoPathFound);
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
                    return Some(CommandError::NotCarryingResource);
                }
            }
        }
        None
    }

    fn unit_return_resource(&self, mut gatherer: RefMut<Entity>, structure: Option<Ref<Entity>>) {
        let structure = structure.or_else(|| {
            // No specific structure was selected as the destination, so we pick one
            for (_entity_id, entity) in &self.entities {
                match entity.try_borrow() {
                    Ok(entity) if entity.team == gatherer.team => {
                        // For now, resources can be returned to any friendly structure
                        if let EntityCategory::Structure { .. } = entity.category {
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
            if let Some(plan) = pathfind::find_path(
                gatherer.position,
                Destination::AdjacentToEntity(structure.cell_rect()),
                &self.obstacle_grid,
            ) {
                gatherer.state = EntityState::ReturningResource(structure.id);
                gatherer.unit_mut().movement_plan.set(plan);
            } else {
                gatherer.state = EntityState::Idle;
            }
        } else {
            gatherer.state = EntityState::Idle;
            eprintln!("WARN: Couldn't return resource. No structure found?");
        }
    }

    pub fn team_state_unchecked(&self, team: &Team) -> &RefCell<TeamState> {
        self.teams
            .get(team)
            .unwrap_or_else(|| panic!("Unknown team: {:?}", team))
    }

    pub fn team_state(&self, team: &Team) -> Option<&RefCell<TeamState>> {
        self.teams.get(team)
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
                let is_free = self
                    .obstacle_grid
                    .get(&[x, y])
                    .map_or(false, |obstacle| obstacle == ObstacleType::None);
                if is_free {
                    let new_unit = data::create_entity(entity_type, [x, y], team);
                    let rect = new_unit.cell_rect();
                    let team = new_unit.team;
                    self.entities.push((new_unit.id, RefCell::new(new_unit)));
                    self.obstacle_grid
                        .set_area(rect, ObstacleType::Entity(team));
                    return Some([x, y]);
                }
            }
        }
        None
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

fn unit_melee_direction(unit_position: [u32; 2], rect: CellRect) -> Option<Direction> {
    for x in rect.position[0]..rect.position[0] + rect.size[0] {
        for y in rect.position[1]..rect.position[1] + rect.size[1] {
            if square_distance(unit_position, [x, y]) <= 2 {
                let unit_x = unit_position[0] as i32;
                let unit_y = unit_position[1] as i32;

                match [x as i32 - unit_x, y as i32 - unit_y] {
                    [0, -1] => return Some(Direction::North),
                    [1, -1] => return Some(Direction::NorthEast),
                    [1, 0] => return Some(Direction::East),
                    [1, 1] => return Some(Direction::SouthEast),
                    [0, 1] => return Some(Direction::South),
                    [-1, 1] => return Some(Direction::SouthWest),
                    [-1, 0] => return Some(Direction::West),
                    [-1, -1] => return Some(Direction::NorthWest),
                    _ => {}
                }
            }
        }
    }
    None
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

impl<'a> Command<'a> {
    fn actor(&self) -> &RefMut<'a, Entity> {
        match self {
            Command::Train(TrainCommand { trainer, .. }) => trainer,
            Command::Construct(ConstructCommand { builder, .. }) => builder,
            Command::Stop(StopCommand { entity }) => entity,
            Command::Move(MoveCommand { unit, .. }) => unit,
            Command::Attack(AttackCommand { attacker, .. }) => attacker,
            Command::GatherResource(GatherResourceCommand { gatherer, .. }) => gatherer,
            Command::ReturnResource(ReturnResourceCommand { gatherer, .. }) => gatherer,
        }
    }
}

#[derive(Debug)]
pub struct TrainCommand<'a> {
    pub trainer: RefMut<'a, Entity>,
    pub trained_unit_type: EntityType,
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
    None,
}

impl Default for ObstacleType {
    fn default() -> Self {
        ObstacleType::None
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CommandError {
    NotEnoughResources,
    NoPathFound,
    NotCarryingResource,
    NotEnoughSpaceForStructure,
}
