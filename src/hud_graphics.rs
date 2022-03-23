use ggez::graphics::{self, DrawParam, Font, Text};
use ggez::{Context, GameResult};

use crate::entities::Entity;

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
        selected_entity: Option<&Entity>,
        _num_entities: usize,
    ) -> GameResult {
        let x = 0.0;
        let name_y = 8.0;
        let health_y = 90.0;
        let training_y = 140.0;

        let small_font = 30.0;
        let large_font = 40.0;

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
