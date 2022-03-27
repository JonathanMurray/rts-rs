use rand::rngs::ThreadRng;
use rand::Rng;
use std::time::Duration;

use crate::entities::{Action, Entity, Team};
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
            self.timer_s = 5.0;
            for entity in entities {
                if entity.team == Team::Enemy && rng.gen_bool(0.5) {
                    for action in entity.actions.iter().flatten() {
                        if action == &Action::Attack && rng.gen_bool(0.8) {
                            if let Some(player_entity) =
                                entities.iter().find(|e| e.team == Team::Player)
                            {
                                commands.push(Command::Attack(entity.id, player_entity.id));
                                break;
                            }
                        }
                        if action == &Action::Move && rng.gen_bool(0.3) {
                            let x: u32 = rng.gen_range(0..self.map_dimensions[0]);
                            let y: u32 = rng.gen_range(0..self.map_dimensions[1]);
                            commands.push(Command::Move(entity.id, [x, y]));
                            break;
                        }
                        if let &Action::Train(entity_type, config) = action {
                            commands.push(Command::Train(entity.id, entity_type, config));
                            break;
                        }
                    }
                }
            }
        }
        commands
    }
}
