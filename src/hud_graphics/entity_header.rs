use ggez::graphics::{DrawMode, DrawParam, Drawable, Mesh, Rect};
use ggez::{Context, GameResult};

use super::entity_portrait::{EntityPortrait, PORTRAIT_DIMENSIONS};
use super::healthbar::Healthbar;
use super::progress_bar::ProgressBar;
use super::HUD_BORDER_COLOR;
use crate::entities::Team;
use crate::text::SharpFont;

pub struct EntityHeader {
    border: Mesh,
    portrait: EntityPortrait,
    font: SharpFont,
    healthbar: Healthbar,
    progress_bar: ProgressBar,
    status_position_on_screen: [f32; 2],
    name_position_on_screen: [f32; 2],
}

impl EntityHeader {
    pub fn new(
        ctx: &mut Context,
        position_on_screen: [f32; 2],
        font: SharpFont,
    ) -> GameResult<Self> {
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            Rect::new(position_on_screen[0], position_on_screen[1], 195.0, 100.0),
            HUD_BORDER_COLOR,
        )?;

        let portrait_pos = [position_on_screen[0] + 5.0, position_on_screen[1] + 5.0];
        let portrait = EntityPortrait::new(ctx, portrait_pos)?;
        let healthbar = Healthbar::new(
            font,
            [
                portrait_pos[0] + PORTRAIT_DIMENSIONS[0] + 5.0,
                position_on_screen[1] + 5.0,
            ],
        );
        let progress_bar = ProgressBar::new(
            [
                portrait_pos[0] + PORTRAIT_DIMENSIONS[0] + 5.0,
                position_on_screen[1] + 35.0,
            ],
            font,
        );
        let status_position_on_screen = [
            portrait_pos[0] + PORTRAIT_DIMENSIONS[0] + 5.0,
            position_on_screen[1] + 30.0,
        ];
        let name_position_on_screen = [position_on_screen[0] + 10.0, position_on_screen[1] + 75.0];
        Ok(Self {
            border,
            portrait,
            font,
            healthbar,
            progress_bar,
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
        self.portrait.draw(ctx, content.portrait, false)?;
        if let Some(status) = content.status {
            self.font
                .text(12.0, status)
                .draw(ctx, self.status_position_on_screen)?;
        }
        if let Some(progress) = content.progress {
            self.progress_bar.draw(ctx, progress)?;
        }
        self.font
            .text(17.5, content.name)
            .draw(ctx, self.name_position_on_screen)?;
        Ok(())
    }
}

pub struct EntityHeaderContent<'a> {
    pub current_health: usize,
    pub max_health: usize,
    pub portrait: &'a Mesh,
    pub name: String,
    pub status: Option<String>,
    pub progress: Option<f32>,
    pub team: Team,
}
