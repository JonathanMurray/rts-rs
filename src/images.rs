use ggez::conf::NumSamples;
use ggez::graphics::{Canvas, Color, DrawParam, Rect};
use ggez::graphics::{Drawable, Image};
use ggez::{graphics, Context, GameError, GameResult};

pub fn mesh_into_image(ctx: &mut Context, drawable: impl Drawable) -> Result<Image, GameError> {
    let dimensions = drawable.dimensions(ctx).unwrap();
    drawable_into_image(ctx, dimensions, |ctx| {
        drawable.draw(ctx, DrawParam::default())
    })
}

pub fn drawable_into_image(
    ctx: &mut Context,
    dimensions: Rect,
    draw: impl FnOnce(&mut Context) -> GameResult,
) -> GameResult<Image> {
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
    let original_screen_coordinates = graphics::screen_coordinates(ctx);
    graphics::set_screen_coordinates(ctx, Rect::new(0.0, 0.0, width, height))?;

    let transparent_bg = Color::new(0.0, 0.0, 0.0, 0.0);
    graphics::clear(ctx, transparent_bg);
    draw(ctx)?;
    let image = canvas.to_image(ctx)?;

    // Change back drawing mode: draw to screen
    graphics::set_canvas(ctx, None);
    graphics::set_screen_coordinates(ctx, original_screen_coordinates)?;

    Ok(image)
}
