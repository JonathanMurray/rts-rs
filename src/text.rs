use ggez::graphics::{Color, DrawParam, Drawable, Font, Text};
use ggez::{Context, GameResult};

/// This module exists to avoid getting blurry text when scaling up game window. Images and meshes
/// scale fine by default, but text becomes blurry.
///
/// To bypass the issue, we create the text using a larger size and then scale down when drawing it.

const SCALING: f32 = 3.0;

#[derive(Copy, Clone)]
pub struct SharpFont {
    font: Font,
}

impl SharpFont {
    pub fn new(font: Font) -> Self {
        Self { font }
    }

    pub fn text(&self, size: f32, text: impl Into<String>) -> SharpText {
        let text = Text::new((text.into(), self.font, size * SCALING));
        SharpText { text }
    }
}

#[derive(Debug)]
pub struct SharpText {
    text: Text,
}

impl SharpText {
    pub fn draw(&self, ctx: &mut Context, position: [f32; 2]) -> GameResult {
        self.text.draw(
            ctx,
            DrawParam::default()
                .scale([1.0 / SCALING, 1.0 / SCALING])
                .dest(position),
        )
    }

    pub fn with_color(mut self, color: Color) -> Self {
        for fragment in self.text.fragments_mut() {
            fragment.color = Some(color);
        }
        self
    }
}
