use std::cell::Ref;
use std::collections::HashMap;
use std::convert::TryInto;
use std::time::Duration;

use ggez::graphics::{
    self, Color, DrawMode, DrawParam, Drawable, Font, Mesh, MeshBuilder, Rect, Text,
};
use ggez::input::keyboard::KeyCode;
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use crate::core::TeamState;
use crate::data::EntityType;
use crate::entities::{
    Action, Entity, EntityState, PhysicalType, Team, TrainingConfig, NUM_ENTITY_ACTIONS,
};
use crate::game::{CursorState, PlayerState, CELL_PIXEL_SIZE, WORLD_VIEWPORT};

const NUM_BUTTONS: usize = NUM_ENTITY_ACTIONS;

pub struct HudGraphics {
    position_on_screen: [f32; 2],
    font: Font,
    buttons: [Button; NUM_BUTTONS],
    minimap: Minimap,
    hovered_button_index: Option<usize>,
    keycode_labels: HashMap<KeyCode, Text>,
    tooltip: Tooltip,
}

impl HudGraphics {
    pub fn new(
        ctx: &mut Context,
        position: [f32; 2],
        font: Font,
        world_dimensions: [u32; 2],
    ) -> GameResult<Self> {
        let w = 80.0;

        let mut buttons = vec![];
        let button_margin = 5.0;
        let buttons_per_row = 3;
        for i in 0..NUM_BUTTONS {
            let x = position[0] + 5.0 + (i % buttons_per_row) as f32 * (w + button_margin);
            let y = position[1] + 240.0 + (i / buttons_per_row) as f32 * (w + button_margin);
            let rect = Rect::new(x, y, w, w);
            buttons.push(Button::new(ctx, rect)?);
        }
        let buttons = buttons.try_into().unwrap();

        let minimap_pos = [900.0, position[1] + 30.0];
        let minimap = Minimap::new(ctx, minimap_pos, world_dimensions)?;

        let keycode_labels = create_keycode_labels(font);

        let tooltip = Tooltip::new(font, [position[0], position[1] + 420.0]);

        Ok(Self {
            position_on_screen: position,
            font,
            buttons,
            minimap,
            hovered_button_index: None,
            keycode_labels,
            tooltip,
        })
    }

    pub fn draw<'a>(
        &self,
        ctx: &mut Context,
        player_team_state: Ref<TeamState>,
        selected_entities: Vec<Ref<'a, Entity>>,
        num_selected_entities: usize,
        player_state: &PlayerState,
    ) -> GameResult {
        let x = 0.0;

        let small_font = 20.0;
        let medium_font = 30.0;
        let large_font = 40.0;

        let cursor_state = player_state.cursor_state();

        let resources_text = Text::new((
            format!("RESOURCES: {}", player_team_state.resources),
            self.font,
            medium_font,
        ));
        resources_text.draw(ctx, DrawParam::new().dest([1200.0, 15.0]))?;

        let name_y = 28.0;
        let health_y = 110.0;
        let resource_status_y = 180.0;
        let training_status_y = 220.0;
        let progress_y = 270.0;

        if num_selected_entities == 0 {
            let y = 28.0;
            self.draw_text(ctx, [x, y], "[nothing selected]", large_font)?;
        } else if num_selected_entities > 1 {
            let mut y = 28.0;
            for entity in &selected_entities {
                self.draw_text(ctx, [x, y], entity.name, large_font)?;
                y += 50.0;
            }
        } else if num_selected_entities == 1 {
            let selected_entity = selected_entities.first().unwrap();
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
                if let EntityState::TrainingUnit(trained_entity_type) = selected_entity.state {
                    // TODO: Use some other way of determining when to hide buttons
                    //is_training = true;
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
                        if gathering.is_carrying() {
                            self.draw_text(
                                ctx,
                                [x, resource_status_y],
                                "[HAS RESOURCE]",
                                medium_font,
                            )?;
                        }
                    }
                }
            }
        }

        for (button_i, button) in self.buttons.iter().enumerate() {
            let is_hovered = self.hovered_button_index == Some(button_i);
            let matches_entity_state = button
                .action
                .map(|action| {
                    selected_entities
                        .iter()
                        .any(|e| state_matches_action(e.state, action))
                })
                .unwrap_or(false);
            button.draw(ctx, is_hovered, cursor_state, matches_entity_state)?;
        }

        let tooltip_text = match cursor_state {
            CursorState::Default => {
                if let Some(index) = self.hovered_button_index {
                    match self.buttons[index].action {
                        Some(Action::Attack) => TooltipText::ActionAttack,
                        Some(Action::Move) => TooltipText::ActionMove,
                        Some(Action::Construct(structure_type)) => {
                            TooltipText::ActionConstruct(structure_type)
                        }
                        Some(Action::GatherResource) => TooltipText::ActionGather,
                        Some(Action::ReturnResource) => TooltipText::ActionReturnResource,
                        Some(Action::Train(unit_type, config)) => {
                            TooltipText::ActionTrain(unit_type, config)
                        }
                        None => TooltipText::None,
                    }
                } else {
                    TooltipText::None
                }
            }
            CursorState::SelectingAttackTarget => TooltipText::CursorSelectAttackTarget,
            CursorState::SelectingMovementDestination => {
                TooltipText::CursorSelectMovementDestination
            }
            CursorState::PlacingStructure(_) => TooltipText::CursorPlaceStructure,
            CursorState::SelectingResourceTarget => TooltipText::CursorSelectResource,
            CursorState::DraggingSelectionArea(_) => TooltipText::None,
        };
        self.tooltip.draw(ctx, tooltip_text)?;

        self.minimap
            .draw(ctx, player_state.camera_position_in_world())?;

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
        for button in &mut self.buttons {
            if button.rect.contains([x, y]) {
                if let Some(input) = button.on_click() {
                    return Some(input);
                }
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
        for action in self.buttons.iter().filter_map(|b| b.action) {
            if action_keycode(&action) == keycode {
                return Some(PlayerInput::UseEntityAction(action));
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
        for (i, action) in actions.iter().enumerate() {
            self.buttons[i].action = *action;
            self.buttons[i].text = action.map(|action| self.action_label(&action).clone());
        }
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

fn state_matches_action(state: EntityState, action: Action) -> bool {
    match action {
        Action::Train(trained_entity_type, _) => {
            state == EntityState::TrainingUnit(trained_entity_type)
        }
        Action::Construct(structure_type) => {
            if let EntityState::Constructing(constructing_type, _) = state {
                structure_type == constructing_type
            } else {
                false
            }
        }
        Action::Move => state == EntityState::Moving,
        Action::Attack => {
            matches!(state, EntityState::Attacking(_))
        }
        Action::GatherResource => {
            matches!(state, EntityState::GatheringResource(_))
        }
        Action::ReturnResource => {
            matches!(state, EntityState::ReturningResource(..))
        }
    }
}

const TOOLTIP_FONT_SIZE: f32 = 30.0;

struct Tooltip {
    position: [f32; 2],
    font: Font,
    text_attack: Text,
    text_move: Text,
    text_gather: Text,
    text_return: Text,
    text_select_attack_target: Text,
    text_select_movement_destination: Text,
    text_place_structure: Text,
    text_select_resource: Text,
}

impl Tooltip {
    fn new(font: Font, position: [f32; 2]) -> Self {
        let text = |t| Text::new((t, font, TOOLTIP_FONT_SIZE));

        Self {
            position,
            font,
            text_attack: text("Attack"),
            text_move: text("Move"),
            text_gather: text("Gather"),
            text_return: text("Return"),
            text_select_attack_target: text("Select attack target"),
            text_select_movement_destination: text("Select destination"),
            text_place_structure: text("Place structure"),
            text_select_resource: text("Select resource to gather"),
        }
    }

    fn draw(&self, ctx: &mut Context, text: TooltipText) -> GameResult {
        let param = DrawParam::default().dest(self.position);
        match text {
            TooltipText::None => {}
            TooltipText::ActionAttack => self.text_attack.draw(ctx, param)?,
            TooltipText::ActionMove => self.text_move.draw(ctx, param)?,
            TooltipText::ActionGather => self.text_gather.draw(ctx, param)?,
            TooltipText::ActionReturnResource => self.text_return.draw(ctx, param)?,
            TooltipText::ActionTrain(trained_entity_type, training_config) => {
                let text = format!(
                    "Train {:?} [cost {}, {}s]",
                    trained_entity_type,
                    training_config.cost,
                    training_config.duration.as_secs()
                );
                Text::new((text, self.font, TOOLTIP_FONT_SIZE)).draw(ctx, param)?;
            }
            TooltipText::ActionConstruct(structure_type) => {
                let text = format!("Construct {:?}", structure_type,);
                Text::new((text, self.font, TOOLTIP_FONT_SIZE)).draw(ctx, param)?;
            }
            TooltipText::CursorSelectAttackTarget => {
                self.text_select_attack_target.draw(ctx, param)?
            }
            TooltipText::CursorSelectMovementDestination => {
                self.text_select_movement_destination.draw(ctx, param)?
            }
            TooltipText::CursorPlaceStructure => self.text_place_structure.draw(ctx, param)?,
            TooltipText::CursorSelectResource => self.text_select_resource.draw(ctx, param)?,
        };
        Ok(())
    }
}

enum TooltipText {
    None,
    ActionAttack,
    ActionMove,
    ActionGather,
    ActionReturnResource,
    ActionTrain(EntityType, TrainingConfig),
    ActionConstruct(EntityType),
    CursorSelectAttackTarget,
    CursorSelectMovementDestination,
    CursorPlaceStructure,
    CursorSelectResource,
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
        let minimap_width = 300.0;
        let aspect_ratio = world_dimensions[0] as f32 / world_dimensions[1] as f32;
        let rect = Rect::new(
            position[0],
            position[1],
            minimap_width,
            minimap_width / aspect_ratio,
        );

        let border_mesh = MeshBuilder::new()
            .rectangle(DrawMode::stroke(2.0), rect, Color::new(1.0, 1.0, 1.0, 1.0))?
            .build(ctx)?;

        let camera_scale = [
            minimap_width / world_dimensions[0] as f32 / CELL_PIXEL_SIZE[0],
            minimap_width / world_dimensions[0] as f32 / CELL_PIXEL_SIZE[1],
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

#[derive(Debug)]
pub struct Button {
    rect: Rect,
    border: Mesh,
    highlight_entity_state: Mesh,
    highlight: Mesh,
    is_down: bool,
    down_cooldown: Duration,
    action: Option<Action>,
    text: Option<Text>,
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
            down_cooldown: Duration::ZERO,
            action: None,
            text: None,
        })
    }

    fn draw(
        &self,
        ctx: &mut Context,
        is_hovered: bool,
        cursor_state: CursorState,
        matches_entity_state: bool,
    ) -> GameResult {
        self.border
            .draw(ctx, DrawParam::default().dest(self.rect.point()))?;
        if matches_entity_state {
            self.highlight_entity_state
                .draw(ctx, DrawParam::default().dest(self.rect.point()))?;
        }

        let matches_cursor_state = match cursor_state {
            CursorState::Default => false,
            CursorState::SelectingAttackTarget => self.action == Some(Action::Attack),
            CursorState::SelectingMovementDestination => self.action == Some(Action::Move),
            CursorState::PlacingStructure(structure_type) => {
                self.action == Some(Action::Construct(structure_type))
            }
            CursorState::SelectingResourceTarget => self.action == Some(Action::GatherResource),
            CursorState::DraggingSelectionArea(_) => false,
        };

        let offset = if self.is_down { [4.0, 4.0] } else { [0.0, 0.0] };
        let scale = if self.is_down { [0.9, 0.9] } else { [1.0, 1.0] };
        if let Some(text) = &self.text {
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
        if matches_cursor_state || (self.action.is_some() && is_hovered) {
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
            self.down_cooldown = self.down_cooldown.checked_sub(dt).unwrap_or(Duration::ZERO);
            if self.down_cooldown.is_zero() {
                self.is_down = false;
            }
        }
    }

    fn on_click(&mut self) -> Option<PlayerInput> {
        self.action.map(|action| {
            self.is_down = true;
            self.down_cooldown = Duration::from_millis(100);
            PlayerInput::UseEntityAction(action)
        })
    }
}

#[derive(Debug)]
pub enum PlayerInput {
    UseEntityAction(Action),
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
        Action::Attack => KeyCode::A,
        Action::GatherResource => KeyCode::G,
        Action::ReturnResource => KeyCode::R,
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
        (KeyCode::R, "R"),
        (KeyCode::S, "S"),
    ]
    .map(|(keycode, text)| (keycode, Text::new((text, font, 30.0))))
    .into()
}
