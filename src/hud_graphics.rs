use ggez::graphics::{self, DrawParam, Font, Text};
use ggez::{Context, GameResult};

use crate::entities::Entity;

pub struct HudGraphics {
    position: [f32; 2],
    font: Font,
}

impl HudGraphics {
    pub fn new(position: [f32; 2], font: Font) -> Self {
        Self { position, font }
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        selected_entity: Option<&Entity>,
        num_entities: usize,
    ) -> GameResult {
        let mut lines = vec![];
        lines.push(format!("Total entities: {:?}", num_entities));
        if let Some(selected_entity) = selected_entity {
            lines.push(format!("Selected: {:?}", selected_entity.id));
            if let Some(training_action) = &selected_entity.training_action {
                if let Some(progress) = training_action.progress() {
                    lines.push("Training in progress:".to_string());
                    let progress_w = 20.0;
                    lines.push(format!(
                        "[{}{}]",
                        "=".repeat((progress * progress_w) as usize),
                        " ".repeat(((1.0 - progress) * progress_w) as usize)
                    ));
                } else {
                    lines.push("Press B to train a unit".to_string());
                }
            }
        } else {
            lines.push("[no selected entity]".to_string());
        }

        let x = self.position[0];
        let mut y = self.position[1];
        for line in lines {
            let text = Text::new((line, self.font, 30.0));
            graphics::draw(ctx, &text, DrawParam::new().dest([x, y]))?;
            y += 35.0;
        }
        Ok(())
    }
}
