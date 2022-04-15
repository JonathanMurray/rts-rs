use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::{Context, GameResult};

use crate::text::{SharpFont, SharpText};

pub struct Healthbar {
    label: SharpText,
    font: SharpFont,
    bg: Mesh,
    position_on_screen: [f32; 2],
}

impl Healthbar {
    pub fn new(
        ctx: &mut Context,
        font: SharpFont,
        position_on_screen: [f32; 2],
    ) -> GameResult<Self> {
        let rect = Rect::new(
            position_on_screen[0] + 25.0,
            position_on_screen[1],
            110.0,
            15.0,
        );
        let bg = MeshBuilder::new()
            .rectangle(DrawMode::fill(), rect, Color::new(0.5, 0.5, 0.5, 1.0))?
            .rectangle(DrawMode::stroke(1.0), rect, Color::new(0.2, 0.2, 0.2, 1.0))?
            .build(ctx)?;

        let label = font.text(15.0, "HP:");

        Ok(Self {
            label,
            font,
            bg,
            position_on_screen,
        })
    }

    pub fn draw(&self, ctx: &mut Context, current: usize, max: usize) -> GameResult {
        self.bg.draw(ctx, DrawParam::default())?;

        let rect = self.bg.dimensions(ctx).unwrap();
        let health_ratio = current as f32 / max as f32;
        let health_color = if health_ratio > 0.65 {
            Color::new(0.2, 0.8, 0.2, 1.0)
        } else if health_ratio > 0.35 {
            Color::new(0.9, 0.9, 0.2, 1.0)
        } else {
            Color::new(0.9, 0.2, 0.2, 1.0)
        };
        let health = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(
                rect.x + 1.0,
                rect.y + 1.0,
                (rect.w - 2.0) * health_ratio,
                rect.h - 2.0,
            ),
            health_color,
        )?;

        health.draw(ctx, DrawParam::default())?;

        self.label.draw(ctx, self.position_on_screen)?;
        let health_text = self.font.text(9.0, format!("{} / {}", current, max));
        health_text.draw(
            ctx,
            [
                rect.center().x - health_text.dimensions(ctx).w / 2.0,
                rect.bottom() + 5.0,
            ],
        )?;
        Ok(())
    }
}
