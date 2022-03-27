use std::cmp::{min, Ordering};
use std::collections::HashMap;
use std::sync::atomic::{self, AtomicUsize};
use std::time::Duration;

use crate::data::EntityType;
use crate::game;

static NEXT_ENTITY_ID: AtomicUsize = AtomicUsize::new(1);

pub const NUM_UNIT_ACTIONS: usize = 3;

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct EntityId(usize);

#[derive(Debug, PartialEq)]
pub enum EntityState {
    Idle,
    TrainingUnit(EntityType),
    Moving,
    Attacking,
}

#[derive(Debug)]
pub struct Entity {
    pub id: EntityId,
    pub name: &'static str,
    pub position: [u32; 2],
    pub is_solid: bool,
    pub physical_type: PhysicalType,
    pub team: Team,
    pub sprite: EntitySprite,
    pub health: Option<HealthComponent>,
    pub training: Option<TrainingComponent>,
    pub actions: [Option<Action>; NUM_UNIT_ACTIONS],
    pub state: EntityState,
}

#[derive(Debug)]
pub enum PhysicalType {
    Unit(UnitComponent),
    Structure { size: [u32; 2] },
}

pub struct EntityConfig {
    pub name: &'static str,
    pub is_solid: bool,
    pub sprite: EntitySprite,
    pub max_health: Option<u32>,
    pub physical_type: PhysicalTypeConfig,
}

pub enum PhysicalTypeConfig {
    MovementCooldown(Duration),
    StructureSize([u32; 2]),
}

impl Entity {
    pub fn new(
        config: EntityConfig,
        position: [u32; 2],
        team: Team,
        actions: [Option<Action>; NUM_UNIT_ACTIONS],
    ) -> Self {
        // Make sure all entities have unique IDs
        let id = EntityId(NEXT_ENTITY_ID.fetch_add(1, atomic::Ordering::Relaxed));

        let health = config.max_health.map(HealthComponent::new);
        let mut training_options: HashMap<EntityType, TrainingConfig> = Default::default();
        let mut has_combat = false;
        for action in actions.into_iter().flatten() {
            match action {
                Action::Train(entity_type, config) => {
                    training_options.insert(entity_type, config);
                }
                Action::Attack => has_combat = true,
                _ => {}
            }
        }
        let training = if !training_options.is_empty() {
            Some(TrainingComponent::new(training_options))
        } else {
            None
        };
        let physical_type = match config.physical_type {
            PhysicalTypeConfig::MovementCooldown(cooldown) => {
                let combat = if has_combat {
                    Some(Combat::new())
                } else {
                    None
                };
                PhysicalType::Unit(UnitComponent::new(position, cooldown, combat))
            }
            PhysicalTypeConfig::StructureSize(size) => PhysicalType::Structure { size },
        };

        Self {
            id,
            name: config.name,
            position,
            is_solid: config.is_solid,
            physical_type,
            team,
            sprite: config.sprite,
            health,
            training,
            actions,
            state: EntityState::Idle,
        }
    }

    pub fn size(&self) -> [u32; 2] {
        match self.physical_type {
            PhysicalType::Unit(_) => [1, 1],
            PhysicalType::Structure { size } => size,
        }
    }

    pub fn contains(&self, position: [u32; 2]) -> bool {
        let [w, h] = self.size();
        position[0] >= self.position[0]
            && position[0] < self.position[0] + w
            && position[1] >= self.position[1]
            && position[1] < self.position[1] + h
    }

    pub fn unit_mut(&mut self) -> &mut UnitComponent {
        match &mut self.physical_type {
            PhysicalType::Unit(unit) => unit,
            PhysicalType::Structure { .. } => panic!("Not a unit"),
        }
    }
}

#[derive(Debug)]
pub struct HealthComponent {
    pub max: u32,
    pub current: u32,
}

impl HealthComponent {
    pub fn new(max_health: u32) -> Self {
        Self {
            max: max_health,
            current: max_health,
        }
    }

    pub fn receive_healing(&mut self, amount: u32) {
        self.current = min(self.current + amount, self.max);
    }

    pub fn receive_damage(&mut self, amount: u32) {
        // TODO if an entity took damage that brings it below 0 and was then healed up by a lower
        //      amount in the same game frame, it would not be marked as dead, which seems wrong.
        self.current = self.current.saturating_sub(amount);
    }
}

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
}

#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq)]
pub enum EntitySprite {
    SquareUnit,
    SmallBuilding,
    CircleUnit,
    LargeBuilding,
    Neutral,
}

#[derive(Debug)]
pub struct UnitComponent {
    pub sub_cell_movement: SubCellMovement,
    pub pathfinder: Pathfinder,
    pub combat: Option<Combat>,
}

impl UnitComponent {
    pub fn new(position: [u32; 2], movement_cooldown: Duration, combat: Option<Combat>) -> Self {
        Self {
            sub_cell_movement: SubCellMovement::new(position, movement_cooldown),
            pathfinder: Pathfinder::new(),
            combat,
        }
    }
}

#[derive(Debug)]
pub struct Pathfinder {
    movement_plan: Vec<[u32; 2]>,
}

impl Pathfinder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            movement_plan: Default::default(),
        }
    }

    pub fn find_path(&mut self, current_pos: &[u32; 2], destination: [u32; 2]) {
        let [mut x, mut y] = current_pos;
        let mut plan = Vec::new();
        while [x, y] != destination {
            match destination[0].cmp(&x) {
                Ordering::Less => x -= 1,
                Ordering::Greater => x += 1,
                Ordering::Equal => {}
            };
            match destination[1].cmp(&y) {
                Ordering::Less => y -= 1,
                Ordering::Greater => y += 1,
                Ordering::Equal => {}
            };
            plan.push([x, y]);
        }
        plan.reverse();
        self.movement_plan = plan;
    }

    pub fn peek_path(&self) -> Option<&[u32; 2]> {
        self.movement_plan.last()
    }

    pub fn advance_path(&mut self) -> [u32; 2] {
        self.movement_plan.pop().expect("Can't advance empty path")
    }

    pub fn clear(&mut self) {
        self.movement_plan.clear();
    }
}

#[derive(Debug)]
pub struct SubCellMovement {
    previous_position: [u32; 2],
    remaining: Duration,
    straight_movement_cooldown: Duration,
    diagonal_movement_cooldown: Duration,
}

impl SubCellMovement {
    pub fn new(position: [u32; 2], movement_cooldown: Duration) -> Self {
        Self {
            previous_position: position,
            remaining: Duration::ZERO,
            straight_movement_cooldown: movement_cooldown,
            diagonal_movement_cooldown: movement_cooldown.mul_f32(2_f32.sqrt()),
        }
    }

    pub fn update(&mut self, dt: Duration, position: [u32; 2]) {
        self.remaining = self.remaining.checked_sub(dt).unwrap_or(Duration::ZERO);
        if self.remaining.is_zero() {
            self.previous_position = position;
        }
    }

    pub fn pixel_position(&self, position: [u32; 2]) -> [f32; 2] {
        let prev_pos = game::grid_to_pixel_position(self.previous_position);
        let pos = game::grid_to_pixel_position(position);
        let progress = match SubCellMovement::direction(self.previous_position, position) {
            MovementDirection::Straight => {
                self.remaining.as_secs_f32() / self.straight_movement_cooldown.as_secs_f32()
            }
            MovementDirection::Diagonal => {
                self.remaining.as_secs_f32() / self.diagonal_movement_cooldown.as_secs_f32()
            }
            MovementDirection::None => 0.0,
        };

        [
            pos[0] - progress * (pos[0] - prev_pos[0]),
            pos[1] - progress * (pos[1] - prev_pos[1]),
        ]
    }

    pub fn is_ready(&self) -> bool {
        self.remaining.is_zero()
    }

    pub fn set_moving(&mut self, old_position: [u32; 2], new_position: [u32; 2]) {
        assert!(self.remaining.is_zero());
        match SubCellMovement::direction(old_position, new_position) {
            MovementDirection::Straight => self.remaining = self.straight_movement_cooldown,
            MovementDirection::Diagonal => self.remaining = self.diagonal_movement_cooldown,
            MovementDirection::None => {}
        }
    }

    fn direction(from: [u32; 2], to: [u32; 2]) -> MovementDirection {
        let dx = (from[0] as i32 - to[0] as i32).abs();
        let dy = (from[1] as i32 - to[1] as i32).abs();
        match (dx, dy) {
            (0, 0) => MovementDirection::None,
            (1, 1) => MovementDirection::Diagonal,
            _ => MovementDirection::Straight,
        }
    }
}

enum MovementDirection {
    Straight,
    Diagonal,
    None,
}

#[derive(Debug)]
pub struct TrainingComponent {
    ongoing: Option<OngoingTraining>,
    options: HashMap<EntityType, TrainingConfig>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TrainingConfig {
    pub duration: Duration,
    pub cost: u32,
}

#[derive(Debug)]
struct OngoingTraining {
    remaining: Duration,
    entity_type: EntityType,
}

impl TrainingComponent {
    fn new(options: HashMap<EntityType, TrainingConfig>) -> Self {
        Self {
            ongoing: None,
            options,
        }
    }

    #[must_use]
    pub fn start(&mut self, trained_entity_type: EntityType) -> TrainingPerformStatus {
        if self.ongoing.is_some() {
            TrainingPerformStatus::AlreadyOngoing
        } else {
            self.ongoing = Some(OngoingTraining {
                remaining: self.options.get(&trained_entity_type).unwrap().duration,
                entity_type: trained_entity_type,
            });
            TrainingPerformStatus::NewTrainingStarted
        }
    }

    pub fn update(&mut self, dt: Duration) -> TrainingUpdateStatus {
        match self.ongoing.take() {
            Some(mut ongoing) => {
                ongoing.remaining = ongoing.remaining.checked_sub(dt).unwrap_or(Duration::ZERO);
                if ongoing.remaining.is_zero() {
                    println!("Training done!");
                    TrainingUpdateStatus::Done(ongoing.entity_type)
                } else {
                    self.ongoing = Some(ongoing);
                    TrainingUpdateStatus::Ongoing
                }
            }
            None => TrainingUpdateStatus::NothingOngoing,
        }
    }

    pub fn progress(&self) -> Option<f32> {
        self.ongoing.as_ref().map(|ongoing_training| {
            let total = self
                .options
                .get(&ongoing_training.entity_type)
                .unwrap()
                .duration;
            1.0 - ongoing_training.remaining.as_secs_f32() / total.as_secs_f32()
        })
    }

    pub fn options(&self) -> impl Iterator<Item = (&EntityType, &TrainingConfig)> {
        self.options.iter()
    }
}

#[derive(PartialEq)]
pub enum TrainingUpdateStatus {
    NothingOngoing,
    Ongoing,
    Done(EntityType),
}

#[derive(PartialEq)]
pub enum TrainingPerformStatus {
    NewTrainingStarted,
    AlreadyOngoing,
}

#[derive(Debug)]
pub struct Combat {
    pub target_entity_id: Option<EntityId>,
    cooldown: Duration,
}

impl Combat {
    fn new() -> Self {
        Self {
            target_entity_id: None,
            cooldown: Duration::ZERO,
        }
    }

    pub fn count_down_cooldown(&mut self, dt: Duration) -> bool {
        self.cooldown = self.cooldown.checked_sub(dt).unwrap_or(Duration::ZERO);
        self.cooldown.is_zero()
    }

    pub fn start_cooldown(&mut self) {
        self.cooldown = Duration::from_secs(3);
    }
}

#[derive(Debug)]
pub struct HealingActionComponent;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Action {
    Train(EntityType, TrainingConfig),
    Move,
    Heal,
    Attack,
}
