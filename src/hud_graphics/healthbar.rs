use ggez::graphics::{Color, DrawParam, Drawable, Font, Text};
use ggez::{Context, GameResult};

use crate::entities::Team;

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

    pub fn draw(&self, ctx: &mut Context, current: usize, max: usize, team: Team) -> GameResult {
        let text = format!(
            "HP: [{}{}]",
            "=".repeat(current as usize),
            " ".repeat((max - current) as usize)
        );
        let color = match team {
            Team::Player => Color::new(0.6, 1.0, 0.6, 1.0),
            Team::Enemy => Color::new(0.8, 0.5, 0.5, 1.0),
            Team::Neutral => Color::new(0.6, 0.6, 0.6, 1.0),
        };
        Text::new((text, self.font, 15.0)).draw(
            ctx,
            DrawParam::new().color(color).dest(self.position_on_screen),
        )?;
        Ok(())
    }
}
