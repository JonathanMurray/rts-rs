mod button;
mod entity_header;
pub mod entity_portrait;
mod group_header;
mod healthbar;
mod minimap;
mod progress_bar;

use std::cell::Ref;
use std::convert::TryInto;
use std::time::Duration;

use ggez::graphics::{Color, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::input::mouse::MouseButton;
use ggez::{Context, GameResult};

use self::button::Button;
use self::entity_header::{EntityHeader, EntityHeaderContent};
use self::group_header::GroupHeader;
use self::minimap::Minimap;
use crate::data::{EntityType, HudAssets};
use crate::entities::{
    Action, ActivityTarget, Entity, EntityCategory, EntityState, Team, NUM_ENTITY_ACTIONS,
};
use crate::game::MAX_NUM_SELECTED_ENTITIES;
use crate::grid::ObstacleGrid;
use crate::player::{CursorState, PlayerState};
use crate::text::{SharpFont, SharpText};

const NUM_BUTTONS: usize = NUM_ENTITY_ACTIONS;

pub const HUD_BORDER_COLOR: Color = Color::new(0.7, 0.7, 0.7, 1.0);

pub struct HudGraphics {
    font: SharpFont,
    buttons: [Button; NUM_BUTTONS],
    minimap: Minimap,
    hovered_button_index: Option<usize>,
    error_message: ErrorMessage,
    tooltip: Tooltip,
    entity_header: EntityHeader,
    group_header: GroupHeader,
    assets: HudAssets,
    num_selected_entities: usize,
    resources_position: [f32; 2],
}

impl HudGraphics {
    pub fn new(
        ctx: &mut Context,
        position: [f32; 2],
        font: SharpFont,
        world_dimensions: [u32; 2],
        tooltip_position: [f32; 2],
    ) -> GameResult<Self> {
        let minimap_pos = position;
        let minimap_w = 195.0;
        let minimap = Minimap::new(ctx, minimap_pos, minimap_w, world_dimensions)?;

        let assets = HudAssets::new(ctx)?;

        let header_pos = [position[0], position[1] + 200.0];
        let entity_header = EntityHeader::new(ctx, header_pos, font)?;
        let group_header = GroupHeader::new(ctx, header_pos)?;
        let error_position = [tooltip_position[0] + 5.0, tooltip_position[1] - 30.0];
        let error_message = ErrorMessage::new(font, error_position);
        let tooltip = Tooltip::new(font, tooltip_position, &assets);

        let buttons_x = header_pos[0];
        let buttons_y = header_pos[1] + 110.0;
        let mut buttons = vec![];
        let button_size = [55.0, 50.0];
        let button_hor_margin = 15.0;
        let button_vert_margin = 12.0;
        let buttons_per_row = 3;
        for i in 0..NUM_BUTTONS {
            let x = buttons_x + (i % buttons_per_row) as f32 * (button_size[0] + button_hor_margin);
            let y =
                buttons_y + (i / buttons_per_row) as f32 * (button_size[1] + button_vert_margin);
            let rect = Rect::new(x, y, button_size[0], button_size[1]);
            buttons.push(Button::new(ctx, rect)?);
        }
        let buttons = buttons.try_into().unwrap();

        Ok(Self {
            font,
            buttons,
            minimap,
            hovered_button_index: None,
            error_message,
            tooltip,
            entity_header,
            group_header,
            assets,
            num_selected_entities: 0,
            resources_position: [600.0, 7.0],
        })
    }

    pub fn draw<'a>(
        &mut self,
        ctx: &mut Context,
        player_resources: Option<u32>,
        selected_entities: Vec<Ref<'a, Entity>>,
        player_state: &PlayerState,
        grid: &ObstacleGrid,
    ) -> GameResult {
        assert_eq!(selected_entities.len(), self.num_selected_entities);

        let cursor_state = player_state.cursor_state();

        if let Some(player_resources) = player_resources {
            self.font
                .text(15.0, format!("Fuel: {}", player_resources))
                .draw(ctx, self.resources_position)?;
        }

        if selected_entities.len() > 1 {
            let mut portraits = [None; MAX_NUM_SELECTED_ENTITIES];
            for (i, entity) in selected_entities.iter().enumerate() {
                let config = self.assets.entity(entity.entity_type);
                portraits[i] = Some(&config.portrait);
            }
            self.group_header.draw(ctx, portraits)?;
        } else if selected_entities.len() == 1 {
            let entity = selected_entities.first().unwrap();
            let config = self.assets.entity(entity.entity_type);

            let mut entity_status_text = None;
            let mut progress = None;
            if entity.team == Team::Player {
                if let EntityCategory::Unit(unit) = &entity.category {
                    if let Some(gathering) = unit.gathering.as_ref() {
                        if gathering.is_carrying() {
                            entity_status_text = Some("[carrying fuel]".to_owned());
                        }
                    }
                }
                if let EntityState::DoingActivity(target) = entity.state {
                    let activity = entity.activity.as_ref().unwrap();
                    let activity_progress = activity.progress(target).unwrap();
                    let text = match target {
                        ActivityTarget::Train(..) => "% Training".to_owned(),
                        ActivityTarget::Research => "% Research".to_owned(),
                    };
                    progress = Some((activity_progress, text));
                }
            }
            if entity.entity_type == EntityType::FuelRift {
                let remaining = *entity.resource_remaining();
                entity_status_text = Some(format!("[remaining fuel: {}]", remaining));
            }
            if let EntityState::UnderConstruction(remaining, total) = entity.state {
                let construction_progress = (total - remaining).as_secs_f32() / total.as_secs_f32();
                progress = Some((construction_progress, "% Construction".to_owned()));
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
                    progress,
                    team: entity.team,
                },
            )?;
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
                    self.buttons[index].action().map(TooltipText::Action)
                } else {
                    None
                }
            }
            CursorState::SelectingAttackTarget => Some(TooltipText::CursorSelectAttackTarget),
            CursorState::SelectingMovementDestination => {
                Some(TooltipText::CursorSelectMovementDestination)
            }
            CursorState::PlacingStructure(_) => Some(TooltipText::CursorPlaceStructure),
            CursorState::SelectingResourceTarget => Some(TooltipText::CursorSelectResource),
            CursorState::DraggingSelectionArea(_) => None,
        };
        self.error_message.draw(ctx)?;
        self.tooltip.draw(ctx, tooltip_text, &self.assets)?;

        self.minimap
            .draw(ctx, player_state.camera_position_in_world(), grid)?;

        Ok(())
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

        if self.num_selected_entities > 1 {
            if let Some(player_input) = self.group_header.on_mouse_button_down(x, y) {
                return Some(player_input);
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

        if self.num_selected_entities > 1 {
            self.group_header.on_mouse_motion(x, y);
        }

        self.minimap
            .on_mouse_motion(x, y)
            .map(PlayerInput::SetCameraPositionRelativeToWorldDimension)
    }

    pub fn on_mouse_button_up(&mut self, button: MouseButton) {
        self.minimap.on_mouse_button_up(button);
    }

    pub fn on_key_down(&mut self, keycode: KeyCode) -> Option<PlayerInput> {
        for button in &mut self.buttons {
            if let Some(action) = button.action() {
                if keycode == self.assets.action(action).keycode {
                    button.on_click();
                    return Some(PlayerInput::UseEntityAction(action));
                }
            }
        }
        None
    }

    pub fn update(&mut self, dt: Duration) {
        for button in &mut self.buttons {
            button.update(dt);
        }
        self.error_message.update(dt);
    }

    pub fn set_entity_actions(&mut self, actions: [Option<Action>; NUM_ENTITY_ACTIONS]) {
        for (i, action) in actions.iter().enumerate() {
            if let Some(action) = action {
                let config = self.assets.action(*action);

                self.buttons[i].set_action(Some((*action, config.icon)));
            } else {
                self.buttons[i].set_action(None);
            }
        }
    }

    pub fn set_num_selected_entities(&mut self, num: usize) {
        self.num_selected_entities = num;
    }

    pub fn set_error_message(&mut self, message: String) {
        self.error_message.set_text(message);
    }
}

fn state_matches_action(state: EntityState, action: Action) -> bool {
    match action {
        Action::StartActivity(activity_target, _config) => {
            state == EntityState::DoingActivity(activity_target)
        }
        Action::Construct(structure_type, _) => {
            if let EntityState::MovingToConstruction(constructing_type, _) = state {
                structure_type == constructing_type
            } else {
                false
            }
        }
        Action::Stop => state == EntityState::Idle,
        Action::Move => state == EntityState::Moving,
        Action::Attack => {
            matches!(
                state,
                EntityState::Attacking(_) | EntityState::MovingToAttackTarget(_)
            )
        }
        Action::GatherResource => {
            matches!(
                state,
                EntityState::GatheringResource(_) | EntityState::MovingToResource(_)
            )
        }
        Action::ReturnResource => {
            matches!(state, EntityState::ReturningResource(..))
        }
    }
}

const TOOLTIP_FONT_SIZE: f32 = 17.5;
const ERROR_MESSAGE_FONT_SIZE: f32 = 15.0;

struct ErrorMessage {
    position: [f32; 2],
    font: SharpFont,
    text: Option<SharpText>,
    remaining: Duration,
}

impl ErrorMessage {
    fn new(font: SharpFont, position: [f32; 2]) -> Self {
        Self {
            position,
            font,
            text: None,
            remaining: Duration::ZERO,
        }
    }

    fn set_text(&mut self, text: String) {
        self.text = Some(self.font.text(ERROR_MESSAGE_FONT_SIZE, text));
        self.remaining = Duration::from_secs_f32(1.5);
    }

    fn update(&mut self, dt: Duration) {
        self.remaining = self.remaining.saturating_sub(dt);
        if self.remaining.is_zero() {
            self.text = None;
        }
    }

    fn draw(&self, ctx: &mut Context) -> GameResult {
        if let Some(text) = &self.text {
            text.draw(ctx, self.position)?;
        };
        Ok(())
    }
}

struct Tooltip {
    position: [f32; 2],
    font: SharpFont,
    text_attack: SharpText,
    text_stop: SharpText,
    text_move: SharpText,
    text_gather: SharpText,
    text_return: SharpText,
    text_select_attack_target: SharpText,
    text_select_movement_destination: SharpText,
    text_place_structure: SharpText,
    text_select_resource: SharpText,
}

impl Tooltip {
    fn new(font: SharpFont, position: [f32; 2], assets: &HudAssets) -> Self {
        let text = |t| font.text(TOOLTIP_FONT_SIZE, t);

        Self {
            position,
            font,
            text_attack: text(assets.action(Action::Attack).text.as_ref()),
            text_stop: text(assets.action(Action::Stop).text.as_ref()),
            text_move: text(assets.action(Action::Move).text.as_ref()),
            text_gather: text(assets.action(Action::GatherResource).text.as_ref()),
            text_return: text(assets.action(Action::ReturnResource).text.as_ref()),
            text_select_attack_target: text("Select attack target"),
            text_select_movement_destination: text("Select destination"),
            text_place_structure: text("Place structure"),
            text_select_resource: text("Select resource to gather"),
        }
    }

    fn draw(&self, ctx: &mut Context, text: Option<TooltipText>, assets: &HudAssets) -> GameResult {
        if let Some(text) = text {
            match text {
                TooltipText::Action(Action::Attack) => self.text_attack.draw(ctx, self.position)?,
                TooltipText::Action(Action::Stop) => self.text_stop.draw(ctx, self.position)?,
                TooltipText::Action(Action::Move) => self.text_move.draw(ctx, self.position)?,
                TooltipText::Action(Action::GatherResource) => {
                    self.text_gather.draw(ctx, self.position)?
                }
                TooltipText::Action(Action::ReturnResource) => {
                    self.text_return.draw(ctx, self.position)?
                }
                TooltipText::Action(action) => {
                    let config = assets.action(action);
                    self.font
                        .text(TOOLTIP_FONT_SIZE, &config.text)
                        .draw(ctx, self.position)?;
                }
                TooltipText::CursorSelectAttackTarget => {
                    self.text_select_attack_target.draw(ctx, self.position)?
                }
                TooltipText::CursorSelectMovementDestination => self
                    .text_select_movement_destination
                    .draw(ctx, self.position)?,
                TooltipText::CursorPlaceStructure => {
                    self.text_place_structure.draw(ctx, self.position)?
                }
                TooltipText::CursorSelectResource => {
                    self.text_select_resource.draw(ctx, self.position)?
                }
            }
        };
        Ok(())
    }
}

enum TooltipText {
    Action(Action),
    CursorSelectAttackTarget,
    CursorSelectMovementDestination,
    CursorPlaceStructure,
    CursorSelectResource,
}

#[derive(Debug)]
pub enum PlayerInput {
    UseEntityAction(Action),
    SetCameraPositionRelativeToWorldDimension([f32; 2]),
    LimitSelectionToIndex(usize),
}
