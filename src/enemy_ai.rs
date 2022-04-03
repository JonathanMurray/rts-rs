use rand::rngs::ThreadRng;
use rand::Rng;
use std::cell::RefCell;
use std::time::Duration;

use crate::core::{AttackCommand, Command, MoveCommand, TrainCommand};
use crate::entities::{Action, Entity, EntityId, Team};

pub struct EnemyPlayerAi {
    timer_s: f32,
    world_dimensions: [u32; 2],
}

impl EnemyPlayerAi {
    pub fn new(world_dimensions: [u32; 2]) -> Self {
        Self {
            timer_s: 0.0,
            world_dimensions,
        }
    }

    pub fn run<'a>(
        &mut self,
        dt: Duration,
        entities: &'a [(EntityId, RefCell<Entity>)],
        rng: &mut ThreadRng,
    ) -> Vec<Command<'a>> {
        let mut commands = vec![];
        self.timer_s -= dt.as_secs_f32();

        if self.timer_s <= 0.0 {
            self.timer_s = 1.0;
            for (_id, enemy_entity) in entities {
                let enemy_entity = match RefCell::try_borrow_mut(enemy_entity) {
                    Ok(e) if e.team == Team::Enemy => Some(e),
                    _ => None,
                };

                if let Some(enemy_entity) = enemy_entity {
                    if rng.gen_bool(0.2) {
                        for action in enemy_entity.actions.iter().flatten() {
                            if action == &Action::Attack && rng.gen_bool(0.8) {
                                if let Some(player_entity) = entities.iter().find_map(|(_id, e)| {
                                    match RefCell::try_borrow(e) {
                                        Ok(e) if e.team == Team::Player => Some(e),
                                        _ => None,
                                    }
                                }) {
                                    commands.push(Command::Attack(AttackCommand {
                                        attacker: enemy_entity,
                                        victim: player_entity,
                                    }));
                                    break;
                                }
                            }
                            if action == &Action::Move && rng.gen_bool(0.3) {
                                let x: u32 = rng.gen_range(0..self.world_dimensions[0]);
                                let y: u32 = rng.gen_range(0..self.world_dimensions[1]);
                                commands.push(Command::Move(MoveCommand {
                                    unit: enemy_entity,
                                    destination: [x, y],
                                }));
                                break;
                            }
                            if let &Action::Train(trained_unit_type, config) = action {
                                commands.push(Command::Train(TrainCommand {
                                    trainer: enemy_entity,
                                    trained_unit_type,
                                    config,
                                }));
                                break;
                            }
                        }
                    }
                }
            }
        }
        commands
    }
}
