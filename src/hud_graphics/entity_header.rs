use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Font, Mesh, Rect, Text};
use ggez::{Context, GameResult};

use super::healthbar::Healthbar;
use super::trainingbar::Trainingbar;
use crate::entities::Team;

pub struct EntityHeader {
    position_on_screen: [f32; 2],
    border: Mesh,
    portrait_border: Mesh,
    font: Font,
    healthbar: Healthbar,
    trainingbar: Trainingbar,
}

impl EntityHeader {
    pub fn new(ctx: &mut Context, position_on_screen: [f32; 2], font: Font) -> GameResult<Self> {
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(3.0),
            Rect::new(position_on_screen[0], position_on_screen[1], 360.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
        )?;
        let portrait_border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            Rect::new(
                position_on_screen[0] + 10.0,
                position_on_screen[1] + 10.0,
                60.0,
                60.0,
            ),
            Color::new(0.7, 0.7, 1.0, 1.0),
        )?;
        let healthbar = Healthbar::new(
            font,
            [position_on_screen[0] + 80.0, position_on_screen[1] + 10.0],
        );
        let trainingbar = Trainingbar::new(
            [position_on_screen[0] + 80.0, position_on_screen[1] + 70.0],
            font,
        );
        Ok(Self {
            position_on_screen,
            border,
            portrait_border,
            font,
            healthbar,
            trainingbar,
        })
    }

    pub fn draw(&self, ctx: &mut Context, content: EntityHeaderContent) -> GameResult {
        self.border.draw(ctx, DrawParam::new())?;
        self.healthbar.draw(
            ctx,
            content.current_health,
            content.max_health,
            content.team,
        )?;
        self.portrait_border.draw(ctx, DrawParam::new())?;
        content.portrait.draw(
            ctx,
            DrawParam::new().dest([
                self.position_on_screen[0] + 15.0,
                self.position_on_screen[1] + 15.0,
            ]),
        )?;
        if let Some(status) = content.status {
            Text::new((status, self.font, 24.0)).draw(
                ctx,
                DrawParam::new().dest([
                    self.position_on_screen[0] + 80.0,
                    self.position_on_screen[1] + 60.0,
                ]),
            )?;
        }
        if let Some(training_progress) = content.training_progress {
            self.trainingbar.draw(ctx, training_progress)?;
        }
        Text::new((content.name, self.font, 32.0)).draw(
            ctx,
            DrawParam::new().dest([
                self.position_on_screen[0] + 20.0,
                self.position_on_screen[1] + 150.0,
            ]),
        )?;
        Ok(())
    }
}

pub struct EntityHeaderContent<'a> {
    pub current_health: usize,
    pub max_health: usize,
    pub portrait: &'a Mesh,
    pub name: String,
    pub status: Option<String>,
    pub training_progress: Option<f32>,
    pub team: Team,
}
