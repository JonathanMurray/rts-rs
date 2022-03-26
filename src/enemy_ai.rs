use rand::rngs::ThreadRng;
use rand::Rng;
use std::time::Duration;

use crate::entities::{Entity, PhysicalType, Team, TrainingPerformStatus};
use crate::game::TeamState;

pub struct EnemyPlayerAi {
    timer_s: f32,
    map_dimensions: [u32; 2],
}

impl EnemyPlayerAi {
    pub fn new(map_dimensions: [u32; 2]) -> Self {
        Self {
            timer_s: 0.0,
            map_dimensions,
        }
    }

    pub fn run(
        &mut self,
        dt: Duration,
        entities: &mut [Entity],
        rng: &mut ThreadRng,
        team_state: &mut TeamState,
    ) {
        self.timer_s -= dt.as_secs_f32();

        // TODO Instead of mutating game state, return commands
        if self.timer_s <= 0.0 {
            self.timer_s = 2.0;
            for entity in entities {
                if entity.team == Team::Enemy && rng.gen_bool(0.7) {
                    let x: u32 = rng.gen_range(0..self.map_dimensions[0]);
                    let y: u32 = rng.gen_range(0..self.map_dimensions[1]);
                    match &mut entity.physical_type {
                        PhysicalType::Mobile(movement) => {
                            movement.pathfinder.find_path(&entity.position, [x, y]);
                        }
                        PhysicalType::Structure { .. } => {
                            if let Some(training_action) = &mut entity.training_action {
                                let cost = training_action.cost();
                                if team_state.resources >= cost {
                                    if let TrainingPerformStatus::NewTrainingStarted =
                                        training_action.perform()
                                    {
                                        team_state.resources -= cost;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
