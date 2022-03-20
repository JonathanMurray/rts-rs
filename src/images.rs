use ggez::conf::NumSamples;
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::Drawable;
use ggez::graphics::{Canvas, Color, DrawParam, Mesh, Rect};
use ggez::{graphics, Context, GameError};

pub fn mesh_into_image(ctx: &mut Context, mesh: Mesh) -> Result<SpriteBatch, GameError> {
    let dimensions = mesh.dimensions(ctx).unwrap();
    let width = dimensions.x + dimensions.w;
    let height = dimensions.y + dimensions.h;
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
    graphics::set_screen_coordinates(ctx, Rect::new(0.0, 0.0, width, height))?;

    let transparent_bg = Color::new(0.0, 0.0, 0.0, 0.0);
    graphics::clear(ctx, transparent_bg);
    graphics::draw(ctx, &mesh, DrawParam::default())?;
    let image = canvas.to_image(ctx)?;
    let sprite_batch = SpriteBatch::new(image);

    // Change back drawing mode: draw to screen
    graphics::set_canvas(ctx, None);
    let size = graphics::drawable_size(ctx);
    graphics::set_screen_coordinates(ctx, Rect::new(0.0, 0.0, size.0, size.1))?;

    Ok(sprite_batch)
}
