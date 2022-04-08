use ggez::graphics::{DrawParam, Drawable, Font, Text};
use ggez::{Context, GameResult};

pub struct ProgressBar {
    position_on_screen: [f32; 2],
    font: Font,
}

impl ProgressBar {
    pub fn new(position_on_screen: [f32; 2], font: Font) -> Self {
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
        Text::new((bar, self.font, 28.0)).draw(
            ctx,
            DrawParam::new().dest([
                self.position_on_screen[0],
                self.position_on_screen[1] + 30.0,
            ]),
        )?;

        Ok(())
    }
}
