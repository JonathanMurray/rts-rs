use ggez::graphics::{DrawMode, DrawParam, Drawable, Mesh, Rect};
use ggez::{Context, GameResult};

use super::entity_portrait::{EntityPortrait, PORTRAIT_DIMENSIONS};
use super::HUD_BORDER_COLOR;
use crate::game::MAX_NUM_SELECTED_ENTITIES;

const NUM_PORTRAITS: usize = MAX_NUM_SELECTED_ENTITIES;

pub struct GroupHeader {
    border: Mesh,
    portraits: [EntityPortrait; NUM_PORTRAITS],
}

impl GroupHeader {
    pub fn new(ctx: &mut Context, position_on_screen: [f32; 2]) -> GameResult<Self> {
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(3.0),
            Rect::new(position_on_screen[0], position_on_screen[1], 390.0, 200.0),
            HUD_BORDER_COLOR,
        )?;
        let x = position_on_screen[0] + 10.0;
        let y = position_on_screen[1] + 10.0;
        let margin = [16.0, 16.0];
        let [w, h] = PORTRAIT_DIMENSIONS;
        let portraits = [
            EntityPortrait::new(ctx, [x, y])?,
            EntityPortrait::new(ctx, [x + w + margin[0], y])?,
            EntityPortrait::new(ctx, [x + (w + margin[0]) * 2.0, y])?,
            EntityPortrait::new(ctx, [x + (w + margin[0]) * 3.0, y])?,
            EntityPortrait::new(ctx, [x, y + h + margin[1]])?,
            EntityPortrait::new(ctx, [x + w + margin[0], y + h + margin[1]])?,
            EntityPortrait::new(ctx, [x + (w + margin[0]) * 2.0, y + h + margin[1]])?,
            EntityPortrait::new(ctx, [x + (w + margin[0]) * 3.0, y + h + margin[1]])?,
        ];
        Ok(Self { border, portraits })
    }

    pub fn draw(&self, ctx: &mut Context, portraits: [Option<&Mesh>; NUM_PORTRAITS]) -> GameResult {
        self.border.draw(ctx, DrawParam::new())?;
        for (i, portrait) in portraits.iter().enumerate() {
            if let Some(portrait) = portrait {
                self.portraits[i].draw(ctx, portrait)?;
            }
        }
        Ok(())
    }
}
