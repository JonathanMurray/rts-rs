use ggez::graphics::{self, Color, DrawMode, DrawParam, Font, Mesh, MeshBuilder, Rect, Text};
use ggez::{Context, GameResult};

use crate::entities::Entity;
use crate::game::{TeamState, CAMERA_SIZE, CELL_PIXEL_SIZE};

pub struct HudGraphics {
    position_on_screen: [f32; 2],
    font: Font,
}

impl HudGraphics {
    pub fn new(position: [f32; 2], font: Font) -> Self {
        Self {
            position_on_screen: position,
            font,
        }
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        player_team_state: &TeamState,
        selected_entity: Option<&Entity>,
    ) -> GameResult {
        let x = 0.0;
        let resources_y = 5.0;
        let name_y = 48.0;
        let health_y = 130.0;
        let training_y = 180.0;

        let small_font = 30.0;
        let large_font = 40.0;

        self.draw_text(
            ctx,
            [x, resources_y],
            format!("Resources: {}", player_team_state.resources),
            small_font,
        )?;

        if let Some(selected_entity) = selected_entity {
            self.draw_text(ctx, [x, name_y], selected_entity.name, large_font)?;

            if let Some(health) = &selected_entity.health {
                let health = format!(
                    "HP: [{}{}]",
                    "=".repeat(health.current as usize),
                    " ".repeat((health.max - health.current) as usize)
                );
                self.draw_text(ctx, [x, health_y], health, small_font)?;
            }

            if let Some(training_action) = &selected_entity.training_action {
                if let Some(progress) = training_action.progress() {
                    self.draw_text(ctx, [x, training_y], "Training in progress", small_font)?;
                    let progress_w = 20.0;
                    let progress_bar = format!(
                        "[{}{}]",
                        "=".repeat((progress * progress_w) as usize),
                        " ".repeat(((1.0 - progress) * progress_w) as usize)
                    );
                    self.draw_text(ctx, [x, training_y + 35.0], progress_bar, small_font)?;
                } else {
                    self.draw_text(
                        ctx,
                        [x, training_y],
                        "Press [B] to train a unit",
                        small_font,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn draw_text(
        &self,
        ctx: &mut Context,
        position: [f32; 2],
        line: impl Into<String>,
        font_size: f32,
    ) -> GameResult {
        let text = Text::new((line.into(), self.font, font_size));
        graphics::draw(
            ctx,
            &text,
            DrawParam::new().dest([
                self.position_on_screen[0] + position[0],
                self.position_on_screen[1] + position[1],
            ]),
        )
    }
}

pub struct MinimapGraphics {
    border_mesh: Mesh,
    camera_mesh: Mesh,
    camera_scale: [f32; 2],
}

impl MinimapGraphics {
    pub fn new(
        ctx: &mut Context,
        position: [f32; 2],
        map_dimensions: [u32; 2],
    ) -> GameResult<Self> {
        let cell_pixel_size_in_minimap = 5.0;

        let border_mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(2.0),
                Rect::new(
                    position[0],
                    position[1],
                    map_dimensions[0] as f32 * cell_pixel_size_in_minimap,
                    map_dimensions[1] as f32 * cell_pixel_size_in_minimap,
                ),
                Color::new(1.0, 1.0, 1.0, 1.0),
            )?
            .build(ctx)?;

        let camera_scale = [
            cell_pixel_size_in_minimap / CELL_PIXEL_SIZE[0],
            cell_pixel_size_in_minimap / CELL_PIXEL_SIZE[1],
        ];
        let camera_mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(1.0),
                Rect::new(
                    position[0],
                    position[1],
                    CAMERA_SIZE[0] * camera_scale[0],
                    CAMERA_SIZE[1] * camera_scale[1],
                ),
                Color::new(1.0, 1.0, 1.0, 1.0),
            )?
            .build(ctx)?;

        Ok(Self {
            border_mesh,
            camera_mesh,
            camera_scale,
        })
    }

    pub fn draw(&self, ctx: &mut Context, camera_position_in_world: [f32; 2]) -> GameResult {
        ggez::graphics::draw(ctx, &self.border_mesh, DrawParam::default())?;
        ggez::graphics::draw(
            ctx,
            &self.camera_mesh,
            DrawParam::default().dest([
                camera_position_in_world[0] * self.camera_scale[0],
                camera_position_in_world[1] * self.camera_scale[1],
            ]),
        )?;

        Ok(())
    }
}
