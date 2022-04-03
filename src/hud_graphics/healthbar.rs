use ggez::graphics::{DrawParam, Drawable, Font, Text};
use ggez::{Context, GameResult};

pub struct Healthbar {
    font: Font,
    position_on_screen: [f32; 2],
}

impl Healthbar {
    pub fn new(font: Font, position_on_screen: [f32; 2]) -> Self {
        Self {
            position_on_screen,
            font,
        }
    }

    pub fn draw(&self, ctx: &mut Context, current: usize, max: usize) -> GameResult {
        let text = format!(
            "HP: [{}{}]",
            "=".repeat(current as usize),
            " ".repeat((max - current) as usize)
        );
        Text::new((text, self.font, 30.0))
            .draw(ctx, DrawParam::new().dest(self.position_on_screen))?;
        Ok(())
    }
}
