use rand::rngs::ThreadRng;
use rand::Rng;
use std::time::Duration;

use crate::entities::{Entity, Team};

pub struct EnemyPlayerAi {
    timer_s: f32,
    map_dimensions: (u32, u32),
}

impl EnemyPlayerAi {
    pub fn new(map_dimensions: (u32, u32)) -> Self {
        Self {
            timer_s: 0.0,
            map_dimensions,
        }
    }

    pub fn run(&mut self, dt: Duration, entities: &mut [Entity], rng: &mut ThreadRng) {
        self.timer_s -= dt.as_secs_f32();

        // TODO Instead of mutating game state, return commands
        if self.timer_s <= 0.0 {
            self.timer_s = 2.0;
            for enemy in entities {
                if enemy.team == Team::Ai && rng.gen_bool(0.7) {
                    let x: u32 = rng.gen_range(0..self.map_dimensions.0);
                    let y: u32 = rng.gen_range(0..self.map_dimensions.1);
                    let current_pos = &enemy.physics.position();
                    enemy.pathfind.find_path(current_pos, [x, y]);
                }
            }
        }
    }
}
