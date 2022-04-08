use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use super::HUD_BORDER_COLOR;
use crate::game::{CELL_PIXEL_SIZE, COLOR_BG, WORLD_VIEWPORT};
use crate::grid::EntityGrid;
use crate::images;

pub struct Minimap {
    container_border: Mesh,
    bg: Mesh,
    camera: Mesh,
    entity_sprite_batch: SpriteBatch,
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
        let container_h = width;
        let container_rect = Rect::new(position[0], position[1], width, container_h);

        let h = width / aspect_ratio;
        let rect = Rect::new(position[0], position[1] + (container_h - h) / 2.0, width, h);

        let container_border = MeshBuilder::new()
            .rectangle(DrawMode::stroke(2.0), container_rect, HUD_BORDER_COLOR)?
            .build(ctx)?;
        let bg = MeshBuilder::new()
            .rectangle(DrawMode::fill(), rect, COLOR_BG)?
            .build(ctx)?;

        let camera_scale = [
            width / world_dimensions[0] as f32 / CELL_PIXEL_SIZE[0],
            width / world_dimensions[0] as f32 / CELL_PIXEL_SIZE[1],
        ];
        let padding = 2.0;
        let camera = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(2.0),
                Rect::new(
                    rect.x,
                    rect.y,
                    WORLD_VIEWPORT.w * camera_scale[0] - padding * 2.0,
                    WORLD_VIEWPORT.h * camera_scale[1] - padding * 2.0,
                ),
                Color::new(1.0, 1.0, 1.0, 1.0),
            )?
            .build(ctx)?;

        let entity_width = 10.0;
        let entity_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(0.0, 0.0, entity_width, entity_width),
            Color::new(0.5, 0.5, 0.5, 1.0),
        )?;
        let entity_sprite_batch = SpriteBatch::new(images::mesh_into_image(ctx, entity_mesh)?);

        Ok(Self {
            container_border,
            bg,
            camera,
            entity_sprite_batch,
            camera_scale,
            rect,
            is_mouse_dragging: false,
            padding,
        })
    }

    pub fn draw(
        &mut self,
        ctx: &mut Context,
        camera_position_in_world: [f32; 2],
        grid: &EntityGrid,
    ) -> GameResult {
        self.bg.draw(ctx, DrawParam::default())?;
        self.draw_entities(ctx, grid)?;
        self.camera.draw(
            ctx,
            DrawParam::default().dest([
                camera_position_in_world[0] * self.camera_scale[0] + self.padding,
                camera_position_in_world[1] * self.camera_scale[1] + self.padding,
            ]),
        )?;

        self.container_border.draw(ctx, DrawParam::default())?;

        Ok(())
    }

    fn draw_entities(&mut self, ctx: &mut Context, grid: &EntityGrid) -> GameResult {
        let [w, h] = grid.dimensions;
        for x in 0..w {
            for y in 0..h {
                if grid.get(&[x, y]) {
                    self.entity_sprite_batch.add(DrawParam::default().dest([
                        (x as f32 / w as f32) * self.rect.w - 5.0,
                        (y as f32 / h as f32) * self.rect.h - 5.0,
                    ]));
                }
            }
        }
        self.entity_sprite_batch
            .draw(ctx, DrawParam::default().dest(self.rect.point()))?;
        self.entity_sprite_batch.clear();
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
