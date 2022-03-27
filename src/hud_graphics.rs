use ggez::graphics::{
    self, Color, DrawMode, DrawParam, Drawable, Font, Mesh, MeshBuilder, Rect, Text,
};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

use crate::entities::{Action, Entity, EntityState, Team, NUM_UNIT_ACTIONS};
use crate::game::{CursorAction, TeamState, CELL_PIXEL_SIZE, WORLD_VIEWPORT};

const NUM_BUTTONS: usize = NUM_UNIT_ACTIONS;

pub struct HudGraphics {
    position_on_screen: [f32; 2],
    font: Font,
    buttons: [Button; NUM_BUTTONS],
}

impl HudGraphics {
    pub fn new(ctx: &mut Context, position: [f32; 2], font: Font) -> GameResult<Self> {
        let w = 80.0;
        let button_1_rect = Rect::new(position[0] + 5.0, position[1] + 270.0, w, w);
        let button_1 = Button::new(ctx, button_1_rect, "C", font)?;
        let button_margin = 5.0;
        let button_2_rect = Rect::new(button_1_rect.x + w + button_margin, button_1_rect.y, w, w);
        let button_2 = Button::new(ctx, button_2_rect, "V", font)?;
        let button_3_rect = Rect::new(button_2_rect.x + w + button_margin, button_1_rect.y, w, w);
        let button_3 = Button::new(ctx, button_3_rect, "B", font)?;
        let buttons = [button_1, button_2, button_3];
        Ok(Self {
            position_on_screen: position,
            font,
            buttons,
        })
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        player_team_state: &TeamState,
        selected_entity: Option<&Entity>,
        cursor_action: CursorAction,
        mouse_position: [f32; 2],
    ) -> GameResult {
        let x = 0.0;
        let resources_y = 5.0;
        let name_y = 48.0;
        let health_y = 130.0;
        let training_status_y = 240.0;
        let progress_y = 290.0;
        let tooltip_y = 380.0;

        let small_font = 20.0;
        let medium_font = 30.0;
        let large_font = 40.0;

        self.draw_text(
            ctx,
            [x, resources_y],
            format!("Resources: {}", player_team_state.resources),
            medium_font,
        )?;

        if let Some(selected_entity) = selected_entity {
            self.draw_text(ctx, [x, name_y], selected_entity.name, large_font)?;

            if let Some(health) = &selected_entity.health {
                let health = format!(
                    "HP: [{}{}]",
                    "=".repeat(health.current as usize),
                    " ".repeat((health.max - health.current) as usize)
                );
                self.draw_text(ctx, [x, health_y], health, medium_font)?;
            }

            self.draw_text(
                ctx,
                [x + 200.0, health_y],
                format!("({:?})", selected_entity.state),
                small_font,
            )?;

            if selected_entity.team == Team::Player {
                let mut is_training = false;
                let mut button_states = [ButtonState {
                    shown: false,
                    matches_entity_state: false,
                    matches_cursor_action: false,
                }; NUM_BUTTONS];
                if let EntityState::TrainingUnit(trained_entity_type) = selected_entity.state {
                    is_training = true;
                    let training = selected_entity.training.as_ref().unwrap();
                    let progress = training.progress().unwrap();
                    let training_status = format!("Training {:?}", trained_entity_type);
                    self.draw_text(ctx, [x, training_status_y], training_status, medium_font)?;
                    let progress_w = 20.0;
                    let progress_bar = format!(
                        "[{}{}]",
                        "=".repeat((progress * progress_w) as usize),
                        " ".repeat(((1.0 - progress) * progress_w) as usize)
                    );
                    self.draw_text(ctx, [x, progress_y], progress_bar, medium_font)?;
                }

                if !is_training {
                    let hovered_button_i = self
                        .buttons
                        .iter()
                        .position(|button| button.rect.contains(mouse_position));
                    let mut tooltip_text = String::new();
                    for (i, action) in selected_entity.actions.iter().enumerate() {
                        if let Some(action) = action {
                            button_states[i].shown = true;
                            match action {
                                Action::Train(trained_entity_type, training_config) => {
                                    if selected_entity.state
                                        == EntityState::TrainingUnit(*trained_entity_type)
                                    {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    if hovered_button_i == Some(i) {
                                        tooltip_text = format!(
                                            "Train {:?} [cost {}, {}s]",
                                            trained_entity_type,
                                            training_config.cost,
                                            training_config.duration.as_secs()
                                        );
                                    }
                                }
                                Action::Move => {
                                    if selected_entity.state == EntityState::Moving {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    const TEXT: &str = "Move";
                                    if cursor_action == CursorAction::IssueMovement {
                                        button_states[i].matches_cursor_action = true;
                                        tooltip_text = TEXT.to_string();
                                    }
                                    if hovered_button_i == Some(i) {
                                        tooltip_text = TEXT.to_string();
                                    }
                                }
                                Action::Heal => {
                                    if hovered_button_i == Some(i) {
                                        tooltip_text = "Heal".to_string();
                                    }
                                }
                                Action::Attack => {
                                    if selected_entity.state == EntityState::Attacking {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    const TEXT: &str = "Attack";
                                    if cursor_action == CursorAction::Attack {
                                        button_states[i].matches_cursor_action = true;
                                        tooltip_text = TEXT.to_string();
                                    }
                                    if hovered_button_i == Some(i) {
                                        tooltip_text = TEXT.to_string();
                                    }
                                }
                            }
                        }
                    }

                    for (button_i, button) in self.buttons.iter().enumerate() {
                        button.draw(ctx, button_states[button_i])?;
                    }
                    if !tooltip_text.is_empty() {
                        self.draw_text(ctx, [x, tooltip_y], tooltip_text, medium_font)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn on_mouse_click(
        &self,
        mouse_position: [f32; 2],
        selected_player_entity: &Entity,
    ) -> Option<Action> {
        for (button_i, button) in self.buttons.iter().enumerate() {
            if button.rect.contains(mouse_position) {
                return selected_player_entity.actions[button_i];
            }
        }

        None
    }

    pub fn on_button_click(
        &self,
        keycode: KeyCode,
        selected_player_entity: &Entity,
    ) -> Option<Action> {
        if keycode == KeyCode::C {
            return selected_player_entity.actions[0];
        }
        if keycode == KeyCode::V {
            return selected_player_entity.actions[1];
        }
        if keycode == KeyCode::B {
            return selected_player_entity.actions[2];
        }
        None
    }

    fn draw_text(
        &self,
        ctx: &mut Context,
        position: [f32; 2],
        line: impl Into<String>,
        font_size: f32,
    ) -> GameResult {
        let text = Text::new((line.into(), self.font, font_size));
        graphics::draw(
            ctx,
            &text,
            DrawParam::new().dest([
                self.position_on_screen[0] + position[0],
                self.position_on_screen[1] + position[1],
            ]),
        )
    }
}

#[derive(Copy, Clone)]
struct ButtonState {
    shown: bool,
    matches_entity_state: bool,
    matches_cursor_action: bool,
}

pub struct MinimapGraphics {
    border_mesh: Mesh,
    camera_mesh: Mesh,
    camera_scale: [f32; 2],
    rect: Rect,
}

impl MinimapGraphics {
    pub fn new(
        ctx: &mut Context,
        position: [f32; 2],
        map_dimensions: [u32; 2],
    ) -> GameResult<Self> {
        let cell_pixel_size_in_minimap = 8.0;

        let rect = Rect::new(
            position[0],
            position[1],
            map_dimensions[0] as f32 * cell_pixel_size_in_minimap,
            map_dimensions[1] as f32 * cell_pixel_size_in_minimap,
        );

        let border_mesh = MeshBuilder::new()
            .rectangle(DrawMode::stroke(2.0), rect, Color::new(1.0, 1.0, 1.0, 1.0))?
            .build(ctx)?;

        let camera_scale = [
            cell_pixel_size_in_minimap / CELL_PIXEL_SIZE[0],
            cell_pixel_size_in_minimap / CELL_PIXEL_SIZE[1],
        ];
        let camera_mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(1.0),
                Rect::new(
                    position[0],
                    position[1],
                    WORLD_VIEWPORT.w * camera_scale[0],
                    WORLD_VIEWPORT.h * camera_scale[1],
                ),
                Color::new(1.0, 1.0, 1.0, 1.0),
            )?
            .build(ctx)?;

        Ok(Self {
            border_mesh,
            camera_mesh,
            camera_scale,
            rect,
        })
    }

    pub fn draw(&self, ctx: &mut Context, camera_position_in_world: [f32; 2]) -> GameResult {
        ggez::graphics::draw(ctx, &self.border_mesh, DrawParam::default())?;
        ggez::graphics::draw(
            ctx,
            &self.camera_mesh,
            DrawParam::default().dest([
                camera_position_in_world[0] * self.camera_scale[0],
                camera_position_in_world[1] * self.camera_scale[1],
            ]),
        )?;
        Ok(())
    }

    pub fn rect(&self) -> &Rect {
        &self.rect
    }
}

pub struct Button {
    rect: Rect,
    border: Mesh,
    highlight_entity_state: Mesh,
    highlight_cursor_action: Mesh,
    text: Text,
}

impl Button {
    fn new(ctx: &mut Context, rect: Rect, text: &str, font: Font) -> GameResult<Button> {
        let border = MeshBuilder::new()
            .rectangle(DrawMode::stroke(1.0), rect, Color::new(0.7, 0.7, 0.7, 1.0))?
            .build(ctx)?;
        let highlight_entity_state = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(3.0),
                Rect::new(rect.x + 2.0, rect.y + 2.0, rect.w - 4.0, rect.h - 4.0),
                Color::new(0.4, 0.95, 0.4, 1.0),
            )?
            .build(ctx)?;
        let highlight_cursor_action = MeshBuilder::new()
            .rectangle(DrawMode::fill(), rect, Color::new(1.0, 1.0, 0.6, 0.05))?
            .build(ctx)?;
        let text = Text::new((text, font, 40.0));
        Ok(Self {
            rect,
            border,
            highlight_entity_state,
            highlight_cursor_action,
            text,
        })
    }

    fn draw(&self, ctx: &mut Context, state: ButtonState) -> GameResult {
        // TODO draw it differently if hovered
        self.border.draw(ctx, DrawParam::default())?;
        if state.shown {
            self.text.draw(
                ctx,
                DrawParam::default().dest([self.rect.x + 30.0, self.rect.y + 20.0]),
            )?;
        }
        if state.matches_entity_state {
            self.highlight_entity_state
                .draw(ctx, DrawParam::default())?;
        }
        if state.matches_cursor_action {
            self.highlight_cursor_action
                .draw(ctx, DrawParam::default())?;
        }
        Ok(())
    }
}
