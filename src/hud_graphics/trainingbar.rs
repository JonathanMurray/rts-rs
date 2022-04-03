use ggez::graphics::{DrawParam, Drawable, Font, Text};
use ggez::{Context, GameResult};

pub struct Trainingbar {
    font: Font,
    position_on_screen: [f32; 2],
}

impl Trainingbar {
    pub fn new(font: Font, position_on_screen: [f32; 2]) -> Self {
        Self {
            position_on_screen,
            font,
        }
    }

    pub fn draw(&self, ctx: &mut Context, unit_name: &str, progress: f32) -> GameResult {
        let header = format!("Training {:?}", unit_name);
        Text::new((header, self.font, 30.0))
            .draw(ctx, DrawParam::new().dest(self.position_on_screen))?;

        let w = 20.0;
        let bar = format!(
            "[{}{}]",
            "=".repeat((progress * w) as usize),
            " ".repeat(((1.0 - progress) * w) as usize)
        );
        Text::new((bar, self.font, 30.0)).draw(
            ctx,
            DrawParam::new().dest([
                self.position_on_screen[0],
                self.position_on_screen[1] + 30.0,
            ]),
        )?;

        Ok(())
    }
}
