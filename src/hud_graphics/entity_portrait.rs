use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::{Context, GameResult};

use crate::game::COLOR_BG;

pub const PORTRAIT_DIMENSIONS: [f32; 2] = [80.0, 80.0];

pub struct EntityPortrait {
    position_on_screen: [f32; 2],
    border: Mesh,
}

impl EntityPortrait {
    pub fn new(ctx: &mut Context, position_on_screen: [f32; 2]) -> GameResult<Self> {
        let rect = Rect::new(
            position_on_screen[0],
            position_on_screen[1],
            PORTRAIT_DIMENSIONS[0],
            PORTRAIT_DIMENSIONS[1],
        );
        let border = MeshBuilder::new()
            .rectangle(DrawMode::fill(), rect, COLOR_BG)?
            .rectangle(DrawMode::stroke(2.0), rect, Color::new(0.1, 0.1, 0.1, 1.0))?
            .build(ctx)?;
        Ok(Self {
            position_on_screen,
            border,
        })
    }

    pub fn draw(&self, ctx: &mut Context, portrait: &Mesh) -> GameResult {
        self.border.draw(ctx, DrawParam::new())?;
        portrait.draw(ctx, DrawParam::new().dest(self.position_on_screen))?;
        Ok(())
    }
}
