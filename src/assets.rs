use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::{Context, GameError, GameResult};

use crate::entities::EntitySprite;
use crate::game::{CELL_PIXEL_SIZE, WORLD_PIXEL_OFFSET};
use crate::images;

const COLOR_GRID: Color = Color::new(0.3, 0.3, 0.4, 1.0);

pub struct Assets {
    pub grid: Mesh,
    player_entity: Mesh,
    player_entity_2: Mesh,
    pub selection: Mesh,
    neutral_entity: Mesh,
    enemy_entity_batch: SpriteBatch,
}

impl Assets {
    pub fn draw_entity(
        &mut self,
        ctx: &mut Context,
        sprite: &EntitySprite,
        screen_coords: [f32; 2],
    ) -> GameResult {
        let param = DrawParam::new().dest(screen_coords);
        match sprite {
            EntitySprite::Player => self.player_entity.draw(ctx, param)?,
            EntitySprite::Player2 => self.player_entity_2.draw(ctx, param)?,
            EntitySprite::Neutral => self.neutral_entity.draw(ctx, param)?,
            EntitySprite::Enemy => {
                self.enemy_entity_batch.add(param);
            },
        };
        Ok(())
    }

    pub fn flush_entity_sprite_batch(&mut self, ctx: &mut Context) -> GameResult {
        self.enemy_entity_batch.draw(ctx, DrawParam::default())?;
        self.enemy_entity_batch.clear();
        Ok(())
    }
}

pub fn create_assets(ctx: &mut Context, map_dimensions: (u32, u32)) -> Result<Assets, GameError> {
    let grid = build_grid(ctx, map_dimensions)?;

    let player_size = (CELL_PIXEL_SIZE.0 * 0.7, CELL_PIXEL_SIZE.1 * 0.8);
    let player_entity = MeshBuilder::new()
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
    let player_entity_2 = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE.0 - player_size.0) / 2.0,
                (CELL_PIXEL_SIZE.1 - player_size.1) / 2.0,
                player_size.0,
                player_size.1,
            ),
            Color::new(0.7, 0.5, 0.8, 1.0),
        )?
        .build(ctx)?;

    let selection = MeshBuilder::new()
        .rectangle(
            DrawMode::stroke(2.0),
            Rect::new(-1.0, -1.0, CELL_PIXEL_SIZE.0 + 2.0, CELL_PIXEL_SIZE.1 + 2.0),
            Color::new(0.6, 0.9, 0.6, 1.0),
        )?
        .build(ctx)?;

    let neutral_size = (CELL_PIXEL_SIZE.0 * 0.7, CELL_PIXEL_SIZE.1 * 0.6);
    let neutral_entity = MeshBuilder::new()
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
    let enemy_entity_batch = SpriteBatch::new(images::mesh_into_image(ctx, enemy_mesh)?);
    let assets = Assets {
        grid,
        player_entity,
        player_entity_2,
        selection,
        neutral_entity,
        enemy_entity_batch,
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
