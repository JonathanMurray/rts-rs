use ggez::input::mouse::{self, CursorIcon};
use ggez::Context;

use std::cell::{Cell, RefCell};
use std::time::Duration;

use crate::camera::Camera;
use crate::data::EntityType;
use crate::entities::EntityId;
use crate::game::WORLD_VIEWPORT;

#[derive(PartialEq, Copy, Clone)]
pub enum CursorState {
    Default,
    SelectingAttackTarget,
    SelectingMovementDestination,
    PlacingStructure(EntityType),
    SelectingResourceTarget,
    DraggingSelectionArea([f32; 2]),
}

pub struct MovementCommandIndicator {
    world_pixel_position: [f32; 2],
    remaining: Duration,
}

impl MovementCommandIndicator {
    fn new() -> Self {
        Self {
            world_pixel_position: Default::default(),
            remaining: Default::default(),
        }
    }

    fn update(&mut self, dt: Duration) {
        self.remaining = self.remaining.checked_sub(dt).unwrap_or(Duration::ZERO);
    }

    pub fn set(&mut self, world_pixel_position: [f32; 2]) {
        self.world_pixel_position = world_pixel_position;
        self.remaining = Duration::from_secs_f32(0.5);
    }

    pub fn graphics(&self) -> Option<([f32; 2], f32)> {
        if !self.remaining.is_zero() {
            let scale = self.remaining.as_secs_f32() / 0.5;
            return Some((self.world_pixel_position, scale));
        }
        None
    }
}

#[derive(Copy, Clone)]
pub enum HighlightType {
    Hostile,
    Friendly,
}

pub struct EntityHighlight {
    pub entity_id: EntityId,
    remaining: Duration,
    pub highlight_type: HighlightType,
}

impl EntityHighlight {
    pub fn new(entity_id: EntityId, highlight_type: HighlightType) -> Self {
        Self {
            entity_id,
            remaining: Duration::from_millis(800),
            highlight_type,
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.remaining = self.remaining.saturating_sub(dt);
    }

    pub fn is_visible(&self) -> bool {
        let blink_ms = 200;
        (self.remaining.as_millis() / blink_ms) % 2 == 0
    }
}

pub struct PlayerState {
    pub selected_entity_ids: Vec<EntityId>,
    pub cursor_state: Cell<CursorState>,
    pub camera: RefCell<Camera>,
    pub movement_command_indicator: RefCell<MovementCommandIndicator>,
    pub entity_highlights: RefCell<Vec<EntityHighlight>>,
}

impl PlayerState {
    pub fn new(camera: Camera) -> Self {
        Self {
            selected_entity_ids: vec![],
            cursor_state: Cell::new(CursorState::Default),
            camera: RefCell::new(camera),
            movement_command_indicator: RefCell::new(MovementCommandIndicator::new()),
            entity_highlights: RefCell::new(vec![]),
        }
    }

    pub fn set_cursor_state(&self, ctx: &mut Context, state: CursorState) {
        match state {
            CursorState::Default => mouse::set_cursor_type(ctx, CursorIcon::Default),
            CursorState::SelectingAttackTarget => {
                mouse::set_cursor_type(ctx, CursorIcon::Crosshair)
            }
            CursorState::SelectingMovementDestination => {
                mouse::set_cursor_type(ctx, CursorIcon::Move)
            }
            CursorState::PlacingStructure(..) => mouse::set_cursor_type(ctx, CursorIcon::Grabbing),
            CursorState::SelectingResourceTarget => mouse::set_cursor_type(ctx, CursorIcon::Grab),
            CursorState::DraggingSelectionArea(..) => {
                mouse::set_cursor_type(ctx, CursorIcon::Default)
            }
        }
        self.cursor_state.set(state);
    }

    pub fn cursor_state(&self) -> CursorState {
        self.cursor_state.get()
    }

    pub fn screen_to_world(&self, coordinates: [f32; 2]) -> Option<[f32; 2]> {
        let [x, y] = coordinates;
        if !WORLD_VIEWPORT.contains(coordinates) {
            return None;
        }

        let camera_pos = self.camera.borrow().position_in_world;
        Some([
            x - WORLD_VIEWPORT.x + camera_pos[0],
            y - WORLD_VIEWPORT.y + camera_pos[1],
        ])
    }

    pub fn world_to_screen(&self, world_pixel_position: [f32; 2]) -> [f32; 2] {
        let [x, y] = world_pixel_position;
        let camera_pos = self.camera.borrow().position_in_world;
        [
            WORLD_VIEWPORT.x + x - camera_pos[0],
            WORLD_VIEWPORT.y + y - camera_pos[1],
        ]
    }

    pub fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.camera.borrow_mut().update(ctx, dt);
        self.movement_command_indicator.borrow_mut().update(dt);
        let mut highlights = self.entity_highlights.borrow_mut();
        for highlight in highlights.iter_mut() {
            highlight.update(dt);
        }
        highlights.retain(|highlight| !highlight.remaining.is_zero());
    }

    pub fn camera_position_in_world(&self) -> [f32; 2] {
        self.camera.borrow().position_in_world
    }
}
