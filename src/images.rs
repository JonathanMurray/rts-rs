use ggez::conf::NumSamples;
use ggez::graphics::{Canvas, Color, DrawParam, Mesh, Rect};
use ggez::graphics::{Drawable, Image};
use ggez::{graphics, Context, GameError};

pub fn mesh_into_image(ctx: &mut Context, mesh: Mesh) -> Result<Image, GameError> {
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
    let original_screen_coordinates = graphics::screen_coordinates(ctx);
    graphics::set_screen_coordinates(ctx, Rect::new(0.0, 0.0, width, height))?;

    let transparent_bg = Color::new(0.0, 0.0, 0.0, 0.0);
    graphics::clear(ctx, transparent_bg);
    graphics::draw(ctx, &mesh, DrawParam::default())?;
    let image = canvas.to_image(ctx)?;

    // Change back drawing mode: draw to screen
    graphics::set_canvas(ctx, None);
    graphics::set_screen_coordinates(ctx, original_screen_coordinates)?;

    Ok(image)
}
