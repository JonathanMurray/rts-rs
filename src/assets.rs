use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::{Context, GameError, GameResult};

use crate::entities::EntitySprite;
use crate::game::{CELL_PIXEL_SIZE, WORLD_PIXEL_OFFSET};
use crate::images;

const COLOR_GRID: Color = Color::new(0.3, 0.3, 0.4, 1.0);

pub struct Assets {
    pub grid_mesh: Mesh,
    player_mesh: Mesh,
    pub selection_mesh: Mesh,
    neutral_mesh: Mesh,
    enemy_sprite_batch: SpriteBatch,
}

impl Assets {
    pub fn draw_entity(
        &mut self,
        ctx: &mut Context,
        sprite: &EntitySprite,
        screen_coords: [f32; 2],
    ) -> GameResult {
        match sprite {
            EntitySprite::Player => {
                self.player_mesh
                    .draw(ctx, DrawParam::new().dest(screen_coords))?;
            }
            EntitySprite::Neutral => {
                self.neutral_mesh
                    .draw(ctx, DrawParam::new().dest(screen_coords))?;
            }
            EntitySprite::Enemy => {
                self.enemy_sprite_batch
                    .add(DrawParam::new().dest(screen_coords));
            }
        };
        Ok(())
    }

    pub fn flush_entity_sprite_batch(&mut self, ctx: &mut Context) -> GameResult {
        self.enemy_sprite_batch.draw(ctx, DrawParam::default())?;
        self.enemy_sprite_batch.clear();
        Ok(())
    }
}

pub fn create_assets(ctx: &mut Context, map_dimensions: (u32, u32)) -> Result<Assets, GameError> {
    let grid_mesh = build_grid(ctx, map_dimensions)?;

    let player_size = (CELL_PIXEL_SIZE.0 * 0.7, CELL_PIXEL_SIZE.1 * 0.8);
    let player_mesh = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE.0 - player_size.0) / 2.0,
                (CELL_PIXEL_SIZE.1 - player_size.1) / 2.0,
                player_size.0,
                player_size.1,
            ),
            Color::new(0.6, 0.8, 0.5, 1.0),
        )?
        .build(ctx)?;
    let selection_mesh = MeshBuilder::new()
        .rectangle(
            DrawMode::stroke(2.0),
            Rect::new(-1.0, -1.0, CELL_PIXEL_SIZE.0 + 2.0, CELL_PIXEL_SIZE.1 + 2.0),
            Color::new(0.6, 0.9, 0.6, 1.0),
        )?
        .build(ctx)?;

    let neutral_size = (CELL_PIXEL_SIZE.0 * 0.7, CELL_PIXEL_SIZE.1 * 0.6);
    let neutral_mesh = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE.0 - player_size.0) / 2.0,
                (CELL_PIXEL_SIZE.1 - player_size.1) / 2.0,
                neutral_size.0,
                neutral_size.1,
            ),
            Color::new(0.8, 0.6, 0.2, 1.0),
        )?
        .build(ctx)?;

    let enemy_mesh = MeshBuilder::new()
        .circle(
            DrawMode::fill(),
            [CELL_PIXEL_SIZE.0 / 2.0, CELL_PIXEL_SIZE.1 / 2.0],
            CELL_PIXEL_SIZE.0 * 0.25,
            0.05,
            Color::new(0.8, 0.4, 0.4, 1.0),
        )?
        .build(ctx)?;
    let enemy_sprite_batch = SpriteBatch::new(images::mesh_into_image(ctx, enemy_mesh)?);
    let assets = Assets {
        grid_mesh,
        player_mesh,
        selection_mesh,
        neutral_mesh,
        enemy_sprite_batch,
    };
    Ok(assets)
}

fn build_grid(ctx: &mut Context, map_dimensions: (u32, u32)) -> Result<Mesh, GameError> {
    let mut builder = MeshBuilder::new();
    const LINE_WIDTH: f32 = 2.0;

    let x0 = WORLD_PIXEL_OFFSET.0;
    let x1 = x0 + map_dimensions.0 as f32 * CELL_PIXEL_SIZE.0;
    let y0 = WORLD_PIXEL_OFFSET.1;
    let y1 = y0 + map_dimensions.1 as f32 * CELL_PIXEL_SIZE.1;

    // Horizontal lines
    for i in 0..map_dimensions.1 + 1 {
        let y = y0 + i as f32 * CELL_PIXEL_SIZE.1;
        builder.line(&[[x0, y], [x1, y]], LINE_WIDTH, COLOR_GRID)?;
    }

    // Vertical lines
    for i in 0..map_dimensions.0 + 1 {
        let x = x0 + i as f32 * CELL_PIXEL_SIZE.0;
        builder.line(&[[x, y0], [x, y1]], LINE_WIDTH, COLOR_GRID)?;
    }

    builder.build(ctx)
}
