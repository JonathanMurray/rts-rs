use rand::rngs::ThreadRng;
use rand::Rng;
use std::time::Duration;

use crate::entities::{Action, Entity, PhysicalType, Team};
use crate::game::Command;

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

    pub fn run(&mut self, dt: Duration, entities: &[Entity], rng: &mut ThreadRng) -> Vec<Command> {
        let mut commands = vec![];
        self.timer_s -= dt.as_secs_f32();

        if self.timer_s <= 0.0 {
            self.timer_s = 2.0;
            for entity in entities {
                if entity.team == Team::Enemy && rng.gen_bool(0.5) {
                    let command = match &entity.physical_type {
                        PhysicalType::Mobile(..) => {
                            let x: u32 = rng.gen_range(0..self.map_dimensions[0]);
                            let y: u32 = rng.gen_range(0..self.map_dimensions[1]);
                            Some(Command::Move(entity.id, [x, y]))
                        }
                        PhysicalType::Structure { .. } => {
                            entity.training.as_ref().map(|training| {
                                let (&entity_type, &config) = training.options().next().unwrap();
                                Command::Train(entity.id, entity_type, config)
                            })
                        }
                    };
                    for action in entity.actions.iter().flatten() {
                        if action == &Action::Harm && rng.gen_bool(0.8) {
                            if let Some(player_entity) =
                                entities.iter().find(|e| e.team == Team::Player)
                            {
                                commands.push(Command::DealDamage(entity.id, player_entity.id));
                            }
                        }
                    }
                    if let Some(command) = command {
                        commands.push(command);
                    }
                }
            }
        }
        commands
    }
}
