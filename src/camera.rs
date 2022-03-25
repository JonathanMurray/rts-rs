use std::time::Duration;

use ggez::input::keyboard::KeyCode;
use ggez::Context;

pub struct Camera {
    position_in_world: [f32; 2],
    max_position: [f32; 2],
}

impl Camera {
    pub fn new(position_in_world: [f32; 2], max_position: [f32; 2]) -> Self {
        Self {
            position_in_world,
            max_position,
        }
    }

    pub fn position_in_world(&self) -> [f32; 2] {
        self.position_in_world
    }

    pub fn update(&mut self, ctx: &Context, dt: Duration) {
        const PAN_SPEED: f32 = 700.0;
        let [mut x, mut y] = self.position_in_world;
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Left) {
            x -= PAN_SPEED * dt.as_secs_f32();
        }
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Right) {
            x += PAN_SPEED * dt.as_secs_f32();
        }
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Up) {
            y -= PAN_SPEED * dt.as_secs_f32();
        }
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Down) {
            y += PAN_SPEED * dt.as_secs_f32();
        }

        x = x.min(self.max_position[0]).max(0.0);
        y = y.min(self.max_position[1]).max(0.0);
        self.position_in_world = [x, y];
    }
}
