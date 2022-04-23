use ggez::graphics::{DrawMode, DrawParam, Drawable, Image, Mesh, Rect};
use ggez::{Context, GameResult};

use super::entity_portrait::{EntityPortrait, PORTRAIT_DIMENSIONS};
use super::{PlayerInput, HUD_BORDER_COLOR};
use crate::game::MAX_NUM_SELECTED_ENTITIES;

const NUM_PORTRAITS: usize = MAX_NUM_SELECTED_ENTITIES;

pub struct GroupHeader {
    border: Mesh,
    portraits: [EntityPortrait; NUM_PORTRAITS],
    hovered_portrait_index: Option<usize>,
}

impl GroupHeader {
    pub fn new(ctx: &mut Context, position_on_screen: [f32; 2]) -> GameResult<Self> {
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            Rect::new(position_on_screen[0], position_on_screen[1], 195.0, 100.0),
            HUD_BORDER_COLOR,
        )?;
        let x = position_on_screen[0] + 5.0;
        let y = position_on_screen[1] + 5.0;
        let margin = [8.0, 8.0];
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
        Ok(Self {
            border,
            portraits,
            hovered_portrait_index: None,
        })
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        portraits: [Option<&Image>; NUM_PORTRAITS],
    ) -> GameResult {
        self.border.draw(ctx, DrawParam::new())?;
        for (i, portrait) in portraits.iter().enumerate() {
            if let Some(portrait) = portrait {
                let is_hovered = self.hovered_portrait_index == Some(i);
                self.portraits[i].draw(ctx, portrait, is_hovered)?;
            }
        }
        Ok(())
    }

    pub fn on_mouse_button_down(&self, x: f32, y: f32) -> Option<PlayerInput> {
        self.portraits
            .iter()
            .position(|portrait| portrait.rect().contains([x, y]))
            .map(PlayerInput::LimitSelectionToIndex)
    }

    pub fn on_mouse_motion(&mut self, x: f32, y: f32) {
        self.hovered_portrait_index = self
            .portraits
            .iter()
            .position(|portrait| portrait.rect().contains([x, y]));
    }
}
