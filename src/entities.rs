use std::time::Duration;

use crate::game;
use std::cmp::Ordering;

#[derive(Debug)]
pub struct Entity {
    pub movement_component: MovementComponent,
    pub team: Team,
    pub sprite: EntitySprite,
    pub movement_plan: Vec<[u32; 2]>,
}

impl Entity {
    pub fn new(movement_component: MovementComponent, team: Team, sprite: EntitySprite) -> Self {
        Self {
            movement_component,
            team,
            sprite,
            movement_plan: Default::default(),
        }
    }

    pub fn set_destination(&mut self, destination: [u32; 2]) {
        let [mut x, mut y] = self.movement_component.position();
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
}

#[derive(Debug, PartialEq)]
pub enum Team {
    Player,
    Ai,
}

#[derive(Debug)]
pub enum EntitySprite {
    Player,
    Enemy,
}

#[derive(Debug)]
pub struct MovementComponent {
    previous_position: [u32; 2],
    position: [u32; 2],
    pub movement_timer: Duration, //TODO
    straight_movement_cooldown: Duration,
    diagonal_movement_cooldown: Duration,
}

impl MovementComponent {
    pub fn new(position: [u32; 2], movement_cooldown: Duration) -> Self {
        Self {
            previous_position: position,
            position,
            movement_timer: Duration::ZERO,
            straight_movement_cooldown: movement_cooldown,
            diagonal_movement_cooldown: movement_cooldown.mul_f32(2_f32.sqrt()),
        }
    }

    pub fn update(&mut self, dt: Duration) {
        if self.movement_timer < dt {
            self.movement_timer = Duration::ZERO;
        } else {
            self.movement_timer -= dt;
        }
        if self.movement_timer.is_zero() {
            self.previous_position = self.position;
        }
    }

    pub fn screen_coords(&self) -> [f32; 2] {
        let prev_pos = game::grid_to_screen_coords(self.previous_position);
        let pos = game::grid_to_screen_coords(self.position);
        let interpolation =
            match MovementComponent::direction(self.previous_position, self.position) {
                MovementDirection::Straight => {
                    self.movement_timer.as_secs_f32()
                        / self.straight_movement_cooldown.as_secs_f32()
                }
                MovementDirection::Diagonal => {
                    self.movement_timer.as_secs_f32()
                        / self.diagonal_movement_cooldown.as_secs_f32()
                }
                MovementDirection::None => 0.0,
            };

        [
            pos[0] - interpolation * (pos[0] - prev_pos[0]),
            pos[1] - interpolation * (pos[1] - prev_pos[1]),
        ]
    }

    pub fn move_to(&mut self, new_position: [u32; 2]) {
        assert!(self.movement_timer.is_zero());
        match MovementComponent::direction(self.position, new_position) {
            MovementDirection::Straight => self.movement_timer = self.straight_movement_cooldown,
            MovementDirection::Diagonal => self.movement_timer = self.diagonal_movement_cooldown,
            MovementDirection::None => {}
        }
        self.position = new_position;
    }

    pub fn position(&self) -> [u32; 2] {
        self.position
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
