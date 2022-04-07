use ggez::graphics::{Color, DrawMode, DrawParam, Mesh, MeshBuilder, Rect};
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use crate::game::{CELL_PIXEL_SIZE, WORLD_VIEWPORT};

pub struct Minimap {
    border_mesh: Mesh,
    camera_mesh: Mesh,
    camera_scale: [f32; 2],
    rect: Rect,
    is_mouse_dragging: bool,
    padding: f32,
}

impl Minimap {
    pub fn new(
        ctx: &mut Context,
        position: [f32; 2],
        width: f32,
        world_dimensions: [u32; 2],
    ) -> GameResult<Self> {
        let aspect_ratio = world_dimensions[0] as f32 / world_dimensions[1] as f32;
        let rect = Rect::new(position[0], position[1], width, width / aspect_ratio);

        let border_mesh = MeshBuilder::new()
            .rectangle(DrawMode::stroke(2.0), rect, Color::new(1.0, 1.0, 1.0, 1.0))?
            .build(ctx)?;

        let camera_scale = [
            width / world_dimensions[0] as f32 / CELL_PIXEL_SIZE[0],
            width / world_dimensions[0] as f32 / CELL_PIXEL_SIZE[1],
        ];
        let padding = 2.0;
        let camera_mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(1.0),
                Rect::new(
                    position[0],
                    position[1],
                    WORLD_VIEWPORT.w * camera_scale[0] - padding * 2.0,
                    WORLD_VIEWPORT.h * camera_scale[1] - padding * 2.0,
                ),
                Color::new(1.0, 1.0, 1.0, 1.0),
            )?
            .build(ctx)?;

        Ok(Self {
            border_mesh,
            camera_mesh,
            camera_scale,
            rect,
            is_mouse_dragging: false,
            padding,
        })
    }

    pub fn draw(&self, ctx: &mut Context, camera_position_in_world: [f32; 2]) -> GameResult {
        ggez::graphics::draw(ctx, &self.border_mesh, DrawParam::default())?;
        ggez::graphics::draw(
            ctx,
            &self.camera_mesh,
            DrawParam::default().dest([
                camera_position_in_world[0] * self.camera_scale[0] + self.padding,
                camera_position_in_world[1] * self.camera_scale[1] + self.padding,
            ]),
        )?;
        Ok(())
    }

    pub fn on_mouse_button_down(
        &mut self,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> Option<[f32; 2]> {
        if button == MouseButton::Left && self.rect.contains([x, y]) {
            self.is_mouse_dragging = true;
            Some(clamped_ratio(x, y, &self.rect))
        } else {
            None
        }
    }

    pub fn on_mouse_motion(&mut self, x: f32, y: f32) -> Option<[f32; 2]> {
        if self.is_mouse_dragging {
            Some(clamped_ratio(x, y, &self.rect))
        } else {
            None
        }
    }

    pub fn on_mouse_button_up(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.is_mouse_dragging = false;
        }
    }
}

fn clamped_ratio(x: f32, y: f32, rect: &Rect) -> [f32; 2] {
    let x_ratio = if x < rect.x {
        0.0
    } else if x > rect.right() {
        1.0
    } else {
        (x - rect.x) / rect.w
    };
    let y_ratio = if y < rect.y {
        0.0
    } else if y > rect.bottom() {
        1.0
    } else {
        (y - rect.y) / rect.h
    };
    [x_ratio, y_ratio]
}
