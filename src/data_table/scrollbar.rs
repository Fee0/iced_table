//! Pure scrollbar geometry shared by the vertical and horizontal bars.
//!
//! The table is an atomic canvas widget, so it hosts no child scrollable. This
//! module instead provides axis-agnostic geometry — used once per axis — plus a
//! small [`Frame`] draw helper, mirroring the render-light style of
//! [`geometry`](super::geometry).

use iced::widget::canvas::Frame;
use iced::{Color, Point, Rectangle, Size};

/// The axis a scrollbar scrolls along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    /// Scrolls vertically; the bar sits on the right edge.
    Vertical,
    /// Scrolls horizontally; the bar sits on the bottom edge.
    Horizontal,
}

impl Axis {
    /// The length of `rect` along this axis.
    fn length(self, rect: Rectangle) -> f32 {
        match self {
            Axis::Vertical => rect.height,
            Axis::Horizontal => rect.width,
        }
    }

    /// The leading coordinate of `rect` along this axis.
    fn start(self, rect: Rectangle) -> f32 {
        match self {
            Axis::Vertical => rect.y,
            Axis::Horizontal => rect.x,
        }
    }
}

/// True when content overflows the viewport on an axis (a bar is needed).
pub(crate) fn visible(content_len: f32, viewport_len: f32) -> bool {
    content_len > viewport_len
}

/// Resolved track + thumb rectangles for one axis, in widget-local coordinates.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scrollbar {
    /// The full track the thumb slides within.
    pub track: Rectangle,
    /// The draggable thumb.
    pub thumb: Rectangle,
}

impl Scrollbar {
    /// Computes the thumb for `axis` within `track`, given the total content
    /// length and the current scroll `offset`. Returns `None` when content fits.
    pub(crate) fn new(
        axis: Axis,
        track: Rectangle,
        content_len: f32,
        offset: f32,
        min_thumb: f32,
    ) -> Option<Self> {
        let viewport_len = axis.length(track);
        if viewport_len <= 0.0 || !visible(content_len, viewport_len) {
            return None;
        }

        let thumb_len = (viewport_len * viewport_len / content_len)
            .max(min_thumb)
            .min(viewport_len);
        let max_offset = content_len - viewport_len;
        let travel = viewport_len - thumb_len;
        let progress = if max_offset > 0.0 {
            (offset / max_offset).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let thumb_pos = progress * travel;

        let thumb = match axis {
            Axis::Vertical => Rectangle {
                x: track.x,
                y: track.y + thumb_pos,
                width: track.width,
                height: thumb_len,
            },
            Axis::Horizontal => Rectangle {
                x: track.x + thumb_pos,
                y: track.y,
                width: thumb_len,
                height: track.height,
            },
        };

        Some(Self { track, thumb })
    }

    /// The scroll offset that places the thumb's leading edge at `thumb_lead`
    /// (a widget-local coordinate along `axis`), clamped to `[0, max]`.
    ///
    /// The thumb length is taken from `self`, which is constant for the duration
    /// of a drag, so a stale thumb position does not affect the mapping.
    pub(crate) fn offset_for_thumb(&self, axis: Axis, content_len: f32, thumb_lead: f32) -> f32 {
        let viewport_len = axis.length(self.track);
        let thumb_len = axis.length(self.thumb);
        let travel = viewport_len - thumb_len;
        if travel <= 0.0 {
            return 0.0;
        }
        let local = (thumb_lead - axis.start(self.track)).clamp(0.0, travel);
        let max_offset = (content_len - viewport_len).max(0.0);
        local / travel * max_offset
    }
}

/// Fills the track and thumb of `bar`.
pub(crate) fn draw(frame: &mut Frame, bar: &Scrollbar, track: Color, thumb: Color) {
    frame.fill_rectangle(
        Point::new(bar.track.x, bar.track.y),
        Size::new(bar.track.width, bar.track.height),
        track,
    );
    frame.fill_rectangle(
        Point::new(bar.thumb.x, bar.thumb.y),
        Size::new(bar.thumb.width, bar.thumb.height),
        thumb,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(axis: Axis, len: f32) -> Rectangle {
        match axis {
            Axis::Vertical => Rectangle {
                x: 200.0,
                y: 30.0,
                width: 10.0,
                height: len,
            },
            Axis::Horizontal => Rectangle {
                x: 0.0,
                y: 400.0,
                width: len,
                height: 10.0,
            },
        }
    }

    #[test]
    fn no_bar_when_content_fits() {
        assert!(!visible(400.0, 500.0));
        assert!(
            Scrollbar::new(
                Axis::Vertical,
                track(Axis::Vertical, 500.0),
                400.0,
                0.0,
                24.0
            )
            .is_none()
        );
    }

    #[test]
    fn thumb_is_proportional_to_the_viewport() {
        let bar = Scrollbar::new(
            Axis::Vertical,
            track(Axis::Vertical, 500.0),
            1000.0,
            0.0,
            24.0,
        )
        .unwrap();
        assert!((bar.thumb.height - 250.0).abs() < 1e-3);
        assert!((bar.thumb.y - 30.0).abs() < 1e-3);
    }

    #[test]
    fn thumb_sits_at_the_track_end_at_max_offset() {
        let bar = Scrollbar::new(
            Axis::Vertical,
            track(Axis::Vertical, 500.0),
            1000.0,
            500.0,
            24.0,
        )
        .unwrap();
        // travel = 500 - 250 = 250, so the thumb ends at track.y + 250.
        assert!((bar.thumb.y - (30.0 + 250.0)).abs() < 1e-3);
    }

    #[test]
    fn tiny_thumb_is_clamped_to_min() {
        let bar = Scrollbar::new(
            Axis::Horizontal,
            track(Axis::Horizontal, 500.0),
            100_000.0,
            0.0,
            24.0,
        )
        .unwrap();
        assert!((bar.thumb.width - 24.0).abs() < 1e-3);
    }

    #[test]
    fn offset_round_trips_through_the_thumb_position() {
        let axis = Axis::Vertical;
        let content = 1000.0;
        let bar = Scrollbar::new(axis, track(axis, 500.0), content, 120.0, 24.0).unwrap();
        let recovered = bar.offset_for_thumb(axis, content, bar.thumb.y);
        assert!((recovered - 120.0).abs() < 1e-3);
    }
}
