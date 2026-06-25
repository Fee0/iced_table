//! Styling for [`DataTable`](crate::DataTable).
//!
//! This module holds appearance only — no layout, event, or draw logic. It
//! follows the same `Catalog` pattern every built-in iced widget uses.

use iced::Color;
use iced::Theme;

use crate::data_table::cell::TextRole;

/// A boxed style function, the default [`Catalog::Class`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

/// The visual status of a row, used to resolve text and background colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// Neither hovered nor active.
    #[default]
    Regular,
    /// The cursor is over the row.
    Hovered,
    /// The row is the consumer-resolved active/selected row.
    Active,
}

/// The resolved appearance of a table.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Header strip background.
    pub header_background: Color,
    /// Header text color.
    pub header_text: Color,
    /// Default row background.
    pub row_background: Color,
    /// Optional zebra background for odd rows.
    pub row_background_alternate: Option<Color>,
    /// Background painted under a hovered row.
    pub hover_background: Color,
    /// Background painted under the active row.
    pub active_background: Color,
    /// Vertical column divider color.
    pub divider: Color,
    /// Indent guide color in the tree column.
    pub indent_guide: Color,
    /// [`TextRole::Primary`] color on a regular row.
    pub text_primary: Color,
    /// [`TextRole::Accent`] color on a regular row.
    pub text_accent: Color,
    /// [`TextRole::Muted`] color on a regular row.
    pub text_muted: Color,
    /// Text color used for every role on a hovered/active row.
    pub text_on_active: Color,
    /// Scrollbar track (the groove the thumb slides within).
    pub scrollbar_track: Color,
    /// Scrollbar thumb at rest.
    pub scrollbar_thumb: Color,
    /// Scrollbar thumb while hovered or dragged.
    pub scrollbar_thumb_hover: Color,
}

impl Style {
    /// Resolves a [`TextRole`] to a concrete color for the given row [`Status`].
    ///
    /// On a hovered or active row every role flips to `text_on_active`, so an
    /// accent or primary cell stays legible against the highlight fill.
    pub fn text_color(&self, role: TextRole, status: Status) -> Color {
        match status {
            Status::Hovered | Status::Active => self.text_on_active,
            Status::Regular => match role {
                TextRole::Primary => self.text_primary,
                TextRole::Accent => self.text_accent,
                TextRole::Muted => self.text_muted,
            },
        }
    }

    /// The background fill for a row in the given [`Status`], if any.
    ///
    /// `row_index` selects the zebra band for regular rows.
    pub fn row_background(&self, status: Status, row_index: usize) -> Option<Color> {
        match status {
            Status::Hovered => Some(self.hover_background),
            Status::Active => Some(self.active_background),
            Status::Regular => {
                if row_index % 2 == 1 {
                    self.row_background_alternate.or(Some(self.row_background))
                } else {
                    Some(self.row_background)
                }
            }
        }
    }
}

/// The theme-side catalog that resolves a class into a [`Style`].
pub trait Catalog {
    /// The style class stored by the widget.
    type Class<'a>;

    /// The default class.
    fn default<'a>() -> Self::Class<'a>;

    /// Resolves a class and status into a [`Style`].
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default table style, derived entirely from the theme's palette.
pub fn default(theme: &Theme, _status: Status) -> Style {
    let palette = theme.extended_palette();

    Style {
        header_background: palette.background.weak.color,
        header_text: palette.background.weak.text,
        row_background: palette.background.base.color,
        row_background_alternate: None,
        hover_background: palette.background.weak.color,
        active_background: palette.primary.strong.color,
        divider: palette.background.strong.color,
        indent_guide: muted(palette.background.strong.color, 0.5),
        text_primary: palette.background.base.text,
        text_accent: palette.primary.base.color,
        text_muted: muted(palette.background.base.text, 0.6),
        text_on_active: palette.primary.strong.text,
        scrollbar_track: palette.background.weak.color,
        scrollbar_thumb: muted(palette.background.strong.color, 0.7),
        scrollbar_thumb_hover: palette.background.strong.color,
    }
}

/// Scales a color's alpha to de-emphasize it without picking a new hue.
fn muted(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}
