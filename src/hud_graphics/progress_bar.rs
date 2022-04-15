use ggez::{Context, GameResult};

use crate::text::SharpFont;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};

pub struct ProgressBar {
    bg: Mesh,
    font: SharpFont,
}

impl ProgressBar {
    pub fn new(
        ctx: &mut Context,
        position_on_screen: [f32; 2],
        font: SharpFont,
    ) -> GameResult<Self> {
        let rect = Rect::new(position_on_screen[0], position_on_screen[1], 110.0, 15.0);
        let bg = MeshBuilder::new()
            .rectangle(DrawMode::fill(), rect, Color::new(0.5, 0.5, 0.5, 1.0))?
            .rectangle(DrawMode::stroke(1.0), rect, Color::new(0.2, 0.2, 0.2, 1.0))?
            .build(ctx)?;

        Ok(Self { bg, font })
    }

    pub fn draw(&self, ctx: &mut Context, progress: f32, text: impl Into<String>) -> GameResult {
        self.bg.draw(ctx, DrawParam::default())?;

        let rect = self.bg.dimensions(ctx).unwrap();
        let progress_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(
                rect.x + 1.0,
                rect.y + 1.0,
                (rect.w - 2.0) * progress,
                rect.h - 2.0,
            ),
            Color::new(0.2, 0.8, 0.2, 1.0),
        )?;
        progress_mesh.draw(ctx, DrawParam::default())?;

        let progress_text = self.font.text(11.0, text);

        let text_x = rect.center().x - progress_text.dimensions(ctx).w / 2.0;
        progress_text.draw(ctx, [text_x, rect.y + 2.0])?;

        Ok(())
    }
}
