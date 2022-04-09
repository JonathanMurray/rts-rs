use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{
    Canvas, Color, DrawMode, DrawParam, Drawable, FilterMode, Image, Mesh, MeshBuilder, Rect,
};
use ggez::{graphics, Context, GameError, GameResult};

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::entities::{EntitySprite, Team};
use crate::game::{CELL_PIXEL_SIZE, COLOR_FG, WORLD_VIEWPORT};
use crate::grid::Grid;
use crate::images;
use crate::map::TileId;
use ggez::conf::NumSamples;

const COLOR_GRID: Color = Color::new(0.3, 0.3, 0.4, 1.0);

const TILE_PIXEL_SIZE: [f32; 2] = [CELL_PIXEL_SIZE[0] / 2.0, CELL_PIXEL_SIZE[1] / 2.0];

pub struct Assets {
    grid: Mesh,
    grid_border: Mesh,
    background_around_grid: Vec<Mesh>,
    selections: HashMap<([u32; 2], Team), Mesh>,
    construction_outlines: HashMap<[u32; 2], Mesh>,
    neutral_entity: Mesh,
    entity_batches: HashMap<(EntitySprite, Team), SpriteBatch>,
    movement_command_indicator: Mesh,
    world_background: Image,
    world_size: [f32; 2],
}

impl Assets {
    pub fn new(
        ctx: &mut Context,
        camera_size: [f32; 2],
        tile_grid: &Grid<TileId>,
    ) -> GameResult<Assets> {
        let grid = create_grid(ctx, camera_size)?;
        let grid_border = MeshBuilder::new()
            .rectangle(
                DrawMode::stroke(2.0),
                Rect::new(0.0, 0.0, camera_size[0], camera_size[1]),
                Color::new(0.0, 0.0, 0.0, 1.0),
            )?
            .build(ctx)?;
        let background_around_grid = create_background_around_grid(ctx, camera_size)?;

        let mut entity_batches = Default::default();
        create_fighter(ctx, &mut entity_batches)?;
        create_worker(ctx, &mut entity_batches)?;
        create_barracks(ctx, &mut entity_batches)?;
        create_townhall(ctx, &mut entity_batches)?;

        let neutral_size = [CELL_PIXEL_SIZE[0] * 0.7, CELL_PIXEL_SIZE[1] * 0.8];
        let neutral_entity = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    (CELL_PIXEL_SIZE[0] - neutral_size[0]) / 2.0,
                    (CELL_PIXEL_SIZE[1] - neutral_size[1]) / 2.0,
                    neutral_size[0],
                    neutral_size[1],
                ),
                Color::new(0.8, 0.6, 0.2, 1.0),
            )?
            .build(ctx)?;

        let movement_command_indicator = MeshBuilder::new()
            .circle(
                DrawMode::stroke(2.0),
                [0.0, 0.0],
                25.0,
                0.01,
                Color::new(0.6, 1.0, 0.6, 1.0),
            )?
            .build(ctx)?;

        let mut tile_map = Image::new(ctx, "/images/tile_map.png")?;
        tile_map.set_filter(FilterMode::Nearest); // Make sure our pixels are preserved exactly

        let world_background = Self::create_background_from_tile_map(ctx, &tile_map, tile_grid)?;

        let world_size = [
            tile_grid.dimensions[0] as f32 * TILE_PIXEL_SIZE[0],
            tile_grid.dimensions[1] as f32 * TILE_PIXEL_SIZE[1],
        ];

        let assets = Assets {
            grid,
            grid_border,
            background_around_grid,
            selections: Default::default(),
            construction_outlines: Default::default(),
            neutral_entity,
            entity_batches,
            movement_command_indicator,
            world_background,
            world_size,
        };
        Ok(assets)
    }

    pub fn draw_selection(
        &mut self,
        ctx: &mut Context,
        size: [u32; 2],
        team: Team,
        screen_coords: [f32; 2],
    ) -> GameResult {
        let mesh = match self.selections.entry((size, team)) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(create_selection_mesh(ctx, size, team)?),
        };
        mesh.draw(ctx, DrawParam::new().dest(screen_coords))
    }

    pub fn draw_construction_outline(
        &mut self,
        ctx: &mut Context,
        size: [u32; 2],
        screen_coords: [f32; 2],
    ) -> GameResult {
        let mesh = match self.construction_outlines.entry(size) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(create_construction_outline_mesh(ctx, size)?),
        };
        mesh.draw(ctx, DrawParam::new().dest(screen_coords))
    }

    pub fn draw_movement_command_indicator(
        &self,
        ctx: &mut Context,
        screen_coords: [f32; 2],
        scale: f32,
    ) -> GameResult {
        self.movement_command_indicator.draw(
            ctx,
            DrawParam::new().dest(screen_coords).scale([scale, scale]),
        )
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

    fn create_background_from_tile_map(
        ctx: &mut Context,
        tile_map: &Image,
        tile_grid: &Grid<TileId>,
    ) -> GameResult<Image> {
        let width = tile_grid.dimensions[0] as f32 * TILE_PIXEL_SIZE[0];
        let height = tile_grid.dimensions[1] as f32 * TILE_PIXEL_SIZE[1];
        let color_format = graphics::get_window_color_format(ctx);
        let canvas = Canvas::new(
            ctx,
            width as u16,
            height as u16,
            NumSamples::One,
            color_format,
        )?;

        // Change drawing mode: draw to canvas
        graphics::set_canvas(ctx, Some(&canvas));
        let original_screen_coordinates = graphics::screen_coordinates(ctx);
        graphics::set_screen_coordinates(ctx, Rect::new(0.0, 0.0, width, height))?;

        for x in 0..tile_grid.dimensions[0] {
            for y in 0..tile_grid.dimensions[1] {
                if let Some(tile) = tile_grid.get(&[x, y]) {
                    // One tile takes up a fraction of the entire tile-map
                    // ggez requires us to specify the src of the tile-map in "relative" terms
                    // (where [0.0, 0.0] is the top-left corner and [1.0, 1.0] is the bottom-right)
                    let fraction = 1.0 / 8.0;

                    // Our tile-map consists of 16x16 pixel tiles
                    // Each tile should take up 32x32 logical game pixels
                    let tile_scale = [2.0, 2.0];

                    let position_of_tile_in_tilemap = match tile {
                        TileId::Ground => [0, 0],
                        TileId::WaterCenter => [1, 2],
                        TileId::WaterEdgeNorth => [1, 1],
                        TileId::WaterCornerNE => [2, 1],
                        TileId::WaterEdgeEast => [2, 2],
                        TileId::WaterCornerSE => [2, 3],
                        TileId::WaterEdgeSouth => [1, 3],
                        TileId::WaterCornerSW => [0, 3],
                        TileId::WaterEdgeWest => [0, 2],
                        TileId::WaterCornerNW => [0, 1],
                        TileId::WaterConcaveNE => [0, 5],
                        TileId::WaterConcaveSE => [0, 4],
                        TileId::WaterConcaveSW => [1, 4],
                        TileId::WaterConcaveNW => [1, 5],
                    };

                    tile_map.draw(
                        ctx,
                        DrawParam::new()
                            .src(Rect::new(
                                fraction * position_of_tile_in_tilemap[0] as f32,
                                fraction * position_of_tile_in_tilemap[1] as f32,
                                fraction,
                                fraction,
                            ))
                            .dest([x as f32 * TILE_PIXEL_SIZE[0], y as f32 * TILE_PIXEL_SIZE[1]])
                            .scale(tile_scale),
                    )?;
                }
            }
        }
        let image = canvas.to_image(ctx)?;

        // Change back drawing mode: draw to screen
        graphics::set_canvas(ctx, None);
        graphics::set_screen_coordinates(ctx, original_screen_coordinates)?;

        Ok(image)
    }

    pub fn draw_world_background(
        &mut self,
        ctx: &mut Context,
        screen_coords: [f32; 2],
        camera_position_in_world: [f32; 2],
    ) -> GameResult {
        // Image src is "relative" in ggez, i.e. not measured in number of pixels
        let relative_src_rect = Rect::new(
            camera_position_in_world[0] / self.world_size[0],
            camera_position_in_world[1] / self.world_size[1],
            WORLD_VIEWPORT.w / self.world_size[0],
            WORLD_VIEWPORT.h / self.world_size[1],
        );
        self.world_background.draw(
            ctx,
            DrawParam::new().src(relative_src_rect).dest(screen_coords),
        )?;

        Ok(())
    }

    pub fn draw_entity(
        &mut self,
        ctx: &mut Context,
        sprite: EntitySprite,
        team: Team,
        screen_coords: [f32; 2],
    ) -> GameResult {
        let param = DrawParam::new().dest(screen_coords);
        match sprite {
            EntitySprite::Resource => self.neutral_entity.draw(ctx, param)?,
            entity_sprite => {
                self.entity_batches
                    .get_mut(&(entity_sprite, team))
                    .unwrap_or_else(|| panic!("Unhandled sprite: {:?}", entity_sprite))
                    .add(param);
            }
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
        for batch in self.entity_batches.values_mut() {
            batch.draw(ctx, DrawParam::default())?;
            batch.clear();
        }
        Ok(())
    }
}

fn create_fighter(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let size = [CELL_PIXEL_SIZE[0] * 0.7, CELL_PIXEL_SIZE[1] * 0.8];
    let rect = Rect::new(
        (CELL_PIXEL_SIZE[0] - size[0]) / 2.0,
        (CELL_PIXEL_SIZE[1] - size[1]) / 2.0,
        size[0],
        size[1],
    );
    let colors = HashMap::from([
        (Team::Player, Color::new(0.6, 0.8, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.8, 0.4, 0.4, 1.0)),
    ]);
    for (team, color) in colors {
        let mesh = MeshBuilder::new()
            .rounded_rectangle(DrawMode::fill(), rect, 5.0, color)?
            .build(ctx)?;
        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Fighter, team), batch);
    }
    Ok(())
}

fn create_worker(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let colors = HashMap::from([
        (Team::Player, Color::new(0.6, 0.8, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.8, 0.4, 0.4, 1.0)),
    ]);
    for (team, color) in colors {
        let mesh = MeshBuilder::new()
            .circle(
                DrawMode::fill(),
                [CELL_PIXEL_SIZE[0] / 2.0, CELL_PIXEL_SIZE[1] / 2.0],
                CELL_PIXEL_SIZE[0] * 0.35,
                0.05,
                color,
            )?
            .build(ctx)?;
        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Worker, team), batch);
    }
    Ok(())
}

fn create_barracks(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let colors = HashMap::from([
        (Team::Player, Color::new(0.6, 0.8, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.8, 0.4, 0.4, 1.0)),
    ]);
    for (team, color) in colors {
        let size = [CELL_PIXEL_SIZE[0] * 1.9, CELL_PIXEL_SIZE[1] * 1.9];
        let mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    (CELL_PIXEL_SIZE[0] * 2.0 - size[0]) / 2.0,
                    (CELL_PIXEL_SIZE[1] * 2.0 - size[1]) / 2.0,
                    size[0],
                    size[1],
                ),
                color,
            )?
            .rectangle(
                DrawMode::stroke(2.0),
                Rect::new(
                    CELL_PIXEL_SIZE[0] * 0.75,
                    CELL_PIXEL_SIZE[1] * 0.5,
                    CELL_PIXEL_SIZE[0] * 0.5,
                    CELL_PIXEL_SIZE[1] * 0.5,
                ),
                Color::new(0.0, 0.0, 0.0, 1.0),
            )?
            .build(ctx)?;

        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Barracks, team), batch);
    }
    Ok(())
}

fn create_townhall(
    ctx: &mut Context,
    sprite_batches: &mut HashMap<(EntitySprite, Team), SpriteBatch>,
) -> GameResult {
    let colors = HashMap::from([
        (Team::Player, Color::new(0.5, 0.7, 0.5, 1.0)),
        (Team::Enemy, Color::new(0.7, 0.3, 0.3, 1.0)),
    ]);
    for (team, color) in colors {
        let size = [CELL_PIXEL_SIZE[0] * 2.9, CELL_PIXEL_SIZE[1] * 1.9];
        let mesh = MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(
                    (CELL_PIXEL_SIZE[0] * 3.0 - size[0]) / 2.0,
                    (CELL_PIXEL_SIZE[1] * 2.0 - size[1]) / 2.0,
                    size[0],
                    size[1],
                ),
                color,
            )?
            .circle(
                DrawMode::stroke(4.0),
                [CELL_PIXEL_SIZE[0] * 1.5, CELL_PIXEL_SIZE[1] * 0.7],
                CELL_PIXEL_SIZE[0] * 0.4,
                0.05,
                Color::new(0.0, 0.0, 0.0, 1.0),
            )?
            .build(ctx)?;

        let batch = SpriteBatch::new(images::mesh_into_image(ctx, mesh)?);
        sprite_batches.insert((EntitySprite::Townhall, team), batch);
    }
    Ok(())
}

fn create_selection_mesh(ctx: &mut Context, size: [u32; 2], team: Team) -> GameResult<Mesh> {
    let color = match team {
        Team::Player => Color::new(0.6, 0.9, 0.6, 1.0),
        Team::Enemy => Color::new(0.8, 0.4, 0.4, 1.0),
        Team::Neutral => Color::new(0.8, 0.8, 0.6, 1.0),
    };
    MeshBuilder::new()
        .rectangle(
            DrawMode::stroke(2.0),
            Rect::new(
                -1.0,
                -1.0,
                CELL_PIXEL_SIZE[0] * size[0] as f32 + 2.0,
                CELL_PIXEL_SIZE[1] * size[1] as f32 + 2.0,
            ),
            color,
        )?
        .build(ctx)
}

fn create_construction_outline_mesh(ctx: &mut Context, size: [u32; 2]) -> GameResult<Mesh> {
    let rect = Rect::new(
        0.0,
        0.0,
        CELL_PIXEL_SIZE[0] * size[0] as f32,
        CELL_PIXEL_SIZE[1] * size[1] as f32,
    );
    MeshBuilder::new()
        .rectangle(DrawMode::fill(), rect, Color::new(0.4, 0.8, 0.4, 0.05))?
        .rectangle(DrawMode::stroke(2.0), rect, Color::new(0.6, 0.9, 0.6, 1.0))?
        .build(ctx)
}

fn create_background_around_grid(
    ctx: &mut Context,
    camera_size: [f32; 2],
) -> GameResult<Vec<Mesh>> {
    // HACK: We use 4 huge meshes that are placed surrounding the grid as a way to
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
                COLOR_FG,
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
                COLOR_FG,
            )?
            .build(ctx)?,
        // LEFT
        MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(-margin, 0.0, margin, camera_size[1]),
                COLOR_FG,
            )?
            .build(ctx)?,
        // RIGHT
        MeshBuilder::new()
            .rectangle(
                DrawMode::fill(),
                Rect::new(camera_size[0], 0.0, margin, camera_size[1]),
                COLOR_FG,
            )?
            .build(ctx)?,
    ];
    Ok(meshes)
}

fn create_grid(ctx: &mut Context, camera_size: [f32; 2]) -> Result<Mesh, GameError> {
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
