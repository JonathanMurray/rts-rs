use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect};
use ggez::{Context, GameError, GameResult};

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::entities::EntitySprite;
use crate::game::{CELL_PIXEL_SIZE, COLOR_BG};
use crate::images;

const COLOR_GRID: Color = Color::new(0.3, 0.3, 0.4, 1.0);

pub struct Assets {
    grid: Mesh,
    grid_border: Mesh,
    background_around_grid: Vec<Mesh>,
    player_unit: Mesh,
    player_building: Mesh,
    enemy_building: Mesh,
    selections: HashMap<[u32; 2], Mesh>,
    neutral_entity: Mesh,
    enemy_entity_batch: SpriteBatch,
}

impl Assets {
    pub fn draw_selection(
        &mut self,
        ctx: &mut Context,
        size: [u32; 2],
        screen_coords: [f32; 2],
    ) -> GameResult {
        let mesh = match self.selections.entry(size) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(create_selection_mesh(ctx, size)?),
        };
        mesh.draw(ctx, DrawParam::new().dest(screen_coords))
    }

    pub fn draw_grid(
        &self,
        ctx: &mut Context,
        screen_coords: [f32; 2],
        camera_position_in_world: [f32; 2],
    ) -> GameResult {
        self.grid.draw(
            ctx,
            DrawParam::new().dest([
                screen_coords[0] - camera_position_in_world[0] % CELL_PIXEL_SIZE[0],
                screen_coords[1] - camera_position_in_world[1] % CELL_PIXEL_SIZE[1],
            ]),
        )?;

        Ok(())
    }

    pub fn draw_entity(
        &mut self,
        ctx: &mut Context,
        sprite: &EntitySprite,
        screen_coords: [f32; 2],
    ) -> GameResult {
        let param = DrawParam::new().dest(screen_coords);
        match sprite {
            EntitySprite::PlayerUnit => self.player_unit.draw(ctx, param)?,
            EntitySprite::PlayerBuilding => self.player_building.draw(ctx, param)?,
            EntitySprite::Neutral => self.neutral_entity.draw(ctx, param)?,
            EntitySprite::Enemy => {
                self.enemy_entity_batch.add(param);
            }
            EntitySprite::EnemyBuilding => self.enemy_building.draw(ctx, param)?,
        };
        Ok(())
    }

    pub fn draw_background_around_grid(
        &self,
        ctx: &mut Context,
        screen_coords: [f32; 2],
    ) -> GameResult {
        for mesh in &self.background_around_grid {
            mesh.draw(
                ctx,
                DrawParam::new().dest([screen_coords[0], screen_coords[1]]),
            )?;
        }
        self.grid_border.draw(
            ctx,
            DrawParam::new().dest([screen_coords[0], screen_coords[1]]),
        )?;
        Ok(())
    }

    pub fn flush_entity_sprite_batch(&mut self, ctx: &mut Context) -> GameResult {
        self.enemy_entity_batch.draw(ctx, DrawParam::default())?;
        self.enemy_entity_batch.clear();
        Ok(())
    }
}

pub fn create_assets(ctx: &mut Context, camera_size: [f32; 2]) -> Result<Assets, GameError> {
    let grid = build_grid(ctx, camera_size)?;
    let grid_border = MeshBuilder::new()
        .rectangle(
            DrawMode::stroke(3.0),
            Rect::new(0.0, 0.0, camera_size[0], camera_size[1]),
            Color::new(6.0, 3.0, 6.0, 1.0),
        )?
        .build(ctx)?;
    let background_around_grid = build_background_around_grid(ctx, camera_size)?;

    let player_unit_size = [CELL_PIXEL_SIZE[0] * 0.7, CELL_PIXEL_SIZE[1] * 0.8];
    let player_unit = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE[0] - player_unit_size[0]) / 2.0,
                (CELL_PIXEL_SIZE[1] - player_unit_size[1]) / 2.0,
                player_unit_size[0],
                player_unit_size[1],
            ),
            Color::new(0.6, 0.8, 0.5, 1.0),
        )?
        .build(ctx)?;
    let player_building_size = [CELL_PIXEL_SIZE[0] * 1.9, CELL_PIXEL_SIZE[1] * 1.9];
    let player_building = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE[0] * 2.0 - player_building_size[0]) / 2.0,
                (CELL_PIXEL_SIZE[1] * 2.0 - player_building_size[1]) / 2.0,
                player_building_size[0],
                player_building_size[1],
            ),
            Color::new(0.7, 0.5, 0.8, 1.0),
        )?
        .build(ctx)?;
    let enemy_building_size = [CELL_PIXEL_SIZE[0] * 2.9, CELL_PIXEL_SIZE[1] * 1.9];
    let enemy_building = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE[0] * 3.0 - enemy_building_size[0]) / 2.0,
                (CELL_PIXEL_SIZE[1] * 2.0 - enemy_building_size[1]) / 2.0,
                enemy_building_size[0],
                enemy_building_size[1],
            ),
            Color::new(0.9, 0.4, 0.4, 1.0),
        )?
        .build(ctx)?;

    let neutral_size = [CELL_PIXEL_SIZE[0] * 0.7, CELL_PIXEL_SIZE[1] * 0.6];
    let neutral_entity = MeshBuilder::new()
        .rectangle(
            DrawMode::fill(),
            Rect::new(
                (CELL_PIXEL_SIZE[0] - player_unit_size[0]) / 2.0,
                (CELL_PIXEL_SIZE[1] - player_unit_size[1]) / 2.0,
                neutral_size[0],
                neutral_size[1],
            ),
            Color::new(0.8, 0.6, 0.2, 1.0),
        )?
        .build(ctx)?;

    let enemy_mesh = MeshBuilder::new()
        .circle(
            DrawMode::fill(),
            [CELL_PIXEL_SIZE[0] / 2.0, CELL_PIXEL_SIZE[1] / 2.0],
            CELL_PIXEL_SIZE[0] * 0.25,
            0.05,
            Color::new(0.8, 0.4, 0.4, 1.0),
        )?
        .build(ctx)?;
    let enemy_entity_batch = SpriteBatch::new(images::mesh_into_image(ctx, enemy_mesh)?);
    let selections = Default::default();
    let assets = Assets {
        grid,
        grid_border,
        background_around_grid,
        player_unit,
        player_building,
        enemy_building,
        selections,
        neutral_entity,
        enemy_entity_batch,
    };
    Ok(assets)
}

fn create_selection_mesh(ctx: &mut Context, size: [u32; 2]) -> GameResult<Mesh> {
    MeshBuilder::new()
        .rectangle(
            DrawMode::stroke(2.0),
            Rect::new(
                -1.0,
                -1.0,
                CELL_PIXEL_SIZE[0] * size[0] as f32 + 2.0,
                CELL_PIXEL_SIZE[1] * size[1] as f32 + 2.0,
            ),
            Color::new(0.6, 0.9, 0.6, 1.0),
        )?
        .build(ctx)
}

fn build_background_around_grid(ctx: &mut Context, camera_size: [f32; 2]) -> GameResult<Vec<Mesh>> {
    // This feels hacky. We use 4 huge meshes that are placed surrounding the grid as a way to
    // draw a background over any entities that were rendered (either fully or just partially)
    // outside of the game world area. Is there some nicer way to do this, supported by ggez?
    // Essentially what we want is an inverted Rect: "Draw a background on the entire screen except
    // this rect."
    let margin = 1000.0;
    let meshes = vec![
        // TOP
        MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(-margin, -margin, camera_size[0] + 2.0 * margin, margin),
                COLOR_BG,
            )?
            .build(ctx)?,
        // BOTTOM
        MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    -margin,
                    camera_size[1],
                    camera_size[0] + 2.0 * margin,
                    margin,
                ),
                COLOR_BG,
            )?
            .build(ctx)?,
        // LEFT
        MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(-margin, 0.0, margin, camera_size[1]),
                COLOR_BG,
            )?
            .build(ctx)?,
        // RIGHT
        MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(camera_size[0], 0.0, margin, camera_size[1]),
                COLOR_BG,
            )?
            .build(ctx)?,
    ];
    Ok(meshes)
}

fn build_grid(ctx: &mut Context, camera_size: [f32; 2]) -> Result<Mesh, GameError> {
    let mut builder = MeshBuilder::new();
    const LINE_WIDTH: f32 = 2.0;

    let x0 = -CELL_PIXEL_SIZE[0];
    let x1 = x0 + camera_size[0] + CELL_PIXEL_SIZE[0] * 2.0;
    let y0 = -CELL_PIXEL_SIZE[1];
    let y1 = y0 + camera_size[1] + CELL_PIXEL_SIZE[1] * 2.0;

    let num_columns = ((x1 - x0) / CELL_PIXEL_SIZE[0] as f32) as u32;
    let num_rows = ((y1 - y0) / CELL_PIXEL_SIZE[1] as f32) as u32;

    // Horizontal lines
    for i in 0..num_rows {
        let y = y0 + i as f32 * CELL_PIXEL_SIZE[1];
        builder.line(&[[x0, y], [x1, y]], LINE_WIDTH, COLOR_GRID)?;
    }

    // Vertical lines
    for i in 0..num_columns {
        let x = x0 + i as f32 * CELL_PIXEL_SIZE[0];
        builder.line(&[[x, y0], [x, y1]], LINE_WIDTH, COLOR_GRID)?;
    }

    builder.build(ctx)
}
