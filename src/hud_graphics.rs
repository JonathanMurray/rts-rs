use std::collections::HashMap;
use std::time::Duration;

use ggez::graphics::{
    self, Color, DrawMode, DrawParam, Drawable, Font, Mesh, MeshBuilder, Rect, Text,
};
use ggez::input::keyboard::KeyCode;
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use crate::core::TeamState;
use crate::data::EntityType;
use crate::entities::{Action, Entity, EntityState, PhysicalType, Team, NUM_ENTITY_ACTIONS};
use crate::game::{CursorAction, PlayerState, CELL_PIXEL_SIZE, WORLD_VIEWPORT};

const NUM_BUTTONS: usize = NUM_ENTITY_ACTIONS;

pub struct HudGraphics {
    position_on_screen: [f32; 2],
    font: Font,
    buttons: [Button; NUM_BUTTONS],
    minimap: Minimap,
    hovered_button_index: Option<usize>,
    entity_actions: [Option<Action>; NUM_ENTITY_ACTIONS],
    keycode_labels: HashMap<KeyCode, Text>,
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
        let button_1 = Button::new(ctx, button_1_rect)?;
        let button_margin = 5.0;
        let button_2_rect = Rect::new(button_1_rect.x + w + button_margin, button_1_rect.y, w, w);
        let button_2 = Button::new(ctx, button_2_rect)?;
        let button_3_rect = Rect::new(button_2_rect.x + w + button_margin, button_1_rect.y, w, w);
        let button_3 = Button::new(ctx, button_3_rect)?;
        let buttons = [button_1, button_2, button_3];

        let minimap_pos = [900.0, position[1] + 100.0];
        let minimap = Minimap::new(ctx, minimap_pos, world_dimensions)?;

        let keycode_labels = create_keycode_labels(font);

        Ok(Self {
            position_on_screen: position,
            font,
            buttons,
            minimap,
            hovered_button_index: None,
            entity_actions: [None; NUM_ENTITY_ACTIONS],
            keycode_labels,
        })
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        player_team_state: &TeamState,
        selected_entity: Option<&Entity>,
        player_state: &PlayerState,
    ) -> GameResult {
        let x = 0.0;
        let name_y = 48.0;
        let health_y = 130.0;
        let resource_status_y = 200.0;
        let training_status_y = 240.0;
        let progress_y = 290.0;
        let tooltip_y = 380.0;

        let small_font = 20.0;
        let medium_font = 30.0;
        let large_font = 40.0;

        let cursor_action = &player_state.cursor_action;

        let resources_text = Text::new((
            format!("RESOURCES: {}", player_team_state.resources),
            self.font,
            medium_font,
        ));
        resources_text.draw(ctx, DrawParam::new().dest([1200.0, 15.0]))?;

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
                    text: None,
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
                if let PhysicalType::Unit(unit) = &selected_entity.physical_type {
                    if let Some(gathering) = unit.gathering.as_ref() {
                        if gathering.carries_resource() {
                            self.draw_text(
                                ctx,
                                [x, resource_status_y],
                                "[HAS RESOURCE]",
                                medium_font,
                            )?;
                        }
                    }
                }

                if !is_training {
                    let mut tooltip_text = String::new();
                    for (i, action) in selected_entity.actions.iter().enumerate() {
                        if let Some(action) = action {
                            button_states[i].text = Some(self.action_label(action));
                            match action {
                                Action::Train(trained_entity_type, training_config) => {
                                    if selected_entity.state
                                        == EntityState::TrainingUnit(*trained_entity_type)
                                    {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    if self.hovered_button_index == Some(i) {
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
                                    if cursor_action
                                        == &CursorAction::PlaceStructure(*structure_type)
                                    {
                                        button_states[i].matches_cursor_action = true;
                                        tooltip_text = format!("Construct {:?}", structure_type);
                                    }
                                    if self.hovered_button_index == Some(i) {
                                        tooltip_text = format!("Construct {:?}", structure_type);
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
                                    if self.hovered_button_index == Some(i) {
                                        tooltip_text = TEXT.to_string();
                                    }
                                }
                                Action::Heal => {
                                    if self.hovered_button_index == Some(i) {
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
                                    if self.hovered_button_index == Some(i) {
                                        tooltip_text = TEXT.to_string();
                                    }
                                }
                                Action::GatherResource => {
                                    if let EntityState::Gathering(..) = selected_entity.state {
                                        button_states[i].matches_entity_state = true;
                                    }
                                    const TEXT: &str = "Gather";
                                    if cursor_action == &CursorAction::SelectResourceTarget {
                                        button_states[i].matches_cursor_action = true;
                                        tooltip_text = TEXT.to_string();
                                    }
                                    if self.hovered_button_index == Some(i) {
                                        tooltip_text = TEXT.to_string();
                                    }
                                }
                            }
                        }
                    }

                    let mut button_states = button_states.into_iter();
                    for (button_i, button) in self.buttons.iter().enumerate() {
                        let is_hovered = self.hovered_button_index == Some(button_i);
                        let button_state = button_states.next().unwrap();
                        button.draw(ctx, button_state, is_hovered)?;
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

    fn action_label(&self, action: &Action) -> &Text {
        let keycode = action_keycode(action);
        self.keycode_labels
            .get(&keycode)
            .unwrap_or_else(|| panic!("No button label for action with keycode: {:?}", keycode))
    }

    pub fn on_mouse_button_down(
        &mut self,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> Option<PlayerInput> {
        for (i, button) in self.buttons.iter_mut().enumerate() {
            if button.rect.contains([x, y]) {
                button.on_click();
                return Some(PlayerInput::UseEntityAction(i));
            }
        }

        self.minimap
            .on_mouse_button_down(button, x, y)
            .map(PlayerInput::SetCameraPositionRelativeToWorldDimension)
    }

    pub fn on_mouse_motion(&mut self, x: f32, y: f32) -> Option<PlayerInput> {
        self.hovered_button_index = self
            .buttons
            .iter()
            .position(|button| button.rect.contains([x, y]));

        self.minimap
            .on_mouse_motion(x, y)
            .map(PlayerInput::SetCameraPositionRelativeToWorldDimension)
    }

    pub fn on_mouse_button_up(&mut self, button: MouseButton) {
        self.minimap.on_mouse_button_up(button);
    }

    pub fn on_key_down(&self, keycode: KeyCode) -> Option<PlayerInput> {
        for (i, action) in self.entity_actions.iter().enumerate() {
            if let Some(action) = action {
                if action_keycode(action) == keycode {
                    return Some(PlayerInput::UseEntityAction(i));
                }
            }
        }
        None
    }

    pub fn update(&mut self, dt: Duration) {
        for button in &mut self.buttons {
            button.update(dt);
        }
    }

    pub fn set_entity_actions(&mut self, actions: [Option<Action>; NUM_ENTITY_ACTIONS]) {
        self.entity_actions = actions;
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
struct ButtonState<'a> {
    text: Option<&'a Text>,
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
    highlight: Mesh,
    is_down: bool,
    cooldown: Duration,
}

impl Button {
    fn new(ctx: &mut Context, rect: Rect) -> GameResult<Button> {
        let local_rect = Rect::new(0.0, 0.0, rect.w, rect.h);
        let border = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(1.0),
                local_rect,
                Color::new(0.7, 0.7, 0.7, 1.0),
            )?
            .build(ctx)?;
        let highlight_entity_state = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(3.0),
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
            rect,
            border,
            highlight_entity_state,
            highlight,
            is_down: false,
            cooldown: Duration::ZERO,
        })
    }

    fn draw(&self, ctx: &mut Context, state: ButtonState, is_hovered: bool) -> GameResult {
        self.border
            .draw(ctx, DrawParam::default().dest(self.rect.point()))?;
        if state.matches_entity_state {
            self.highlight_entity_state
                .draw(ctx, DrawParam::default().dest(self.rect.point()))?;
        }

        let offset = if self.is_down { [4.0, 4.0] } else { [0.0, 0.0] };
        let scale = if self.is_down { [0.9, 0.9] } else { [1.0, 1.0] };
        if let Some(text) = state.text {
            text.draw(
                ctx,
                DrawParam::default()
                    .dest([
                        self.rect.x + 30.0 + offset[0],
                        self.rect.y + 20.0 + offset[1],
                    ])
                    .scale(scale),
            )?;
        }
        if state.matches_cursor_action || is_hovered {
            self.highlight.draw(
                ctx,
                DrawParam::default()
                    .dest([self.rect.x + offset[0], self.rect.y + offset[1]])
                    .scale(scale),
            )?;
        }
        Ok(())
    }

    fn update(&mut self, dt: Duration) {
        if self.is_down {
            self.cooldown = self.cooldown.checked_sub(dt).unwrap_or(Duration::ZERO);
            if self.cooldown.is_zero() {
                self.is_down = false;
            }
        }
    }

    fn on_click(&mut self) {
        self.is_down = true;
        self.cooldown = Duration::from_millis(100);
    }
}

#[derive(Debug)]
pub enum PlayerInput {
    UseEntityAction(usize),
    SetCameraPositionRelativeToWorldDimension([f32; 2]),
}

fn action_keycode(action: &Action) -> KeyCode {
    match action {
        Action::Train(EntityType::CircleUnit, _) => KeyCode::C,
        Action::Train(EntityType::SquareUnit, _) => KeyCode::S,
        Action::Train(unit_type, _) => panic!("No keycode for training {:?}", unit_type),
        Action::Construct(EntityType::SmallBuilding) => KeyCode::S,
        Action::Construct(EntityType::LargeBuilding) => KeyCode::L,
        Action::Construct(structure_type) => {
            panic!("No keycode for constructing {:?}", structure_type)
        }
        Action::Move => KeyCode::M,
        Action::Heal => KeyCode::H,
        Action::Attack => KeyCode::A,
        Action::GatherResource => KeyCode::G,
    }
}

fn create_keycode_labels(font: Font) -> HashMap<KeyCode, Text> {
    [
        (KeyCode::A, "A"),
        (KeyCode::C, "C"),
        (KeyCode::G, "G"),
        (KeyCode::H, "H"),
        (KeyCode::L, "L"),
        (KeyCode::M, "M"),
        (KeyCode::S, "S"),
    ]
    .map(|(keycode, text)| (keycode, Text::new((text, font, 30.0))))
    .into()
}
