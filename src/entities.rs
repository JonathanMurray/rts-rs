use std::cmp::min;
use std::collections::HashMap;
use std::sync::atomic::{self, AtomicUsize};
use std::time::Duration;

use ggez::graphics::Rect;

use crate::data::EntityType;
use crate::game::{self, CELL_PIXEL_SIZE};
use crate::grid::CellRect;

static NEXT_ENTITY_ID: AtomicUsize = AtomicUsize::new(1);

pub const NUM_ENTITY_ACTIONS: usize = 6;

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct EntityId(usize);

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum EntityState {
    Idle,
    TrainingUnit(EntityType),
    MovingToConstruction(EntityType, [u32; 2]),
    Moving,
    MovingToAttackTarget(EntityId),
    Attacking(EntityId),
    MovingToResource(EntityId),
    GatheringResource(EntityId),
    ReturningResource(EntityId),
    UnderConstruction(Duration, Duration),
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

#[derive(Debug)]
pub struct Entity {
    pub entity_type: EntityType,
    pub id: EntityId,
    pub position: [u32; 2],
    pub category: EntityCategory,
    pub team: Team,
    pub animation: AnimationState,
    pub health: Option<HealthComponent>,
    pub training: Option<TrainingComponent>,
    pub actions: [Option<Action>; NUM_ENTITY_ACTIONS],
    pub state: EntityState,
}

#[derive(Debug)]
pub struct AnimationState {
    pub ms_counter: u16,
}

#[derive(Debug)]
pub enum EntityCategory {
    Unit(UnitComponent),
    Structure { size: [u32; 2] },
    Resource { remaining: u32 },
}

pub struct EntityConfig {
    pub max_health: Option<u32>,
    pub category: CategoryConfig,
    pub actions: [Option<ActionConfig>; NUM_ENTITY_ACTIONS],
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ActionConfig {
    Train(EntityType, TrainingConfig),
    Construct(EntityType, ConstructionConfig),
    Stop,
    Move(Duration),
    Attack(u32),
    GatherResource,
    ReturnResource,
}

pub enum CategoryConfig {
    Unit,
    StructureSize([u32; 2]),
    ResourceCapacity(u32),
}

impl Entity {
    pub fn new(
        entity_type: EntityType,
        config: EntityConfig,
        position: [u32; 2],
        team: Team,
    ) -> Self {
        // Make sure all entities have unique IDs
        let id = EntityId(NEXT_ENTITY_ID.fetch_add(1, atomic::Ordering::Relaxed));

        let health = config.max_health.map(HealthComponent::new);
        let mut training_options: HashMap<EntityType, TrainingConfig> = Default::default();
        let mut construction_options: HashMap<EntityType, ConstructionConfig> = Default::default();
        let mut attack_damage = None;
        let mut can_gather = false;
        let mut movement_cooldown = None;
        let mut actions = [None; NUM_ENTITY_ACTIONS];
        for (i, action) in config.actions.into_iter().enumerate() {
            actions[i] = action.map(|action| match action {
                ActionConfig::Train(unit_type, config) => {
                    training_options.insert(unit_type, config);
                    Action::Train(unit_type, config)
                }
                ActionConfig::Construct(structure_type, config) => {
                    construction_options.insert(structure_type, config);
                    Action::Construct(structure_type, config)
                }
                ActionConfig::Attack(damage) => {
                    attack_damage = Some(damage);
                    Action::Attack
                }
                ActionConfig::GatherResource => {
                    can_gather = true;
                    Action::GatherResource
                }
                ActionConfig::Move(cooldown) => {
                    movement_cooldown = Some(cooldown);
                    Action::Move
                }
                ActionConfig::Stop => Action::Stop,
                ActionConfig::ReturnResource => Action::ReturnResource,
            });
        }
        let training =
            (!training_options.is_empty()).then(|| TrainingComponent::new(training_options));
        let construction_options = (!construction_options.is_empty()).then(|| construction_options);
        let category = match config.category {
            CategoryConfig::Unit => {
                let combat = attack_damage.map(Combat::new);
                let gathering = can_gather.then(Gathering::new);
                let cooldown = movement_cooldown.expect("Unit must have movement");
                EntityCategory::Unit(UnitComponent::new(
                    position,
                    cooldown,
                    combat,
                    gathering,
                    construction_options,
                ))
            }
            CategoryConfig::StructureSize(size) => EntityCategory::Structure { size },
            CategoryConfig::ResourceCapacity(capacity) => EntityCategory::Resource {
                remaining: capacity,
            },
        };
        let animation = AnimationState { ms_counter: 0 };

        Self {
            entity_type,
            id,
            position,
            category,
            team,
            animation,
            health,
            training,
            actions,
            state: EntityState::Idle,
        }
    }

    pub fn size(&self) -> [u32; 2] {
        match self.category {
            EntityCategory::Structure { size } => size,
            EntityCategory::Unit(..) | EntityCategory::Resource { .. } => [1, 1],
        }
    }

    pub fn world_pixel_position(&self) -> [f32; 2] {
        match &self.category {
            EntityCategory::Unit(unit) => unit.sub_cell_movement.pixel_position(self.position),
            EntityCategory::Structure { .. } | EntityCategory::Resource { .. } => {
                game::grid_to_world(self.position)
            }
        }
    }

    pub fn cell_rect(&self) -> CellRect {
        CellRect {
            position: self.position,
            size: self.size(),
        }
    }

    pub fn pixel_rect(&self) -> Rect {
        let [pixel_x, pixel_y] = self.world_pixel_position();
        let [grid_w, grid_h] = self.size();
        Rect {
            x: pixel_x,
            y: pixel_y,
            w: (grid_w as f32) * CELL_PIXEL_SIZE[0],
            h: (grid_h as f32) * CELL_PIXEL_SIZE[1],
        }
    }

    pub fn resource_remaining(&self) -> &u32 {
        match &self.category {
            EntityCategory::Resource { remaining } => remaining,
            _ => panic!("Not a resource"),
        }
    }

    pub fn resource_remaining_mut(&mut self) -> &mut u32 {
        match &mut self.category {
            EntityCategory::Resource { remaining } => remaining,
            _ => panic!("Not a resource"),
        }
    }

    pub fn unit(&self) -> &UnitComponent {
        match &self.category {
            EntityCategory::Unit(unit) => unit,
            _ => panic!("Not a unit"),
        }
    }

    pub fn unit_mut(&mut self) -> &mut UnitComponent {
        match &mut self.category {
            EntityCategory::Unit(unit) => unit,
            _ => panic!("Not a unit"),
        }
    }

    pub fn direction(&self) -> Direction {
        match &self.category {
            EntityCategory::Unit(unit) => unit.direction,
            EntityCategory::Structure { .. } | EntityCategory::Resource { .. } => Direction::South,
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
    Enemy1,
    Enemy2,
    Neutral,
}

#[derive(Debug)]
pub struct UnitComponent {
    pub sub_cell_movement: SubCellMovement,
    pub movement_plan: MovementPlan,
    pub direction: Direction,
    pub combat: Option<Combat>,
    pub gathering: Option<Gathering>,
    pub construction_options: Option<HashMap<EntityType, ConstructionConfig>>,
}

impl UnitComponent {
    pub fn new(
        position: [u32; 2],
        movement_cooldown: Duration,
        combat: Option<Combat>,
        gathering: Option<Gathering>,
        construction_options: Option<HashMap<EntityType, ConstructionConfig>>,
    ) -> Self {
        Self {
            sub_cell_movement: SubCellMovement::new(position, movement_cooldown),
            movement_plan: MovementPlan::new(),
            direction: Direction::South,
            combat,
            gathering,
            construction_options,
        }
    }

    pub fn move_to_adjacent_cell(&mut self, old_position: [u32; 2], new_position: [u32; 2]) {
        let dx = new_position[0] as i32 - old_position[0] as i32;
        let dy = new_position[1] as i32 - old_position[1] as i32;
        self.direction = match (dx, dy) {
            (0, -1) => Direction::North,
            (1, -1) => Direction::NorthEast,
            (1, 0) => Direction::East,
            (1, 1) => Direction::SouthEast,
            (0, 1) => Direction::South,
            (-1, 1) => Direction::SouthWest,
            (-1, 0) => Direction::West,
            (-1, -1) => Direction::NorthWest,
            _ => panic!("Invalid movement: {:?} -> {:?}", old_position, new_position),
        };
        self.sub_cell_movement
            .set_moving(old_position, new_position);
    }
}

#[derive(Debug)]
pub struct MovementPlan {
    cell_positions: Vec<[u32; 2]>,
    blocked_counter: u32,
}

impl MovementPlan {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            cell_positions: Default::default(),
            blocked_counter: 0,
        }
    }

    pub fn set(&mut self, movement_plan: Vec<[u32; 2]>) {
        self.blocked_counter = 0;
        self.cell_positions = movement_plan;
    }

    pub fn peek(&self) -> Option<[u32; 2]> {
        self.cell_positions.last().copied()
    }

    pub fn advance(&mut self) -> [u32; 2] {
        self.blocked_counter = 0;
        self.cell_positions.pop().expect("Can't advance empty path")
    }

    pub fn destination(&self) -> [u32; 2] {
        self.cell_positions[0]
    }

    pub fn clear(&mut self) {
        self.blocked_counter = 0;
        self.cell_positions.clear();
    }

    pub fn on_movement_blocked(&mut self) -> bool {
        const MAX_ALLOWED_BLOCKS: u32 = 10;
        self.blocked_counter += 1;
        self.blocked_counter > MAX_ALLOWED_BLOCKS
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
        self.remaining = self.remaining.saturating_sub(dt);
        if self.remaining.is_zero() {
            self.previous_position = position;
        }
    }

    fn pixel_position(&self, position: [u32; 2]) -> [f32; 2] {
        let prev_pos = game::grid_to_world(self.previous_position);
        let pos = game::grid_to_world(position);
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

    pub fn is_between_cells(&self) -> bool {
        !self.remaining.is_zero()
    }

    fn set_moving(&mut self, old_position: [u32; 2], new_position: [u32; 2]) {
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
}

impl TrainingComponent {
    fn new(options: HashMap<EntityType, TrainingConfig>) -> Self {
        Self {
            ongoing: None,
            options,
        }
    }

    #[must_use]
    pub fn try_start(&mut self, trained_entity_type: EntityType) -> TrainingPerformStatus {
        if self.ongoing.is_some() {
            TrainingPerformStatus::AlreadyOngoing
        } else {
            self.ongoing = Some(OngoingTraining {
                remaining: self.options.get(&trained_entity_type).unwrap().duration,
            });
            TrainingPerformStatus::NewTrainingStarted
        }
    }

    pub fn update(&mut self, dt: Duration) -> TrainingUpdateStatus {
        match self.ongoing.take() {
            Some(mut ongoing) => {
                ongoing.remaining = ongoing.remaining.saturating_sub(dt);
                if ongoing.remaining.is_zero() {
                    println!("Training done!");
                    TrainingUpdateStatus::Done
                } else {
                    self.ongoing = Some(ongoing);
                    TrainingUpdateStatus::Ongoing
                }
            }
            None => TrainingUpdateStatus::NothingOngoing,
        }
    }

    pub fn progress(&self, trained_entity_type: EntityType) -> Option<f32> {
        self.ongoing.as_ref().map(|ongoing_training| {
            let total = self.options.get(&trained_entity_type).unwrap().duration;
            1.0 - ongoing_training.remaining.as_secs_f32() / total.as_secs_f32()
        })
    }

    pub fn config(&self, entity_type: &EntityType) -> &TrainingConfig {
        self.options.get(entity_type).unwrap_or_else(|| {
            panic!(
                "No config found for {:?}. Available options: {:?}",
                entity_type, self.options
            )
        })
    }
}

#[derive(PartialEq)]
pub enum TrainingUpdateStatus {
    NothingOngoing,
    Ongoing,
    Done,
}

#[derive(PartialEq)]
pub enum TrainingPerformStatus {
    NewTrainingStarted,
    AlreadyOngoing,
}

#[derive(Debug)]
pub struct Combat {
    cooldown: Duration,
    damage: u32,
}

impl Combat {
    fn new(damage: u32) -> Self {
        Self {
            cooldown: Duration::ZERO,
            damage,
        }
    }

    pub fn count_down_cooldown(&mut self, dt: Duration) {
        self.cooldown = self.cooldown.saturating_sub(dt);
    }

    pub fn is_attack_ready(&self) -> bool {
        self.cooldown.is_zero()
    }

    pub fn start_cooldown(&mut self) {
        // note: might be good to keep this in sync with attack animation
        self.cooldown = Duration::from_millis(1000);
    }

    pub fn damage_amount(&self) -> u32 {
        self.damage
    }
}

#[derive(Debug)]
pub struct Gathering {
    held_resource: Option<EntityId>,
    countdown: Duration,
}

impl Gathering {
    fn new() -> Self {
        Self {
            held_resource: None,
            countdown: Duration::ZERO,
        }
    }

    pub fn start_gathering(&mut self) {
        self.countdown = Duration::from_secs_f32(1.5);
    }

    pub fn make_progress_on_gathering(
        &mut self,
        dt: Duration,
        resource_id: EntityId,
    ) -> GatheringProgress {
        self.countdown = self.countdown.saturating_sub(dt);
        if self.countdown.is_zero() {
            assert!(
                self.held_resource.is_none(),
                "Can only hold one resource at a time"
            );
            self.held_resource = Some(resource_id);
            GatheringProgress::Done
        } else {
            GatheringProgress::InProgress
        }
    }

    pub fn is_carrying(&self) -> bool {
        self.held_resource.is_some()
    }

    pub fn drop_resource(&mut self) -> EntityId {
        self.held_resource
            .take()
            .expect("Can't drop a resource that's not being held")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GatheringProgress {
    Done,
    InProgress,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ConstructionConfig {
    pub construction_time: Duration,
    pub cost: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Action {
    Train(EntityType, TrainingConfig),
    Construct(EntityType, ConstructionConfig),
    Stop,
    Move,
    Attack,
    GatherResource,
    ReturnResource,
}
