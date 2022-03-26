use ggez::graphics::{
    self, Color, DrawMode, DrawParam, Drawable, Font, Mesh, MeshBuilder, Rect, Text,
};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};

use crate::entities::{ActionType, Entity, Team};
use crate::game::{TeamState, CAMERA_SIZE, CELL_PIXEL_SIZE};

const NUM_BUTTONS: usize = 2;

pub struct HudGraphics {
    position_on_screen: [f32; 2],
    font: Font,
    buttons: [Button; NUM_BUTTONS],
}

impl HudGraphics {
    pub fn new(ctx: &mut Context, position: [f32; 2], font: Font) -> GameResult<Self> {
        let w = 80.0;
        let button_1_rect = Rect::new(position[0] + 5.0, position[1] + 270.0, w, w);
        let button_1 = Button::new(ctx, button_1_rect, "V", font)?;
        let button_margin = 5.0;
        let button_2_rect = Rect::new(button_1_rect.x + w + button_margin, button_1_rect.y, w, w);
        let button_2 = Button::new(ctx, button_2_rect, "B", font)?;
        let buttons = [button_1, button_2];
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
    ) -> GameResult {
        let x = 0.0;
        let resources_y = 5.0;
        let name_y = 48.0;
        let health_y = 130.0;
        let action_y = 180.0;

        let small_font = 30.0;
        let large_font = 40.0;

        self.draw_text(
            ctx,
            [x, resources_y],
            format!("Resources: {}", player_team_state.resources),
            small_font,
        )?;

        if let Some(selected_entity) = selected_entity {
            self.draw_text(ctx, [x, name_y], selected_entity.name, large_font)?;

            if let Some(health) = &selected_entity.health {
                let health = format!(
                    "HP: [{}{}]",
                    "=".repeat(health.current as usize),
                    " ".repeat((health.max - health.current) as usize)
                );
                self.draw_text(ctx, [x, health_y], health, small_font)?;
            }

            if selected_entity.team == Team::Player {
                let mut actions = [false; NUM_BUTTONS];
                if let Some(training_action) = &selected_entity.training_action {
                    actions[0] = true;
                    if let Some(progress) = training_action.progress() {
                        self.draw_text(ctx, [x, action_y], "Training in progress", small_font)?;
                        let progress_w = 20.0;
                        let progress_bar = format!(
                            "[{}{}]",
                            "=".repeat((progress * progress_w) as usize),
                            " ".repeat(((1.0 - progress) * progress_w) as usize)
                        );
                        self.draw_text(ctx, [x, action_y + 35.0], progress_bar, small_font)?;
                    } else {
                        self.draw_text(
                            ctx,
                            [x, action_y],
                            "Press [V] to train a unit",
                            small_font,
                        )?;
                    }
                }
                let mut action_text = String::new();
                for (action_i, action) in selected_entity.instant_actions.iter().enumerate() {
                    if let Some(action_type) = action {
                        actions[action_i] = true;
                        match action_type {
                            ActionType::Heal => action_text.push_str("Heal "),
                            ActionType::SelfHarm => action_text.push_str("Self-harm "),
                            _ => panic!("Unhandled action: {:?}", action_type),
                        }
                    }
                }
                self.draw_text(ctx, [x, action_y], action_text, small_font)?;

                for (button_i, button) in self.buttons.iter().enumerate() {
                    button.draw(ctx, actions[button_i])?;
                }
            }
        }

        Ok(())
    }

    pub fn on_mouse_click(
        &self,
        mouse_position: [f32; 2],
        selected_player_entity: &Entity,
    ) -> Option<ActionType> {
        for (button_i, button) in self.buttons.iter().enumerate() {
            if button.rect.contains(mouse_position) {
                if button_i == 0 {
                    if let Some(training_action) = &selected_player_entity.training_action {
                        return Some(ActionType::Train(training_action.trained_entity_type));
                    }
                }
                return selected_player_entity.instant_actions[button_i];
            }
        }

        None
    }

    pub fn on_button_click(
        &self,
        keycode: KeyCode,
        selected_player_entity: &Entity,
    ) -> Option<ActionType> {
        if keycode == KeyCode::V {
            if let Some(training_action) = &selected_player_entity.training_action {
                return Some(ActionType::Train(training_action.trained_entity_type));
            }
            return selected_player_entity.instant_actions[0];
        }
        if keycode == KeyCode::B {
            return selected_player_entity.instant_actions[1];
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
                    CAMERA_SIZE[0] * camera_scale[0],
                    CAMERA_SIZE[1] * camera_scale[1],
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
    text: Text,
}

impl Button {
    fn new(ctx: &mut Context, rect: Rect, text: &str, font: Font) -> GameResult<Button> {
        let border = MeshBuilder::new()
            .rectangle(DrawMode::stroke(1.0), rect, Color::new(1.0, 1.0, 1.0, 1.0))?
            .build(ctx)?;
        let text = Text::new((text, font, 40.0));
        Ok(Self { rect, border, text })
    }

    fn draw(&self, ctx: &mut Context, draw_action: bool) -> GameResult {
        // TODO draw it differently if hovered
        self.border.draw(ctx, DrawParam::default())?;
        if draw_action {
            self.text.draw(
                ctx,
                DrawParam::default().dest([self.rect.x + 30.0, self.rect.y + 20.0]),
            )?;
        }
        Ok(())
    }
}
