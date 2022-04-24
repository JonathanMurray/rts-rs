use crate::assets::Assets;
use crate::entities::Entity;
use crate::game::{CELL_PIXEL_SIZE, WORLD_VIEWPORT};
use crate::grid::Grid;
use crate::map::{self, WorldInitData};

use ggez;
use ggez::conf::{NumSamples, WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::graphics::{Color, FilterMode, Rect};
use ggez::input::mouse::MouseButton;
use ggez::{graphics, Context, ContextBuilder, GameError, GameResult};
use std::io::Read;

const COLOR_FG: Color = Color::new(0.3, 0.3, 0.4, 1.0);
const GAME_SIZE: [f32; 2] = [800.0, 450.0];

pub fn run(filepath: String) -> GameResult {
    const GAME_SCALE: f32 = 3.0;
    let window_setup = WindowSetup::default()
        .title("EDITOR")
        .samples(NumSamples::One);
    let window_mode =
        WindowMode::default().dimensions(GAME_SIZE[0] * GAME_SCALE, GAME_SIZE[1] * GAME_SCALE);
    let (mut ctx, event_loop) = ContextBuilder::new("rts editor", "jm")
        .window_setup(window_setup)
        .window_mode(window_mode)
        .add_resource_path("resources")
        .build()
        .expect("Creating ggez context");

    graphics::set_default_filter(&mut ctx, FilterMode::Nearest);
    graphics::set_screen_coordinates(&mut ctx, Rect::new(0.0, 0.0, GAME_SIZE[0], GAME_SIZE[1]))
        .unwrap();

    let mut file = std::fs::File::open(&filepath).unwrap();
    let mut map_file_contents = String::new();
    file.read_to_string(&mut map_file_contents).unwrap();

    let WorldInitData {
        dimensions: _dimensions,
        entities,
        water_grid,
        tile_grid,
    } = WorldInitData::load_from_file_contents(map_file_contents);

    let assets = Assets::new(&mut ctx, [WORLD_VIEWPORT.w, WORLD_VIEWPORT.h], &tile_grid)?;

    let editor = Editor {
        filepath,
        assets,
        water_grid,
        entities,
        left_mouse_current_cell: None,
        right_mouse_current_cell: None,
    };

    ggez::event::run(ctx, event_loop, editor)
}

struct Editor {
    filepath: String,
    assets: Assets,
    water_grid: Grid<bool>,
    entities: Vec<Entity>,
    left_mouse_current_cell: Option<[u32; 2]>,
    right_mouse_current_cell: Option<[u32; 2]>,
}

impl EventHandler for Editor {
    fn update(&mut self, _ctx: &mut Context) -> Result<(), GameError> {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        graphics::clear(ctx, COLOR_FG);
        let camera_pos = [0.0, 0.0];
        self.assets
            .draw_world_background(ctx, WORLD_VIEWPORT.point().into(), camera_pos)?;
        self.assets
            .draw_grid(ctx, WORLD_VIEWPORT.point().into(), camera_pos)?;

        for entity in &self.entities {
            let world_pixel_coords = entity.world_pixel_position();
            let screen_coords = [
                world_pixel_coords[0] + WORLD_VIEWPORT.x,
                world_pixel_coords[1] + WORLD_VIEWPORT.y,
            ];
            self.assets.draw_entity(ctx, entity, screen_coords)?;
        }

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        let [x, y] = physical_to_logical(ctx, [x, y]);
        if WORLD_VIEWPORT.contains([x, y]) {
            let world_pos = world_to_grid([x - WORLD_VIEWPORT.x, y - WORLD_VIEWPORT.y]);
            if button == MouseButton::Left {
                self.left_mouse_current_cell = Some(world_pos);
                self.add_water(ctx, world_pos);
            } else if button == MouseButton::Right {
                self.right_mouse_current_cell = Some(world_pos);
                self.remove_water(ctx, world_pos);
            }
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Left {
            self.left_mouse_current_cell = None;
        } else if button == MouseButton::Right {
            self.right_mouse_current_cell = None;
        }
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        let [x, y] = physical_to_logical(ctx, [x, y]);
        if WORLD_VIEWPORT.contains([x, y]) {
            let world_pos = world_to_grid([x - WORLD_VIEWPORT.x, y - WORLD_VIEWPORT.y]);
            if self.left_mouse_current_cell.is_some()
                && self.left_mouse_current_cell != Some(world_pos)
            {
                self.left_mouse_current_cell = Some(world_pos);
                self.add_water(ctx, world_pos);
            }
            if self.right_mouse_current_cell.is_some()
                && self.right_mouse_current_cell != Some(world_pos)
            {
                self.right_mouse_current_cell = Some(world_pos);
                self.remove_water(ctx, world_pos);
            }
        }
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        if keycode == KeyCode::Escape {
            event::quit(ctx);
        } else if keycode == KeyCode::S {
            self.save();
        }
    }
}

impl Editor {
    fn add_water(&mut self, ctx: &mut Context, clicked_world_pos: [u32; 2]) {
        if !self.water_grid.get(&clicked_world_pos).unwrap() {
            self.water_grid.set(clicked_world_pos, true);
            self.update_background_tiles(ctx);
        }
    }

    fn remove_water(&mut self, ctx: &mut Context, clicked_world_pos: [u32; 2]) {
        if self.water_grid.get(&clicked_world_pos).unwrap() {
            self.water_grid.set(clicked_world_pos, false);
            self.update_background_tiles(ctx);
        }
    }

    fn update_background_tiles(&mut self, ctx: &mut Context) {
        let tile_grid = map::create_tile_grid(&self.water_grid);
        self.assets
            .update_background_tiles(ctx, &tile_grid)
            .unwrap();
    }

    fn save(&self) {
        WorldInitData::save_to_file(&self.water_grid, &self.entities, &self.filepath);
    }
}

fn physical_to_logical(ctx: &mut Context, coordinates: [f32; 2]) -> [f32; 2] {
    let screen_rect = graphics::screen_coordinates(ctx);
    let size = graphics::window(ctx).inner_size();
    [
        screen_rect.x + coordinates[0] / size.width as f32 * screen_rect.w,
        screen_rect.y + coordinates[1] / size.height as f32 * screen_rect.h,
    ]
}

fn world_to_grid(world_coordinates: [f32; 2]) -> [u32; 2] {
    let grid_x = world_coordinates[0] / CELL_PIXEL_SIZE[0];
    let grid_y = world_coordinates[1] / CELL_PIXEL_SIZE[1];
    let grid_x = grid_x as u32;
    let grid_y = grid_y as u32;
    [grid_x, grid_y]
}
