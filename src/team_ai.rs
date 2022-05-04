use rand::rngs::ThreadRng;
use rand::Rng;
use std::cell::{Ref, RefCell};
use std::time::Duration;

use crate::core::{
    AttackCommand, Command, ConstructCommand, Core, GatherResourceCommand, StartActivityCommand,
};
use crate::data::EntityType;
use crate::entities::{ActivityTarget, EntityState, Team};

use std::cmp;

pub struct TeamAi {
    team: Team,
    opponent: Team,
    timer_s: f32,
}

impl TeamAi {
    pub fn new(team: Team, opponent: Team) -> Self {
        Self {
            team,
            opponent,
            timer_s: 0.0,
        }
    }

    pub fn team(&self) -> Team {
        self.team
    }

    pub fn run<'a>(
        &mut self,
        dt: Duration,
        core: &'a Core,
        rng: &mut ThreadRng,
    ) -> Option<Command<'a>> {
        self.timer_s -= dt.as_secs_f32();
        if self.timer_s <= 0.0 {
            self.timer_s = 1.0;
            self.act(core, rng)
        } else {
            None
        }
    }

    fn act<'a>(&mut self, core: &'a Core, rng: &mut ThreadRng) -> Option<Command<'a>> {
        let entities = core.entities();

        let mut idle_workers = vec![];
        let mut idle_bases = vec![];
        let mut idle_military_buildings = vec![];
        let mut idle_fighters = vec![];
        let mut has_base = false;
        let mut military_building_count = 0;
        let mut worker_count = 0;

        for (_id, entity) in entities {
            let entity_ref = entity.borrow();
            if entity_ref.team == self.team {
                match (entity_ref.entity_type, entity_ref.state) {
                    (EntityType::Engineer, state) => {
                        worker_count += 1;
                        if state == EntityState::Idle {
                            idle_workers.push(entity);
                        }
                    }
                    (EntityType::Enforcer, EntityState::Idle) => {
                        idle_fighters.push(entity);
                    }
                    (EntityType::TechLab, state) => {
                        has_base = true;
                        if state == EntityState::Idle {
                            idle_bases.push(entity);
                        }
                    }
                    (EntityType::BattleAcademy, state) => {
                        military_building_count += 1;
                        if state == EntityState::Idle {
                            idle_military_buildings.push(entity);
                        }
                    }
                    _ => {}
                }
            }
        }

        if !has_base {
            if let Some(worker) = idle_workers.pop() {
                let worker = worker.borrow_mut();
                let structure_size = core.structure_size(&EntityType::TechLab);
                if let Some(pos) =
                    find_free_position_for_structure(core, worker.position, *structure_size, rng)
                {
                    return Some(Command::Construct(ConstructCommand {
                        builder: worker,
                        structure_position: pos,
                        structure_type: EntityType::TechLab,
                    }));
                }
            }
        }

        if military_building_count < 2 {
            if let Some(worker) = idle_workers.pop() {
                let worker = worker.borrow_mut();
                let structure_size = core.structure_size(&EntityType::BattleAcademy);
                if let Some(pos) =
                    find_free_position_for_structure(core, worker.position, *structure_size, rng)
                {
                    return Some(Command::Construct(ConstructCommand {
                        builder: worker,
                        structure_position: pos,
                        structure_type: EntityType::BattleAcademy,
                    }));
                }
            }
        }

        if !idle_workers.is_empty() {
            if let Some(resource) =
                entities
                    .iter()
                    .find_map(|(_id, e)| match RefCell::try_borrow(e) {
                        Ok(e) if e.entity_type == EntityType::FuelRift => Some(e),
                        _ => None,
                    })
            {
                if let Some(worker) = idle_workers.pop() {
                    return Some(Command::GatherResource(GatherResourceCommand {
                        gatherer: worker.borrow_mut(),
                        resource: Ref::clone(&resource),
                    }));
                }
            }
        }

        if worker_count < 3 {
            if let Some(base) = idle_bases.into_iter().next() {
                return Some(Command::StartActivity(StartActivityCommand {
                    structure: base.borrow_mut(),
                    target: ActivityTarget::Train(EntityType::Engineer),
                }));
            }
        }

        if let Some(military_building) = idle_military_buildings.into_iter().next() {
            return Some(Command::StartActivity(StartActivityCommand {
                structure: military_building.borrow_mut(),
                target: ActivityTarget::Train(EntityType::Enforcer),
            }));
        }

        if !idle_fighters.is_empty() {
            let mut victims = vec![];
            for (_id, entity) in entities {
                if let Ok(entity) = entity.try_borrow() {
                    if entity.team == self.opponent {
                        victims.push(entity);
                        if victims.len() == idle_fighters.len() {
                            // Have enough victims, one for each attacker
                            break;
                        }
                    }
                }
            }

            for fighter in idle_fighters {
                if let Some(victim) = victims.pop() {
                    return Some(Command::Attack(AttackCommand {
                        attacker: fighter.borrow_mut(),
                        victim,
                    }));
                }
            }
        }

        None
    }
}

fn find_free_position_for_structure(
    core: &Core,
    worker_position: [u32; 2],
    structure_size: [u32; 2],
    rng: &mut ThreadRng,
) -> Option<[u32; 2]> {
    let mut x = worker_position[0] as i32;
    let mut y = worker_position[1] as i32;

    // randomize the structure placement a bit to make AI less deterministic
    x = rng.gen_range(cmp::max(0, x - 2)..=x + 2);
    y = rng.gen_range(cmp::max(0, y - 2)..=y + 2);

    // Look for a free position by going in an outward spiral
    // starting from the worker position. This is quite
    // inefficient.

    let mut spiral_distance = 1;
    while spiral_distance < 15 {
        // move right
        for _ in 0..spiral_distance {
            if x >= 0
                && y >= 0
                && core.can_structure_fit(worker_position, [x as u32, y as u32], structure_size)
            {
                return Some([x as u32, y as u32]);
            }
            x += 1;
        }
        // move up
        for _ in 0..spiral_distance {
            if x >= 0
                && y >= 0
                && core.can_structure_fit(worker_position, [x as u32, y as u32], structure_size)
            {
                return Some([x as u32, y as u32]);
            }
            y -= 1;
        }
        spiral_distance += 1;
        // move left
        for _ in 0..spiral_distance {
            if x >= 0
                && y >= 0
                && core.can_structure_fit(worker_position, [x as u32, y as u32], structure_size)
            {
                return Some([x as u32, y as u32]);
            }
            x -= 1;
        }
        // move down
        for _ in 0..spiral_distance {
            if x >= 0
                && y >= 0
                && core.can_structure_fit(worker_position, [x as u32, y as u32], structure_size)
            {
                return Some([x as u32, y as u32]);
            }
            y += 1;
        }
        spiral_distance += 1;
    }
    None
}
