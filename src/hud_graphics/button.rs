use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::{Context, GameResult};

use std::time::Duration;

use crate::data::Picture;
use crate::entities::Action;
use crate::hud_graphics::entity_portrait::PORTRAIT_DIMENSIONS;
use crate::hud_graphics::HUD_BORDER_COLOR;
use crate::player::CursorState;

pub struct Button {
    action: Option<Action>,
    rect: Rect,
    border: Mesh,
    outline_active: Mesh,
    highlight: Mesh,
    is_down: bool,
    down_cooldown: Duration,
    graphics: Option<Picture>,
}

impl Button {
    pub fn new(ctx: &mut Context, rect: Rect) -> GameResult<Button> {
        let local_rect = Rect::new(0.0, 0.0, rect.w, rect.h);
        let border = MeshBuilder::new()
            .rectangle(DrawMode::stroke(2.0), local_rect, HUD_BORDER_COLOR)?
            .build(ctx)?;
        let outline_active = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(2.0),
                Rect::new(2.0, 2.0, rect.w - 4.0, rect.h - 4.0),
                Color::new(0.4, 0.95, 0.4, 1.0),
            )?
            .build(ctx)?;
        let highlight = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                local_rect,
                Color::new(1.0, 1.0, 0.6, 0.05),
            )?
            .build(ctx)?;

        Ok(Self {
            action: None,
            rect,
            border,
            outline_active,
            highlight,
            is_down: false,
            down_cooldown: Duration::ZERO,
            graphics: None,
        })
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        hover: bool,
        cursor_state: CursorState,
        active: bool,
    ) -> GameResult {
        if let Some(action) = self.action {
            self.border
                .draw(ctx, DrawParam::default().dest(self.rect.point()))?;
            if active {
                self.outline_active
                    .draw(ctx, DrawParam::default().dest(self.rect.point()))?;
            }

            let matches_cursor_state = match cursor_state {
                CursorState::Default => false,
                CursorState::SelectingAttackTarget => action == Action::Attack,
                CursorState::SelectingMovementDestination => action == Action::Move,
                CursorState::PlacingStructure(structure_type) => {
                    matches!(action, Action::Construct(s_type, _) if s_type == structure_type)
                }
                CursorState::SelectingResourceTarget => action == Action::GatherResource,
                CursorState::DraggingSelectionArea(_) => false,
            };

            let offset = if self.is_down { [4.0, 4.0] } else { [0.0, 0.0] };
            let scale = if self.is_down { [0.9, 0.9] } else { [1.0, 1.0] };
            if let Some(graphics) = &self.graphics {
                graphics.draw(
                    ctx,
                    DrawParam::default()
                        .dest([
                            self.rect.x + (self.rect.w - PORTRAIT_DIMENSIONS[0]) / 2.0,
                            self.rect.y + (self.rect.h - PORTRAIT_DIMENSIONS[1]) / 2.0,
                        ])
                        .scale(scale),
                )?;
            }
            if matches_cursor_state || hover {
                self.highlight.draw(
                    ctx,
                    DrawParam::default()
                        .dest([self.rect.x + offset[0], self.rect.y + offset[1]])
                        .scale(scale),
                )?;
            }
        }

        Ok(())
    }

    pub fn update(&mut self, dt: Duration) {
        if self.is_down {
            self.down_cooldown = self.down_cooldown.saturating_sub(dt);
            if self.down_cooldown.is_zero() {
                self.is_down = false;
            }
        }
    }

    pub fn on_click(&mut self) -> Option<Action> {
        self.action.map(|action| {
            self.is_down = true;
            self.down_cooldown = Duration::from_millis(100);
            action
        })
    }

    pub fn set_action(&mut self, action_and_picture: Option<(Action, Picture)>) {
        if let Some((action, picture)) = action_and_picture {
            self.action = Some(action);
            self.graphics = Some(picture);
        } else {
            self.action = None;
            self.graphics = None;
        }
    }

    pub fn action(&self) -> Option<Action> {
        self.action
    }

    pub fn contains(&self, screen_pixel_coords: [f32; 2]) -> bool {
        self.rect.contains(screen_pixel_coords)
    }
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("rect", &self.rect)
            .field("action", &self.action)
            .finish()
    }
}
