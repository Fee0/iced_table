//! Edge fades that dissolve cell text running into a neighbouring column.
//!
//! `iced_wgpu` draws all of a layer's text after all of its meshes, so a fade
//! drawn next to the text it covers would end up beneath it. The fades are
//! therefore built as their own geometry, which the widget renders in a layer
//! above the cell text.

use iced::widget::canvas::{Frame, gradient};
use iced::{Color, Point, Rectangle, Size};

use crate::data_table::column::CellAlign;

/// Fades the edges of `content` that overflowing text leaves through, blending
/// into `background` over at most `width` pixels per edge.
///
/// Text that stops short of an edge never reaches that edge's fade, so cells
/// can be faded unconditionally instead of being measured.
pub(crate) fn draw(
    frame: &mut Frame,
    content: Rectangle,
    align: CellAlign,
    width: f32,
    background: Color,
) {
    // Capped so a narrow column keeps at least half of its text legible.
    let width = width.min(content.width / 2.0);
    if width <= 0.0 {
        return;
    }
    let left = content.x;
    let right = content.x + content.width;
    match align {
        CellAlign::Start => strip(frame, content, right - width, right, background),
        CellAlign::End => strip(frame, content, left + width, left, background),
        CellAlign::Center => {
            strip(frame, content, left + width, left, background);
            strip(frame, content, right - width, right, background);
        }
    }
}

/// A full-height strip of `content` ramping from transparent at `clear_x` to
/// opaque `background` at `solid_x`.
fn strip(frame: &mut Frame, content: Rectangle, clear_x: f32, solid_x: f32, background: Color) {
    // Keeping the hue while dropping alpha avoids the grey band an
    // interpolation through transparent black would leave mid-fade.
    let transparent = Color {
        a: 0.0,
        ..background
    };
    let ramp = gradient::Linear::new(
        Point::new(clear_x, content.y),
        Point::new(solid_x, content.y),
    )
    .add_stop(0.0, transparent)
    .add_stop(1.0, background);
    frame.fill_rectangle(
        Point::new(clear_x.min(solid_x), content.y),
        Size::new((solid_x - clear_x).abs(), content.height),
        ramp,
    );
}
