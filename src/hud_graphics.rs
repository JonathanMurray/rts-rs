use ggez::graphics::{
    self, Color, DrawMode, DrawParam, Drawable, Font, Mesh, MeshBuilder, Rect, Text,
};
use ggez::input::keyboard::KeyCode;
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use crate::core::TeamState;
use crate::entities::{Action, Entity, EntityState, Team, NUM_ENTITY_ACTIONS};
use crate::game::{CursorAction, PlayerState, CELL_PIXEL_SIZE, WORLD_VIEWPORT};

const NUM_BUTTONS: usize = NUM_ENTITY_ACTIONS;

pub struct HudGraphics {
    position_on_screen: [f32; 2],
    font: Font,
    buttons: [Button; NUM_BUTTONS],
    minimap: Minimap,
}

impl HudGraphics {
    pub fn new(
        ctx: &mut Context,
        position: [f32; 2],
        font: Font,
        world_dimensions: [u32; 2],
    ) -> GameResult<Self> {
        let w = 80.0;
        let button_1_rect = Rect::new(position[0] + 5.0, position[1] + 270.0, w, w);
        let button_1 = Button::new(ctx, button_1_rect, "C", font)?;
        let button_margin = 5.0;
        let button_2_rect = Rect::new(button_1_rect.x + w + button_margin, button_1_rect.y, w, w);
        let button_2 = Button::new(ctx, button_2_rect, "V", font)?;
        let button_3_rect = Rect::new(button_2_rect.x + w + button_margin, button_1_rect.y, w, w);
        let button_3 = Button::new(ctx, button_3_rect, "B", font)?;
        let buttons = [button_1, button_2, button_3];

        let minimap_pos = [900.0, position[1] + 100.0];
        let minimap = Minimap::new(ctx, minimap_pos, world_dimensions)?;

        Ok(Self {
            position_on_screen: position,
            font,
            buttons,
            minimap,
        })
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        player_team_state: &TeamState,
        selected_entity: Option<&Entity>,
        player_state: &PlayerState,
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

        let cursor_action = &player_state.cursor_action;

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
                    let progress = training.progress(trained_entity_type).unwrap();
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
                                Action::Construct(structure_type) => {
                                    if selected_entity.state
                                        == EntityState::Constructing(*structure_type)
                                    {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    if hovered_button_i == Some(i) {
                                        tooltip_text = format!("Construct {:?}", structure_type,);
                                    }
                                }
                                Action::Move => {
                                    if selected_entity.state == EntityState::Moving {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    const TEXT: &str = "Move";
                                    if cursor_action == &CursorAction::SelectMovementDestination {
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
                                    if let EntityState::Attacking(_) = selected_entity.state {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    const TEXT: &str = "Attack";
                                    if cursor_action == &CursorAction::SelectAttackTarget {
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

        self.minimap
            .draw(ctx, player_state.camera.position_in_world)?;

        Ok(())
    }

    pub fn on_mouse_button_down(
        &mut self,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> Option<PlayerInput> {
        for (i, button) in self.buttons.iter().enumerate() {
            if button.rect.contains([x, y]) {
                return Some(PlayerInput::UseEntityAction(i));
            }
        }

        self.minimap
            .on_mouse_button_down(button, x, y)
            .map(PlayerInput::SetCameraPositionRelativeToWorldDimension)
    }

    pub fn on_mouse_motion(&mut self, x: f32, y: f32) -> Option<PlayerInput> {
        self.minimap
            .on_mouse_motion(x, y)
            .map(PlayerInput::SetCameraPositionRelativeToWorldDimension)
    }

    pub fn on_mouse_button_up(&mut self, button: MouseButton) {
        self.minimap.on_mouse_button_up(button);
    }

    pub fn on_key_down(&self, keycode: KeyCode) -> Option<PlayerInput> {
        if keycode == KeyCode::C {
            return Some(PlayerInput::UseEntityAction(0));
        }
        if keycode == KeyCode::V {
            return Some(PlayerInput::UseEntityAction(1));
        }
        if keycode == KeyCode::B {
            return Some(PlayerInput::UseEntityAction(2));
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

struct Minimap {
    border_mesh: Mesh,
    camera_mesh: Mesh,
    camera_scale: [f32; 2],
    rect: Rect,
    is_mouse_dragging: bool,
}

impl Minimap {
    fn new(ctx: &mut Context, position: [f32; 2], world_dimensions: [u32; 2]) -> GameResult<Self> {
        let cell_pixel_size_in_minimap = 8.0;

        let rect = Rect::new(
            position[0],
            position[1],
            world_dimensions[0] as f32 * cell_pixel_size_in_minimap,
            world_dimensions[1] as f32 * cell_pixel_size_in_minimap,
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
            is_mouse_dragging: false,
        })
    }

    fn draw(&self, ctx: &mut Context, camera_position_in_world: [f32; 2]) -> GameResult {
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

    fn on_mouse_button_down(&mut self, button: MouseButton, x: f32, y: f32) -> Option<[f32; 2]> {
        if button == MouseButton::Left && self.rect.contains([x, y]) {
            self.is_mouse_dragging = true;
            Some(clamped_ratio(x, y, &self.rect))
        } else {
            None
        }
    }

    fn on_mouse_motion(&mut self, x: f32, y: f32) -> Option<[f32; 2]> {
        if self.is_mouse_dragging {
            Some(clamped_ratio(x, y, &self.rect))
        } else {
            None
        }
    }

    fn on_mouse_button_up(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.is_mouse_dragging = false;
        }
    }
}

fn clamped_ratio(x: f32, y: f32, rect: &Rect) -> [f32; 2] {
    let x_ratio = if x < rect.x {
        0.0
    } else if x > rect.right() {
        1.0
    } else {
        (x - rect.x) / rect.w
    };
    let y_ratio = if y < rect.y {
        0.0
    } else if y > rect.bottom() {
        1.0
    } else {
        (y - rect.y) / rect.h
    };
    [x_ratio, y_ratio]
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

#[derive(Debug)]
pub enum PlayerInput {
    UseEntityAction(usize),
    SetCameraPositionRelativeToWorldDimension([f32; 2]),
}
