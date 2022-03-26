use std::cmp::Ordering;
use std::sync::atomic::{self, AtomicUsize};
use std::time::Duration;

use crate::data::EntityType;
use crate::game;

static NEXT_ENTITY_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct EntityId(usize);

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
    pub training_action: Option<TrainingActionComponent>,
}

#[derive(Debug)]
pub enum PhysicalType {
    Mobile(MovementComponent),
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
        training_action: Option<TrainingActionComponent>,
    ) -> Self {
        // Make sure all entities have unique IDs
        let id = EntityId(NEXT_ENTITY_ID.fetch_add(1, atomic::Ordering::Relaxed));
        let physical_type = match config.physical_type {
            PhysicalTypeConfig::MovementCooldown(cooldown) => {
                PhysicalType::Mobile(MovementComponent::new(position, cooldown))
            }
            PhysicalTypeConfig::StructureSize(size) => PhysicalType::Structure { size },
        };
        let health = config.max_health.map(HealthComponent::new);
        Self {
            id,
            name: config.name,
            position,
            is_solid: config.is_solid,
            physical_type,
            team,
            sprite: config.sprite,
            health,
            training_action,
        }
    }

    pub fn size(&self) -> [u32; 2] {
        match self.physical_type {
            PhysicalType::Mobile(_) => [1, 1],
            PhysicalType::Structure { size } => size,
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
}

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
}

#[derive(Debug)]
pub enum EntitySprite {
    PlayerUnit,
    PlayerBuilding,
    Enemy,
    EnemyBuilding,
    Neutral,
}

#[derive(Debug)]
pub struct MovementComponent {
    pub sub_cell_movement: SubCellMovement,
    pub pathfinder: Pathfinder,
}

impl MovementComponent {
    pub fn new(position: [u32; 2], movement_cooldown: Duration) -> Self {
        Self {
            sub_cell_movement: SubCellMovement::new(position, movement_cooldown),
            pathfinder: Pathfinder::new(),
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
pub struct TrainingActionComponent {
    remaining_duration: Option<Duration>,
    total_duration: Duration,
    trained_entity_type: EntityType,
}

impl TrainingActionComponent {
    #[allow(clippy::new_without_default)]
    pub fn new(trained_entity_type: EntityType) -> Self {
        Self {
            remaining_duration: None,
            total_duration: Duration::from_secs(3),
            trained_entity_type,
        }
    }

    pub fn cost(&self) -> u32 {
        1 // TODO dynamic costs
    }

    pub fn perform(&mut self) -> TrainingPerformStatus {
        if self.remaining_duration.is_some() {
            TrainingPerformStatus::AlreadyOngoing
        } else {
            self.remaining_duration = Some(self.total_duration);
            TrainingPerformStatus::NewTrainingStarted
        }
    }

    pub fn update(&mut self, dt: Duration) -> TrainingUpdateStatus {
        match self.remaining_duration.take() {
            Some(remaining) => {
                let remaining = remaining.checked_sub(dt).unwrap_or(Duration::ZERO);
                if remaining.is_zero() {
                    println!("Training done!");
                    TrainingUpdateStatus::Done(self.trained_entity_type)
                } else {
                    self.remaining_duration = Some(remaining);
                    TrainingUpdateStatus::Ongoing
                }
            }
            None => TrainingUpdateStatus::NothingOngoing,
        }
    }

    pub fn progress(&self) -> Option<f32> {
        self.remaining_duration
            .map(|remaining| 1.0 - remaining.as_secs_f32() / self.total_duration.as_secs_f32())
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
