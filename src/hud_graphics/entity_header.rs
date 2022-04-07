use ggez::graphics::{DrawMode, DrawParam, Drawable, Font, Mesh, Rect, Text};
use ggez::{Context, GameResult};

use super::entity_portrait::{EntityPortrait, PORTRAIT_DIMENSIONS};
use super::healthbar::Healthbar;
use super::trainingbar::Trainingbar;
use super::HUD_BORDER_COLOR;
use crate::entities::Team;

pub struct EntityHeader {
    border: Mesh,
    portrait: EntityPortrait,
    font: Font,
    healthbar: Healthbar,
    trainingbar: Trainingbar,
    status_position_on_screen: [f32; 2],
    name_position_on_screen: [f32; 2],
}

impl EntityHeader {
    pub fn new(ctx: &mut Context, position_on_screen: [f32; 2], font: Font) -> GameResult<Self> {
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(3.0),
            Rect::new(position_on_screen[0], position_on_screen[1], 390.0, 200.0),
            HUD_BORDER_COLOR,
        )?;

        let portrait_pos = [position_on_screen[0] + 10.0, position_on_screen[1] + 10.0];
        let portrait = EntityPortrait::new(ctx, portrait_pos)?;
        let healthbar = Healthbar::new(
            font,
            [
                portrait_pos[0] + PORTRAIT_DIMENSIONS[0] + 10.0,
                position_on_screen[1] + 10.0,
            ],
        );
        let trainingbar = Trainingbar::new(
            [
                portrait_pos[0] + PORTRAIT_DIMENSIONS[0] + 10.0,
                position_on_screen[1] + 70.0,
            ],
            font,
        );
        let status_position_on_screen = [
            position_on_screen[0] + PORTRAIT_DIMENSIONS[0] + 10.0,
            position_on_screen[1] + 60.0,
        ];
        let name_position_on_screen = [position_on_screen[0] + 20.0, position_on_screen[1] + 150.0];
        Ok(Self {
            border,
            portrait,
            font,
            healthbar,
            trainingbar,
            status_position_on_screen,
            name_position_on_screen,
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
        self.portrait.draw(ctx, content.portrait)?;
        if let Some(status) = content.status {
            Text::new((status, self.font, 24.0))
                .draw(ctx, DrawParam::new().dest(self.status_position_on_screen))?;
        }
        if let Some(training_progress) = content.training_progress {
            self.trainingbar.draw(ctx, training_progress)?;
        }
        Text::new((content.name, self.font, 35.0))
            .draw(ctx, DrawParam::new().dest(self.name_position_on_screen))?;
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
