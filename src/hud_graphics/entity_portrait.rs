use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Image, Mesh, MeshBuilder, Rect};
use ggez::{Context, GameResult};

use crate::game::COLOR_BG;

pub const PORTRAIT_DIMENSIONS: [f32; 2] = [40.0, 40.0];

pub struct EntityPortrait {
    rect: Rect,
    position_on_screen: [f32; 2],
    border: Mesh,
    highlight: Mesh,
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
            .rectangle(DrawMode::stroke(1.0), rect, Color::new(0.1, 0.1, 0.1, 1.0))?
            .build(ctx)?;
        let highlight = MeshBuilder::new()
            .rectangle(DrawMode::stroke(1.0), rect, Color::new(0.6, 0.6, 0.6, 1.0))?
            .build(ctx)?;
        Ok(Self {
            rect,
            position_on_screen,
            border,
            highlight,
        })
    }

    pub fn draw(&self, ctx: &mut Context, portrait: &Image, highlight: bool) -> GameResult {
        self.border.draw(ctx, DrawParam::new())?;
        portrait.draw(ctx, DrawParam::new().dest(self.position_on_screen))?;
        if highlight {
            self.highlight.draw(ctx, DrawParam::new())?;
        }
        Ok(())
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }
}
