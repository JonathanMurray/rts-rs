use ggez::graphics::Color;
use ggez::{Context, GameResult};

use crate::entities::Team;
use crate::text::SharpFont;

pub struct Healthbar {
    font: SharpFont,
    position_on_screen: [f32; 2],
}

impl Healthbar {
    pub fn new(font: SharpFont, position_on_screen: [f32; 2]) -> Self {
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
        self.font
            .text(15.0, text)
            .with_color(color)
            .draw(ctx, self.position_on_screen)?;
        Ok(())
    }
}
