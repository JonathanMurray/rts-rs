use ggez::{Context, GameResult};

use crate::text::SharpFont;

pub struct ProgressBar {
    position_on_screen: [f32; 2],
    font: SharpFont,
}

impl ProgressBar {
    pub fn new(position_on_screen: [f32; 2], font: SharpFont) -> Self {
        Self {
            position_on_screen,
            font,
        }
    }

    pub fn draw(&self, ctx: &mut Context, progress: f32) -> GameResult {
        let w = 14.0;
        let bar = format!(
            "[{}{}]",
            "=".repeat((progress * w) as usize),
            " ".repeat(((1.0 - progress) * w) as usize)
        );
        self.font.text(14.0, bar).draw(
            ctx,
            [
                self.position_on_screen[0],
                self.position_on_screen[1] + 15.0,
            ],
        )?;

        Ok(())
    }
}
