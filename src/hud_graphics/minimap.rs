use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use super::HUD_BORDER_COLOR;
use crate::core::ObstacleType;
use crate::entities::Team;
use crate::game::{CELL_PIXEL_SIZE, COLOR_BG, WORLD_VIEWPORT};
use crate::grid::ObstacleGrid;
use crate::images;

pub struct Minimap {
    container_border: Mesh,
    bg: Mesh,
    camera: Mesh,
    player_entity_sprite_batch: SpriteBatch,
    enemy_1_entity_sprite_batch: SpriteBatch,
    enemy_2_entity_sprite_batch: SpriteBatch,
    neutral_entity_sprite_batch: SpriteBatch,
    water_sprite_batch: SpriteBatch,
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

        let height = width / aspect_ratio;
        let rect = Rect::new(
            position[0],
            position[1] + (container_h - height) / 2.0,
            width,
            height,
        );

        let container_border = MeshBuilder::new()
            .rectangle(DrawMode::stroke(1.0), container_rect, HUD_BORDER_COLOR)?
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
                DrawMode::stroke(1.0),
                Rect::new(
                    rect.x,
                    rect.y,
                    WORLD_VIEWPORT.w * camera_scale[0] - padding * 2.0,
                    WORLD_VIEWPORT.h * camera_scale[1] - padding * 2.0,
                ),
                Color::new(1.0, 1.0, 1.0, 1.0),
            )?
            .build(ctx)?;

        let cell_size = [
            width / world_dimensions[0] as f32 + 1.0,
            height / world_dimensions[1] as f32 + 1.0,
        ];

        let cell_rect = Rect::new(0.0, 0.0, cell_size[0], cell_size[1]);
        let player_entity_sprite_batch =
            sprite_batch(ctx, cell_rect, Color::new(0.5, 1.0, 0.5, 1.0))?;
        let enemy_1_entity_sprite_batch =
            sprite_batch(ctx, cell_rect, Color::new(0.8, 0.3, 0.3, 1.0))?;
        let enemy_2_entity_sprite_batch =
            sprite_batch(ctx, cell_rect, Color::new(1.0, 0.2, 1.0, 1.0))?;
        let neutral_entity_sprite_batch =
            sprite_batch(ctx, cell_rect, Color::new(0.5, 0.5, 0.5, 1.0))?;
        let water_sprite_batch = sprite_batch(ctx, cell_rect, Color::new(0.5, 0.5, 1.0, 1.0))?;

        Ok(Self {
            container_border,
            bg,
            camera,
            player_entity_sprite_batch,
            enemy_1_entity_sprite_batch,
            enemy_2_entity_sprite_batch,
            neutral_entity_sprite_batch,
            water_sprite_batch,
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
        grid: &ObstacleGrid,
    ) -> GameResult {
        self.bg.draw(ctx, DrawParam::default())?;
        self.draw_entity_markers(ctx, grid)?;
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

    fn draw_entity_markers(&mut self, ctx: &mut Context, grid: &ObstacleGrid) -> GameResult {
        let [w, h] = grid.dimensions();
        for x in 0..w {
            for y in 0..h {
                let sprite_batch = match grid.get(&[x, y]).unwrap() {
                    ObstacleType::Entity(Team::Player) => {
                        Some(&mut self.player_entity_sprite_batch)
                    }
                    ObstacleType::Entity(Team::Enemy1) => {
                        Some(&mut self.enemy_1_entity_sprite_batch)
                    }
                    ObstacleType::Entity(Team::Enemy2) => {
                        Some(&mut self.enemy_2_entity_sprite_batch)
                    }
                    ObstacleType::Entity(Team::Neutral) => {
                        Some(&mut self.neutral_entity_sprite_batch)
                    }
                    ObstacleType::Water => Some(&mut self.water_sprite_batch),
                    ObstacleType::None => None,
                };
                if let Some(sprite_batch) = sprite_batch {
                    let pos = [
                        (x as f32 / w as f32) * self.rect.w,
                        (y as f32 / h as f32) * self.rect.h,
                    ];
                    sprite_batch.add(DrawParam::default().dest(pos));
                }
            }
        }
        let param = DrawParam::default().dest(self.rect.point());
        self.player_entity_sprite_batch.draw(ctx, param)?;
        self.enemy_1_entity_sprite_batch.draw(ctx, param)?;
        self.enemy_2_entity_sprite_batch.draw(ctx, param)?;
        self.neutral_entity_sprite_batch.draw(ctx, param)?;
        self.water_sprite_batch.draw(ctx, param)?;
        self.player_entity_sprite_batch.clear();
        self.enemy_1_entity_sprite_batch.clear();
        self.enemy_2_entity_sprite_batch.clear();
        self.neutral_entity_sprite_batch.clear();
        self.water_sprite_batch.clear();
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

fn sprite_batch(ctx: &mut Context, rect: Rect, color: Color) -> GameResult<SpriteBatch> {
    let mesh = Mesh::new_rectangle(ctx, DrawMode::fill(), rect, color)?;
    let image = images::mesh_into_image(ctx, mesh)?;
    Ok(SpriteBatch::new(image))
}
