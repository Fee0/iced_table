//! The chevron glyph shared by the tree toggle and the header sort indicator.

use iced::advanced::svg;
use iced::widget::canvas::{Frame, Path};
use iced::{Color, Point, Rectangle};

/// How far the flat edge of a vertical chevron sits from the box edge opposite
/// its tip, as a fraction of the box. Keeps the triangle from looking equilateral.
const BASE_INSET: f32 = 0.25;

/// Which way the chevron points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Right,
    Down,
    Up,
}

/// Draws a chevron filling `bounds`: `handle`'s artwork when one is supplied,
/// otherwise a filled triangle.
pub(crate) fn draw(
    frame: &mut Frame,
    bounds: Rectangle,
    direction: Direction,
    color: Color,
    handle: Option<&svg::Handle>,
) {
    if let Some(handle) = handle {
        frame.draw_svg(bounds, svg::Svg::new(handle.clone()).color(color));
        return;
    }

    let Rectangle {
        x,
        y,
        width,
        height,
    } = bounds;
    let path = Path::new(|builder| {
        match direction {
            Direction::Right => {
                builder.move_to(Point::new(x, y));
                builder.line_to(Point::new(x + width / 2.0, y + height / 2.0));
                builder.line_to(Point::new(x, y + height));
            }
            Direction::Down => {
                let base = y + height * BASE_INSET;
                builder.move_to(Point::new(x, base));
                builder.line_to(Point::new(x + width, base));
                builder.line_to(Point::new(x + width / 2.0, y + height));
            }
            Direction::Up => {
                let base = y + height * (1.0 - BASE_INSET);
                builder.move_to(Point::new(x, base));
                builder.line_to(Point::new(x + width, base));
                builder.line_to(Point::new(x + width / 2.0, y));
            }
        }
        builder.close();
    });
    frame.fill(&path, color);
}
