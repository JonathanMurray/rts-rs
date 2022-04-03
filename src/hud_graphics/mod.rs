mod button;
mod entity_header;
mod healthbar;
mod minimap;
mod trainingbar;

use std::cell::Ref;
use std::collections::HashMap;
use std::convert::TryInto;
use std::time::Duration;

use ggez::graphics::{self, DrawParam, Drawable, Font, Rect, Text};
use ggez::input::keyboard::KeyCode;
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use self::button::Button;
use self::entity_header::{EntityHeader, EntityHeaderContent};
use self::minimap::Minimap;
use crate::core::TeamState;
use crate::data::{EntityType, HudAssets};
use crate::entities::{
    Action, Entity, EntityState, PhysicalType, Team, TrainingConfig, NUM_ENTITY_ACTIONS,
};
use crate::game::{CursorState, PlayerState};

const NUM_BUTTONS: usize = NUM_ENTITY_ACTIONS;

pub struct HudGraphics {
    position_on_screen: [f32; 2],
    font: Font,
    buttons: [Button; NUM_BUTTONS],
    minimap: Minimap,
    hovered_button_index: Option<usize>,
    keycode_labels: HashMap<KeyCode, Text>,
    tooltip: Tooltip,
    entity_header: EntityHeader,
    assets: HudAssets,
}

impl HudGraphics {
    pub fn new(
        ctx: &mut Context,
        position: [f32; 2],
        font: Font,
        world_dimensions: [u32; 2],
    ) -> GameResult<Self> {
        let minimap_pos = [900.0, position[1] + 30.0];
        let minimap = Minimap::new(ctx, minimap_pos, world_dimensions)?;

        let keycode_labels = create_keycode_labels(font);

        let tooltip = Tooltip::new(font, [position[0], position[1] + 420.0]);
        let entity_header = EntityHeader::new(ctx, [position[0], position[1] + 20.0], font)?;

        let buttons_x = position[0];
        let buttons_y = position[1] + 240.0;
        let mut buttons = vec![];
        let button_size = [100.0, 70.0];
        let button_hor_margin = 30.0;
        let button_vert_margin = 15.0;
        let buttons_per_row = 3;
        for i in 0..NUM_BUTTONS {
            let x = buttons_x + (i % buttons_per_row) as f32 * (button_size[0] + button_hor_margin);
            let y =
                buttons_y + (i / buttons_per_row) as f32 * (button_size[1] + button_vert_margin);
            let rect = Rect::new(x, y, button_size[0], button_size[1]);
            buttons.push(Button::new(ctx, rect)?);
        }
        let buttons = buttons.try_into().unwrap();

        let assets = HudAssets::new(ctx)?;

        Ok(Self {
            position_on_screen: position,
            font,
            buttons,
            minimap,
            hovered_button_index: None,
            keycode_labels,
            tooltip,
            entity_header,
            assets,
        })
    }

    pub fn draw<'a>(
        &self,
        ctx: &mut Context,
        player_team_state: Ref<TeamState>,
        selected_entities: Vec<Ref<'a, Entity>>,
        player_state: &PlayerState,
    ) -> GameResult {
        let x = 0.0;

        let medium_font = 30.0;
        let large_font = 40.0;

        let cursor_state = player_state.cursor_state();

        let resources_text = Text::new((
            format!("RESOURCES: {}", player_team_state.resources),
            self.font,
            medium_font,
        ));
        resources_text.draw(ctx, DrawParam::new().dest([1200.0, 15.0]))?;

        if selected_entities.len() > 1 {
            let mut y = 28.0;
            for entity in &selected_entities {
                let config = self.assets.get(entity.entity_type);
                self.draw_text(ctx, [x, y], &config.name, large_font)?;
                y += 50.0;
            }
        } else if selected_entities.len() == 1 {
            let entity = selected_entities.first().unwrap();
            let config = self.assets.get(entity.entity_type);

            let mut entity_status_text = None;
            let mut training_progress = None;
            if entity.team == Team::Player {
                if let PhysicalType::Unit(unit) = &entity.physical_type {
                    if let Some(gathering) = unit.gathering.as_ref() {
                        if gathering.is_carrying() {
                            entity_status_text = Some("[carrying resource]".to_owned());
                        }
                    }
                }
                if let EntityState::TrainingUnit(trained_entity_type) = entity.state {
                    // TODO: Use some other way of determining when to hide buttons
                    //is_training = true;
                    entity_status_text = Some(format!("[training {:?}]", trained_entity_type));
                    let training = entity.training.as_ref().unwrap();
                    training_progress = Some(training.progress(trained_entity_type).unwrap());
                }
            } else if entity.team == Team::Neutral {
                entity_status_text = Some("[plenty of resources]".to_owned());
            }

            let (current_health, max_health) = entity
                .health
                .as_ref()
                .map(|h| (h.current as usize, h.max as usize))
                .unwrap_or((0, 1));
            self.entity_header.draw(
                ctx,
                EntityHeaderContent {
                    current_health,
                    max_health,
                    portrait: &config.portrait,
                    name: config.name.clone(),
                    status: entity_status_text,
                    training_progress,
                    team: entity.team,
                },
            )?;

            // self.draw_text(
            //     ctx,
            //     [x + 200.0, 110.0],
            //     format!("({:?})", entity.state),
            //     small_font,
            // )?;
        }

        for (button_i, button) in self.buttons.iter().enumerate() {
            let is_hovered = self.hovered_button_index == Some(button_i);
            let matches_entity_state = button
                .action()
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
                    match self.buttons[index].action() {
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
            if button.contains([x, y]) {
                if let Some(action) = button.on_click() {
                    return Some(PlayerInput::UseEntityAction(action));
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
            .position(|button| button.contains([x, y]));

        self.minimap
            .on_mouse_motion(x, y)
            .map(PlayerInput::SetCameraPositionRelativeToWorldDimension)
    }

    pub fn on_mouse_button_up(&mut self, button: MouseButton) {
        self.minimap.on_mouse_button_up(button);
    }

    pub fn on_key_down(&self, keycode: KeyCode) -> Option<PlayerInput> {
        for action in self.buttons.iter().filter_map(|b| b.action()) {
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
            if let Some(action) = action {
                let text = self.action_label(action).clone();
                self.buttons[i].set_action(Some((*action, text)));
            } else {
                self.buttons[i].set_action(None);
            }
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
