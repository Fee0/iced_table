//! A generic, canvas-rendered data table widget.
//!
//! See the [crate] documentation for the overall design. This module holds the
//! [`DataTable`] widget itself: its builder, persistent [`State`], and the
//! [`advanced::Widget`](iced::advanced::Widget) implementation that owns layout,
//! virtualization, column resizing, and hover/active highlighting.

pub mod cell;
pub mod column;
mod geometry;
pub mod row;
pub mod style;

use std::cell::RefCell;

use iced::advanced::Clipboard;
use iced::advanced::Renderer as _;
use iced::advanced::Shell;
use iced::advanced::Widget;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::text::Alignment as TextAlignment;
use iced::advanced::widget::{Tree, tree};
use iced::alignment::Vertical;
use iced::mouse;
use iced::widget::canvas::{Cache, Frame, Path, Text};
use iced::{Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Vector, font};

use crate::data_table::cell::{Cell, FontKind, TextRole, Weight};
use crate::data_table::column::{CellAlign, Column};
use crate::data_table::row::{Row, Toggle};
use crate::data_table::style::{Catalog, Status, Style, StyleFn};

const DEFAULT_ROW_HEIGHT: f32 = 24.0;
const DEFAULT_HEADER_HEIGHT: f32 = 28.0;
const DEFAULT_TEXT_SIZE: f32 = 13.0;

const CELL_PADDING_X: f32 = 8.0;
const INDENT_STEP: f32 = 14.0;
const CHEVRON_BOX: f32 = 16.0;
const CHEVRON_GLYPH: f32 = 8.0;
const DIVIDER_WIDTH: f32 = 1.0;
const INDENT_GUIDE_WIDTH: f32 = 1.0;

/// One scrolled wheel line, in pixels.
const SCROLL_LINE_HEIGHT: f32 = DEFAULT_ROW_HEIGHT;

/// A reusable, canvas-rendered table generic over its `Theme`.
///
/// The table is rebuilt every frame from consumer-provided columns and rows and
/// identifies rows/columns purely by index; the consumer maps an index back to
/// its own domain.
#[allow(clippy::type_complexity)]
pub struct DataTable<'a, Message, Theme = iced::Theme>
where
    Theme: Catalog,
{
    columns: Vec<Column>,
    rows: Vec<Row<'a>>,
    row_height: f32,
    header_height: f32,
    text_size: f32,
    active_row: Option<usize>,
    revision: u64,
    on_row_press: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_toggle_press: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_hover: Option<Box<dyn Fn(Option<usize>) -> Message + 'a>>,
    on_column_resize: Option<Box<dyn Fn(usize, f32) -> Message + 'a>>,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme> DataTable<'a, Message, Theme>
where
    Theme: Catalog,
{
    /// Creates a table from the given columns and (already-filtered, flat) rows.
    pub fn new(columns: Vec<Column>, rows: Vec<Row<'a>>) -> Self {
        Self {
            columns,
            rows,
            row_height: DEFAULT_ROW_HEIGHT,
            header_height: DEFAULT_HEADER_HEIGHT,
            text_size: DEFAULT_TEXT_SIZE,
            active_row: None,
            revision: 0,
            on_row_press: None,
            on_toggle_press: None,
            on_hover: None,
            on_column_resize: None,
            class: Theme::default(),
        }
    }

    /// Sets the per-row pixel height.
    pub fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    /// Sets the header strip pixel height.
    pub fn header_height(mut self, header_height: f32) -> Self {
        self.header_height = header_height;
        self
    }

    /// Sets the text size used for cells and headers.
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }

    /// Sets the consumer-resolved active (selected) row.
    pub fn active_row(mut self, active_row: Option<usize>) -> Self {
        self.active_row = active_row;
        self
    }

    /// Bumps to invalidate the cached row geometry when row content changes.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// Sets the callback fired when a row is pressed.
    pub fn on_row_press(mut self, callback: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_row_press = Some(Box::new(callback));
        self
    }

    /// Sets the collapse/expand hook fired when a chevron is pressed.
    pub fn on_toggle_press(mut self, callback: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_toggle_press = Some(Box::new(callback));
        self
    }

    /// Sets the callback fired when the hovered row changes.
    pub fn on_hover(mut self, callback: impl Fn(Option<usize>) -> Message + 'a) -> Self {
        self.on_hover = Some(Box::new(callback));
        self
    }

    /// Sets the callback fired while a column divider is dragged.
    pub fn on_column_resize(mut self, callback: impl Fn(usize, f32) -> Message + 'a) -> Self {
        self.on_column_resize = Some(Box::new(callback));
        self
    }

    /// Sets the style.
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    fn body_height(&self, bounds: Rectangle) -> f32 {
        (bounds.height - self.header_height).max(0.0)
    }

    /// The tree column index, if any column hosts the collapse affordance.
    fn tree_column(&self) -> Option<usize> {
        self.columns.iter().position(|column| column.tree_column)
    }

    /// The local hit rectangle of a row's chevron, if it has one.
    fn chevron_zone(&self, widths: &[f32], row_index: usize, top_y: f32) -> Option<Rectangle> {
        let row = self.rows.get(row_index)?;
        if row.toggle == Toggle::None {
            return None;
        }
        let tree_column = self.tree_column()?;
        let indent = f32::from(row.depth) * INDENT_STEP;
        let left = geometry::column_left(widths, tree_column) + CELL_PADDING_X + indent;
        Some(Rectangle {
            x: left,
            y: top_y,
            width: CHEVRON_BOX,
            height: self.row_height,
        })
    }
}

/// Persistent widget state, kept in the widget tree across the per-frame rebuild.
struct State {
    scroll_y: f32,
    hovered_row: Option<usize>,
    drag: Option<ColumnDrag>,
    cache_header: Cache,
    cache_rows: Cache,
    cache_highlight: Cache,
    keys: RefCell<CacheKeys>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scroll_y: 0.0,
            hovered_row: None,
            drag: None,
            cache_header: Cache::new(),
            cache_rows: Cache::new(),
            cache_highlight: Cache::new(),
            keys: RefCell::new(CacheKeys::stale()),
        }
    }
}

/// An in-progress column resize: the column being sized and its fixed left edge.
struct ColumnDrag {
    column: usize,
    left_edge: f32,
}

/// The inputs each cached layer was last drawn against, for invalidation.
struct CacheKeys {
    revision: u64,
    scroll_y: f32,
    size: Size,
    widths: Vec<f32>,
    hover: Option<usize>,
    active: Option<usize>,
}

impl CacheKeys {
    /// Keys that never match a real frame, forcing the first draw to populate.
    fn stale() -> Self {
        Self {
            revision: u64::MAX,
            scroll_y: f32::NAN,
            size: Size::ZERO,
            widths: Vec::new(),
            hover: None,
            active: None,
        }
    }
}

/// Borrowed context shared by the per-layer drawing routines.
struct Painter<'p> {
    style: &'p Style,
    columns: &'p [Column],
    widths: &'p [f32],
    row_height: f32,
    text_size: f32,
}

impl Painter<'_> {
    /// Draws a full row: its background fill, dividers-aware cells, chevron, and
    /// indent guides, all in the given [`Status`].
    fn row(&self, frame: &mut Frame, row: &Row, row_index: usize, top_y: f32, status: Status) {
        if let Some(background) = self.style.row_background(status, row_index) {
            frame.fill_rectangle(
                Point::new(0.0, top_y),
                Size::new(self.total_width(), self.row_height),
                background,
            );
        }

        let center_y = top_y + self.row_height / 2.0;
        for (index, column) in self.columns.iter().enumerate() {
            let left = geometry::column_left(self.widths, index);
            let width = self.widths[index];
            if column.tree_column {
                self.tree_cell(frame, row, index, center_y, status);
            } else {
                self.cell(
                    frame,
                    &row.cells[index],
                    column.align,
                    left,
                    width,
                    center_y,
                    status,
                );
            }
        }
    }

    fn tree_cell(&self, frame: &mut Frame, row: &Row, index: usize, center_y: f32, status: Status) {
        let left = geometry::column_left(self.widths, index);
        let width = self.widths[index];
        let cell = &row.cells[index];
        let indent = f32::from(row.depth) * INDENT_STEP;
        let content_left = left + CELL_PADDING_X + indent;

        self.indent_guides(frame, left, row.depth, center_y);

        if row.toggle != Toggle::None {
            let color = self.style.text_color(TextRole::Primary, status);
            let glyph_left = content_left + (CHEVRON_BOX - CHEVRON_GLYPH) / 2.0;
            draw_chevron(
                frame,
                glyph_left,
                center_y,
                row.toggle == Toggle::Expanded,
                color,
            );
        }

        let text_left = content_left + CHEVRON_BOX;
        let available = (left + width - CELL_PADDING_X - text_left).max(0.0);
        self.text(
            frame,
            cell,
            text_left,
            available,
            TextAlignment::Left,
            center_y,
            status,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn cell(
        &self,
        frame: &mut Frame,
        cell: &Cell,
        align: CellAlign,
        left: f32,
        width: f32,
        center_y: f32,
        status: Status,
    ) {
        let inner = (width - 2.0 * CELL_PADDING_X).max(0.0);
        let (x, alignment) = match align {
            CellAlign::Start => (left + CELL_PADDING_X, TextAlignment::Left),
            CellAlign::Center => (left + width / 2.0, TextAlignment::Center),
            CellAlign::End => (left + width - CELL_PADDING_X, TextAlignment::Right),
        };
        self.text(frame, cell, x, inner, alignment, center_y, status);
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &self,
        frame: &mut Frame,
        cell: &Cell,
        x: f32,
        max_width: f32,
        alignment: TextAlignment,
        center_y: f32,
        status: Status,
    ) {
        let Cell::Text {
            text,
            role,
            weight,
            font_kind,
        } = cell
        else {
            return;
        };
        frame.fill_text(Text {
            content: text.to_string(),
            position: Point::new(x, center_y),
            color: self.style.text_color(*role, status),
            size: Pixels(self.text_size),
            font: font_for(*font_kind, *weight),
            align_x: alignment,
            align_y: Vertical::Center,
            max_width,
            ..Text::default()
        });
    }

    fn indent_guides(&self, frame: &mut Frame, cell_left: f32, depth: u16, center_y: f32) {
        for level in 1..=depth {
            let x = cell_left + CELL_PADDING_X + f32::from(level) * INDENT_STEP;
            frame.fill_rectangle(
                Point::new(x, center_y - self.row_height / 2.0),
                Size::new(INDENT_GUIDE_WIDTH, self.row_height),
                self.style.indent_guide,
            );
        }
    }

    /// Vertical column dividers spanning `[top, top + height]`.
    fn dividers(&self, frame: &mut Frame, top: f32, height: f32) {
        let mut edge = 0.0;
        for width in &self.widths[..self.widths.len().saturating_sub(1)] {
            edge += width;
            frame.fill_rectangle(
                Point::new(edge - DIVIDER_WIDTH / 2.0, top),
                Size::new(DIVIDER_WIDTH, height),
                self.style.divider,
            );
        }
    }

    fn total_width(&self) -> f32 {
        self.widths.iter().sum()
    }
}

/// Draws a filled chevron triangle centered vertically on `center_y`.
fn draw_chevron(frame: &mut Frame, x: f32, center_y: f32, expanded: bool, color: Color) {
    let path = Path::new(|builder| {
        if expanded {
            builder.move_to(Point::new(x, center_y - CHEVRON_GLYPH / 4.0));
            builder.line_to(Point::new(
                x + CHEVRON_GLYPH,
                center_y - CHEVRON_GLYPH / 4.0,
            ));
            builder.line_to(Point::new(
                x + CHEVRON_GLYPH / 2.0,
                center_y + CHEVRON_GLYPH / 2.0,
            ));
        } else {
            builder.move_to(Point::new(x, center_y - CHEVRON_GLYPH / 2.0));
            builder.line_to(Point::new(x + CHEVRON_GLYPH / 2.0, center_y));
            builder.line_to(Point::new(x, center_y + CHEVRON_GLYPH / 2.0));
        }
        builder.close();
    });
    frame.fill(&path, color);
}

/// Resolves a [`FontKind`] and [`Weight`] to a concrete [`Font`].
fn font_for(kind: FontKind, weight: Weight) -> Font {
    let mut resolved = match kind {
        FontKind::Ui => Font::DEFAULT,
        FontKind::Editor => Font::MONOSPACE,
    };
    if weight == Weight::Bold {
        resolved.weight = font::Weight::Bold;
    }
    resolved
}

impl<'a, Message, Theme> Widget<Message, Theme, iced::Renderer> for DataTable<'a, Message, Theme>
where
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fill)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }

        let state = tree.state.downcast_ref::<State>();
        let resolved = <Theme as Catalog>::style(theme, &self.class, Status::Regular);
        let widths = geometry::distribute_widths(&self.columns, bounds.width);

        let scroll_y = state.scroll_y.min(geometry::max_scroll(
            self.rows.len(),
            self.row_height,
            self.body_height(bounds),
        ));

        self.reconcile_caches(state, bounds.size(), &widths, scroll_y);

        let painter = Painter {
            style: &resolved,
            columns: &self.columns,
            widths: &widths,
            row_height: self.row_height,
            text_size: self.text_size,
        };

        let header = state.cache_header.draw(renderer, bounds.size(), |frame| {
            self.draw_header(frame, &painter, bounds)
        });
        let rows = state.cache_rows.draw(renderer, bounds.size(), |frame| {
            self.draw_rows(frame, &painter, bounds, scroll_y)
        });
        let highlight = state
            .cache_highlight
            .draw(renderer, bounds.size(), |frame| {
                self.draw_highlight(frame, &painter, bounds, scroll_y, state.hovered_row)
            });

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(header);
            renderer.draw_geometry(rows);
            renderer.draw_geometry(highlight);
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let widths = geometry::distribute_widths(&self.columns, bounds.width);
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.position_over(bounds).is_none() {
                    return;
                }
                let amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * SCROLL_LINE_HEIGHT,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
                let max = geometry::max_scroll(
                    self.rows.len(),
                    self.row_height,
                    self.body_height(bounds),
                );
                let next = (state.scroll_y - amount).clamp(0.0, max);
                if next != state.scroll_y {
                    state.scroll_y = next;
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(drag) = &state.drag {
                    let Some(position) = cursor.position() else {
                        return;
                    };
                    let min_width = self.columns[drag.column].min_width;
                    let new_width = (position.x - bounds.x - drag.left_edge).max(min_width);
                    if let Some(callback) = &self.on_column_resize {
                        shell.publish(callback(drag.column, new_width));
                    }
                    shell.request_redraw();
                    return;
                }

                let next = cursor.position_in(bounds).and_then(|position| {
                    geometry::row_at(
                        position.y,
                        self.header_height,
                        self.row_height,
                        state.scroll_y,
                        self.rows.len(),
                    )
                });
                if next != state.hovered_row {
                    state.hovered_row = next;
                    if let Some(callback) = &self.on_hover {
                        shell.publish(callback(next));
                    }
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_in(bounds) else {
                    return;
                };

                if let Some(column) =
                    geometry::divider_at(&widths, &self.columns, position.x, geometry::DIVIDER_GRAB)
                {
                    state.drag = Some(ColumnDrag {
                        column,
                        left_edge: geometry::column_left(&widths, column),
                    });
                    shell.capture_event();
                    return;
                }

                let Some(index) = geometry::row_at(
                    position.y,
                    self.header_height,
                    self.row_height,
                    state.scroll_y,
                    self.rows.len(),
                ) else {
                    return;
                };

                let top_y = self.header_height + index as f32 * self.row_height - state.scroll_y;
                if let Some(zone) = self.chevron_zone(&widths, index, top_y)
                    && zone.contains(position)
                {
                    if let Some(callback) = &self.on_toggle_press {
                        shell.publish(callback(index));
                        shell.capture_event();
                    }
                    return;
                }

                if let Some(callback) = &self.on_row_press {
                    shell.publish(callback(index));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag.is_some() =>
            {
                state.drag = None;
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.drag.is_some() {
            return mouse::Interaction::ResizingHorizontally;
        }

        let bounds = layout.bounds();
        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };

        let widths = geometry::distribute_widths(&self.columns, bounds.width);
        if geometry::divider_at(&widths, &self.columns, position.x, geometry::DIVIDER_GRAB)
            .is_some()
        {
            return mouse::Interaction::ResizingHorizontally;
        }

        let over_row = geometry::row_at(
            position.y,
            self.header_height,
            self.row_height,
            state.scroll_y,
            self.rows.len(),
        )
        .is_some();
        if over_row && (self.on_row_press.is_some() || self.on_toggle_press.is_some()) {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::Idle
    }
}

impl<'a, Message, Theme> DataTable<'a, Message, Theme>
where
    Theme: Catalog,
{
    /// Clears any cached layer whose inputs changed since the last draw.
    fn reconcile_caches(&self, state: &State, size: Size, widths: &[f32], scroll_y: f32) {
        let mut keys = state.keys.borrow_mut();

        let rows_dirty = keys.revision != self.revision
            || keys.size != size
            || keys.scroll_y != scroll_y
            || keys.widths != widths;
        let header_dirty = keys.size != size || keys.widths != widths;
        let highlight_dirty =
            rows_dirty || keys.hover != state.hovered_row || keys.active != self.active_row;

        if rows_dirty {
            state.cache_rows.clear();
        }
        if header_dirty {
            state.cache_header.clear();
        }
        if highlight_dirty {
            state.cache_highlight.clear();
        }

        *keys = CacheKeys {
            revision: self.revision,
            scroll_y,
            size,
            widths: widths.to_vec(),
            hover: state.hovered_row,
            active: self.active_row,
        };
    }

    fn draw_header(&self, frame: &mut Frame, painter: &Painter, bounds: Rectangle) {
        frame.fill_rectangle(
            Point::ORIGIN,
            Size::new(bounds.width, self.header_height),
            painter.style.header_background,
        );

        let header_style = Style {
            text_primary: painter.style.header_text,
            ..*painter.style
        };
        let header_painter = Painter {
            style: &header_style,
            ..*painter
        };

        let center_y = self.header_height / 2.0;
        for (index, column) in self.columns.iter().enumerate() {
            let left = geometry::column_left(painter.widths, index);
            let width = painter.widths[index];
            let header_cell = Cell::text(column.header.clone());
            header_painter.cell(
                frame,
                &header_cell,
                column.align,
                left,
                width,
                center_y,
                Status::Regular,
            );
        }

        painter.dividers(frame, 0.0, self.header_height);
    }

    fn draw_rows(&self, frame: &mut Frame, painter: &Painter, bounds: Rectangle, scroll_y: f32) {
        let body = Rectangle {
            x: 0.0,
            y: self.header_height,
            width: bounds.width,
            height: self.body_height(bounds),
        };
        frame.with_clip(body, |frame| {
            let range = geometry::visible_rows(
                scroll_y,
                self.body_height(bounds),
                self.row_height,
                self.rows.len(),
            );
            for index in range {
                let top_y = self.header_height + index as f32 * self.row_height - scroll_y;
                painter.row(frame, &self.rows[index], index, top_y, Status::Regular);
            }
            painter.dividers(frame, self.header_height, self.body_height(bounds));
        });
    }

    fn draw_highlight(
        &self,
        frame: &mut Frame,
        painter: &Painter,
        bounds: Rectangle,
        scroll_y: f32,
        hovered_row: Option<usize>,
    ) {
        let body = Rectangle {
            x: 0.0,
            y: self.header_height,
            width: bounds.width,
            height: self.body_height(bounds),
        };
        let row_count = self.rows.len();
        let active = self.active_row.filter(|&index| index < row_count);
        let hovered = hovered_row.filter(|&index| index < row_count);

        frame.with_clip(body, |frame| {
            // Active first so a row that is both hovered and active shows hover on top.
            if let Some(index) = active.filter(|index| Some(*index) != hovered) {
                let top_y = self.header_height + index as f32 * self.row_height - scroll_y;
                painter.row(frame, &self.rows[index], index, top_y, Status::Active);
            }
            if let Some(index) = hovered {
                let top_y = self.header_height + index as f32 * self.row_height - scroll_y;
                painter.row(frame, &self.rows[index], index, top_y, Status::Hovered);
            }
        });
    }
}

impl<'a, Message, Theme> From<DataTable<'a, Message, Theme>> for Element<'a, Message, Theme>
where
    Message: 'a,
    Theme: 'a + Catalog,
{
    fn from(table: DataTable<'a, Message, Theme>) -> Self {
        Element::new(table)
    }
}
