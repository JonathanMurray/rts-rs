use ggez::graphics::{DrawParam, Drawable, Font, Rect, Text};
use ggez::{Context, GameResult};

/// This module exists to avoid getting blurry text when scaling up game window. Images and meshes
/// scale fine by default, but text becomes blurry.
///
/// To bypass the issue, we create the text using a larger size and then scale down when drawing it.

const SCALING: f32 = 3.0;

#[derive(Copy, Clone)]
pub struct SharpFont(Font);

impl SharpFont {
    pub fn new(font: Font) -> Self {
        Self(font)
    }

    pub fn text(&self, size: f32, text: impl Into<String>) -> SharpText {
        let text = Text::new((text.into(), self.0, size * SCALING));
        SharpText(text)
    }
}

#[derive(Debug)]
pub struct SharpText(Text);

impl SharpText {
    pub fn draw(&self, ctx: &mut Context, position: [f32; 2]) -> GameResult {
        self.0.draw(
            ctx,
            DrawParam::default()
                .scale([1.0 / SCALING, 1.0 / SCALING])
                .dest(position),
        )
    }

    pub fn dimensions(&self, ctx: &Context) -> Rect {
        let mut rect = self.0.dimensions(ctx);
        rect.scale(1.0 / SCALING, 1.0 / SCALING);
        rect
    }
}
