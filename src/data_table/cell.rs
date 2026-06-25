//! Cell contents for a [`Row`](crate::Row).

use std::borrow::Cow;

/// One cell's worth of content. The widget resolves [`TextRole`] to a concrete
/// color through the active style, so consumers never pass raw colors.
#[derive(Debug, Clone)]
pub enum Cell<'a> {
    /// Renders nothing.
    Empty,
    /// Renders a single line of text.
    Text {
        /// The string to draw (borrowed or owned).
        text: Cow<'a, str>,
        /// Semantic role, resolved to a color together with the row's status.
        role: TextRole,
        /// Font weight.
        weight: Weight,
        /// Which font family to use.
        font_kind: FontKind,
    },
}

impl<'a> Cell<'a> {
    /// A primary, regular-weight, UI-font text cell.
    pub fn text(text: impl Into<Cow<'a, str>>) -> Self {
        Self::Text {
            text: text.into(),
            role: TextRole::Primary,
            weight: Weight::Regular,
            font_kind: FontKind::Ui,
        }
    }

    /// Overrides the text role.
    pub fn role(mut self, role: TextRole) -> Self {
        if let Self::Text { role: r, .. } = &mut self {
            *r = role;
        }
        self
    }

    /// Overrides the font weight.
    pub fn weight(mut self, weight: Weight) -> Self {
        if let Self::Text { weight: w, .. } = &mut self {
            *w = weight;
        }
        self
    }

    /// Overrides the font family.
    pub fn font_kind(mut self, font_kind: FontKind) -> Self {
        if let Self::Text { font_kind: f, .. } = &mut self {
            *f = font_kind;
        }
        self
    }
}

/// Semantic text color, resolved by the style together with the row's status.
///
/// A `Primary` or `Accent` cell automatically flips to the on-active color when
/// its row is hovered or active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextRole {
    /// Default foreground.
    #[default]
    Primary,
    /// Emphasized, accent-colored text (e.g. a type badge).
    Accent,
    /// De-emphasized text (e.g. an address).
    Muted,
}

/// Font weight for a text cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weight {
    /// Regular weight.
    #[default]
    Regular,
    /// Bold weight.
    Bold,
}

/// Which font family a text cell uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontKind {
    /// The proportional UI font.
    #[default]
    Ui,
    /// The monospace editor font (e.g. for addresses).
    Editor,
}
