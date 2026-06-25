//! A generic, canvas-rendered data table widget.
//!
//! See the [crate] documentation for the overall design. This module holds the
//! [`DataTable`] widget itself: its builder, persistent [`State`], and the
//! [`advanced::Widget`](iced::advanced::Widget) implementation that owns layout,
//! virtualization, column resizing, scrolling, and hover/active highlighting.

pub mod cell;
pub mod column;
mod geometry;
pub mod row;
mod scrollbar;
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
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{Cache, Frame, Path, Text};
use iced::{Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Vector, font};

use crate::data_table::cell::{Cell, FontKind, TextRole, Weight};
use crate::data_table::column::{CellAlign, Column};
use crate::data_table::row::{Row, Toggle};
use crate::data_table::scrollbar::{Axis, Scrollbar};
use crate::data_table::style::{Catalog, Status, Style, StyleFn};

const DEFAULT_ROW_HEIGHT: f32 = 24.0;
const DEFAULT_HEADER_HEIGHT: f32 = 28.0;
const DEFAULT_TEXT_SIZE: f32 = 13.0;
const DEFAULT_CELL_PADDING_X: f32 = 8.0;
const DEFAULT_INDENT_STEP: f32 = 14.0;
const DEFAULT_CHEVRON_BOX: f32 = 16.0;
const DEFAULT_CHEVRON_GLYPH: f32 = 8.0;
const DEFAULT_SCROLLBAR_THICKNESS: f32 = 10.0;
const DEFAULT_SCROLLBAR_MIN_THUMB: f32 = 24.0;
const DEFAULT_DIVIDER_GRAB: f32 = 4.0;
const DEFAULT_DIVIDER_WIDTH: f32 = 1.0;
const DEFAULT_INDENT_GUIDE_WIDTH: f32 = 1.0;

/// A reusable, canvas-rendered table generic over its `Theme`.
///
/// The table is rebuilt every frame from consumer-provided columns and rows and
/// identifies rows/columns purely by index; the consumer maps an index back to
/// its own domain. The widget owns the live column widths (seeded from each
/// column's preferred `width`) and adjusts them as the header dividers are
/// dragged.
pub struct DataTable<'a, Message, Theme = iced::Theme>
where
    Theme: Catalog,
{
    columns: Vec<Column>,
    rows: Vec<Row<'a>>,
    row_height: f32,
    header_height: f32,
    text_size: f32,
    cell_padding_x: f32,
    indent_step: f32,
    chevron_box: f32,
    chevron_glyph: f32,
    scrollbar_thickness: f32,
    scrollbar_min_thumb: f32,
    divider_grab: f32,
    divider_width: f32,
    indent_guide_width: f32,
    scroll_line_height: f32,
    active_row: Option<usize>,
    revision: u64,
    on_row_press: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_toggle_press: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_hover: Option<Box<dyn Fn(Option<usize>) -> Message + 'a>>,
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
            cell_padding_x: DEFAULT_CELL_PADDING_X,
            indent_step: DEFAULT_INDENT_STEP,
            chevron_box: DEFAULT_CHEVRON_BOX,
            chevron_glyph: DEFAULT_CHEVRON_GLYPH,
            scrollbar_thickness: DEFAULT_SCROLLBAR_THICKNESS,
            scrollbar_min_thumb: DEFAULT_SCROLLBAR_MIN_THUMB,
            divider_grab: DEFAULT_DIVIDER_GRAB,
            divider_width: DEFAULT_DIVIDER_WIDTH,
            indent_guide_width: DEFAULT_INDENT_GUIDE_WIDTH,
            scroll_line_height: DEFAULT_ROW_HEIGHT,
            active_row: None,
            revision: 0,
            on_row_press: None,
            on_toggle_press: None,
            on_hover: None,
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

    /// Sets horizontal padding inside each cell.
    pub fn cell_padding_x(mut self, cell_padding_x: f32) -> Self {
        self.cell_padding_x = cell_padding_x;
        self
    }

    /// Sets the pixel step per tree depth level.
    pub fn indent_step(mut self, indent_step: f32) -> Self {
        self.indent_step = indent_step;
        self
    }

    /// Sets the bounding box size reserved for the chevron icon.
    pub fn chevron_box(mut self, chevron_box: f32) -> Self {
        self.chevron_box = chevron_box;
        self
    }

    /// Sets the rendered size of the chevron triangle glyph.
    pub fn chevron_glyph(mut self, chevron_glyph: f32) -> Self {
        self.chevron_glyph = chevron_glyph;
        self
    }

    /// Sets the scrollbar track thickness (width for vertical, height for horizontal).
    pub fn scrollbar_thickness(mut self, scrollbar_thickness: f32) -> Self {
        self.scrollbar_thickness = scrollbar_thickness;
        self
    }

    /// Sets the minimum scrollbar thumb length so it stays grabbable with huge content.
    pub fn scrollbar_min_thumb(mut self, scrollbar_min_thumb: f32) -> Self {
        self.scrollbar_min_thumb = scrollbar_min_thumb;
        self
    }

    /// Sets the half-extent hit zone on each side of a column divider for resize dragging.
    pub fn divider_grab(mut self, divider_grab: f32) -> Self {
        self.divider_grab = divider_grab;
        self
    }

    /// Sets the column separator line thickness in pixels.
    pub fn divider_width(mut self, divider_width: f32) -> Self {
        self.divider_width = divider_width;
        self
    }

    /// Sets the tree indent guide line thickness in pixels.
    pub fn indent_guide_width(mut self, indent_guide_width: f32) -> Self {
        self.indent_guide_width = indent_guide_width;
        self
    }

    /// Sets the pixel distance scrolled per wheel line event.
    pub fn scroll_line_height(mut self, scroll_line_height: f32) -> Self {
        self.scroll_line_height = scroll_line_height;
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

    /// The persisted basis widths, falling back to the columns' preferred widths
    /// when the stored basis is stale (e.g. the column count changed).
    fn basis_for(&self, state: &State) -> Vec<f32> {
        if state.basis.len() == self.columns.len() {
            state.basis.clone()
        } else {
            self.columns.iter().map(|column| column.width).collect()
        }
    }

    /// The per-frame layout metrics shared by drawing and event handling.
    fn metrics(&self, state: &State) -> Metrics {
        let mins: Vec<f32> = self.columns.iter().map(|column| column.min_width).collect();
        let basis = self.basis_for(state);
        Metrics {
            widths: geometry::fit_widths(&basis, &mins, state.viewport.width),
            content_width: geometry::content_width(&mins, state.viewport.width),
            content_height: self.rows.len() as f32 * self.row_height,
            mins,
        }
    }

    /// The clamped scroll offsets for the given metrics.
    fn scroll_offsets(&self, state: &State, metrics: &Metrics) -> (f32, f32) {
        let body_height = (state.viewport.height - self.header_height).max(0.0);
        let max_x = geometry::max_scroll_x(metrics.content_width, state.viewport.width);
        let max_y = geometry::max_scroll(self.rows.len(), self.row_height, body_height);
        (
            state.scroll_x.clamp(0.0, max_x),
            state.scroll_y.clamp(0.0, max_y),
        )
    }

    /// The resolved vertical and horizontal scrollbars, each present only when
    /// its axis overflows. Coordinates are widget-local (origin at the top-left).
    fn scrollbars(
        &self,
        size: Size,
        metrics: &Metrics,
        scroll_x: f32,
        scroll_y: f32,
    ) -> (Option<Scrollbar>, Option<Scrollbar>) {
        let body_height = (size.height - self.header_height).max(0.0);
        let v_needed = scrollbar::visible(metrics.content_height, body_height);
        let h_needed = scrollbar::visible(metrics.content_width, size.width);

        let v_height = body_height
            - if h_needed {
                self.scrollbar_thickness
            } else {
                0.0
            };
        let h_width = size.width
            - if v_needed {
                self.scrollbar_thickness
            } else {
                0.0
            };

        let vertical = v_needed
            .then(|| {
                Scrollbar::new(
                    Axis::Vertical,
                    Rectangle {
                        x: size.width - self.scrollbar_thickness,
                        y: self.header_height,
                        width: self.scrollbar_thickness,
                        height: v_height,
                    },
                    metrics.content_height,
                    scroll_y,
                    self.scrollbar_min_thumb,
                )
            })
            .flatten();
        let horizontal = h_needed
            .then(|| {
                Scrollbar::new(
                    Axis::Horizontal,
                    Rectangle {
                        x: 0.0,
                        y: size.height - self.scrollbar_thickness,
                        width: h_width,
                        height: self.scrollbar_thickness,
                    },
                    metrics.content_width,
                    scroll_x,
                    self.scrollbar_min_thumb,
                )
            })
            .flatten();

        (vertical, horizontal)
    }

    /// Whether the columns currently have slack to redistribute, i.e. the
    /// viewport is wide enough to honor every minimum width.
    fn columns_resizable(&self, metrics: &Metrics, viewport_width: f32) -> bool {
        metrics.content_width <= viewport_width + 0.5
    }

    /// The tree column index, if any column hosts the collapse affordance.
    fn tree_column(&self) -> Option<usize> {
        self.columns.iter().position(|column| column.tree_column)
    }

    /// The local hit rectangle of a row's chevron in content space (the x is the
    /// unscrolled column offset), if it has one.
    fn chevron_zone(&self, widths: &[f32], row_index: usize, top_y: f32) -> Option<Rectangle> {
        let row = self.rows.get(row_index)?;
        if row.toggle == Toggle::None {
            return None;
        }
        let tree_column = self.tree_column()?;
        let indent = f32::from(row.depth) * self.indent_step;
        let left = geometry::column_left(widths, tree_column) + self.cell_padding_x + indent;
        Some(Rectangle {
            x: left,
            y: top_y,
            width: self.chevron_box,
            height: self.row_height,
        })
    }
}

/// Per-frame layout metrics derived from the columns and the viewport.
struct Metrics {
    /// The fitted display widths (sum to [`Metrics::content_width`]).
    widths: Vec<f32>,
    /// Each column's minimum width.
    mins: Vec<f32>,
    /// The horizontal content extent.
    content_width: f32,
    /// The vertical content extent.
    content_height: f32,
}

/// Persistent widget state, kept in the widget tree across the per-frame rebuild.
struct State {
    scroll_x: f32,
    scroll_y: f32,
    viewport: Size,
    hovered_row: Option<usize>,
    hovered_thumb: Option<Axis>,
    shift_held: bool,
    basis: Vec<f32>,
    drag: Option<Drag>,
    cache_header: Cache,
    cache_rows: Cache,
    cache_highlight: Cache,
    cache_overlay: Cache,
    keys: RefCell<CacheKeys>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scroll_x: 0.0,
            scroll_y: 0.0,
            viewport: Size::ZERO,
            hovered_row: None,
            hovered_thumb: None,
            shift_held: false,
            basis: Vec::new(),
            drag: None,
            cache_header: Cache::new(),
            cache_rows: Cache::new(),
            cache_highlight: Cache::new(),
            cache_overlay: Cache::new(),
            keys: RefCell::new(CacheKeys::stale()),
        }
    }
}

impl State {
    /// Reseeds the basis widths from the columns when the stored basis is stale.
    fn ensure_basis(&mut self, columns: &[Column]) {
        if self.basis.len() != columns.len() {
            self.basis = columns.iter().map(|column| column.width).collect();
        }
    }
}

/// An in-progress drag: either a column border or a scrollbar thumb.
enum Drag {
    /// Resizing the internal border on the right edge of column `border`.
    Column {
        border: usize,
        /// Updated to the new widths after every mouse-move frame (see
        /// [`geometry::resize_columns`] snapshot contract).
        snapshot: Vec<f32>,
        /// Cursor-to-divider offset captured at press time, so the divider
        /// tracks the pointer exactly rather than jumping to it.
        grab_dx: f32,
    },
    /// Dragging a scrollbar thumb; `grab` is the pointer offset within the thumb.
    Scroll { axis: Axis, grab: f32 },
}

/// The inputs each cached layer was last drawn against, for invalidation.
struct CacheKeys {
    revision: u64,
    scroll_x: f32,
    scroll_y: f32,
    size: Size,
    widths: Vec<f32>,
    content_width: f32,
    hover: Option<usize>,
    active: Option<usize>,
    hovered_thumb: Option<Axis>,
}

impl CacheKeys {
    /// Keys that never match a real frame, forcing the first draw to populate.
    fn stale() -> Self {
        Self {
            revision: u64::MAX,
            scroll_x: f32::NAN,
            scroll_y: f32::NAN,
            size: Size::ZERO,
            widths: Vec::new(),
            content_width: f32::NAN,
            hover: None,
            active: None,
            hovered_thumb: None,
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
    cell_padding_x: f32,
    indent_step: f32,
    chevron_box: f32,
    chevron_glyph: f32,
    divider_width: f32,
    indent_guide_width: f32,
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
        let indent = f32::from(row.depth) * self.indent_step;
        let content_left = left + self.cell_padding_x + indent;

        self.indent_guides(frame, left, row.depth, center_y);

        if row.toggle != Toggle::None {
            let color = self.style.text_color(TextRole::Primary, status);
            let glyph_left = content_left + (self.chevron_box - self.chevron_glyph) / 2.0;
            draw_chevron(
                frame,
                glyph_left,
                center_y,
                row.toggle == Toggle::Expanded,
                color,
                self.chevron_glyph,
            );
        }

        let text_left = content_left + self.chevron_box;
        let available = (left + width - self.cell_padding_x - text_left).max(0.0);
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
        let inner = (width - 2.0 * self.cell_padding_x).max(0.0);
        let (x, alignment) = match align {
            CellAlign::Start => (left + self.cell_padding_x, TextAlignment::Left),
            CellAlign::Center => (left + width / 2.0, TextAlignment::Center),
            CellAlign::End => (left + width - self.cell_padding_x, TextAlignment::Right),
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
            let x = cell_left + self.cell_padding_x + f32::from(level) * self.indent_step;
            frame.fill_rectangle(
                Point::new(x, center_y - self.row_height / 2.0),
                Size::new(self.indent_guide_width, self.row_height),
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
                Point::new(edge - self.divider_width / 2.0, top),
                Size::new(self.divider_width, height),
                self.style.divider,
            );
        }
    }

    fn total_width(&self) -> f32 {
        self.widths.iter().sum()
    }
}

/// Draws a filled chevron triangle centered vertically on `center_y`.
fn draw_chevron(
    frame: &mut Frame,
    x: f32,
    center_y: f32,
    expanded: bool,
    color: Color,
    glyph_size: f32,
) {
    let path = Path::new(|builder| {
        if expanded {
            builder.move_to(Point::new(x, center_y - glyph_size / 4.0));
            builder.line_to(Point::new(x + glyph_size, center_y - glyph_size / 4.0));
            builder.line_to(Point::new(
                x + glyph_size / 2.0,
                center_y + glyph_size / 2.0,
            ));
        } else {
            builder.move_to(Point::new(x, center_y - glyph_size / 2.0));
            builder.line_to(Point::new(x + glyph_size / 2.0, center_y));
            builder.line_to(Point::new(x, center_y + glyph_size / 2.0));
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

        let metrics = self.metrics(state);
        let (scroll_x, scroll_y) = self.scroll_offsets(state, &metrics);

        self.reconcile_caches(state, bounds.size(), &metrics, scroll_x, scroll_y);

        let painter = Painter {
            style: &resolved,
            columns: &self.columns,
            widths: &metrics.widths,
            row_height: self.row_height,
            text_size: self.text_size,
            cell_padding_x: self.cell_padding_x,
            indent_step: self.indent_step,
            chevron_box: self.chevron_box,
            chevron_glyph: self.chevron_glyph,
            divider_width: self.divider_width,
            indent_guide_width: self.indent_guide_width,
        };

        let header = state.cache_header.draw(renderer, bounds.size(), |frame| {
            self.draw_header(frame, &painter, bounds, scroll_x);
        });
        let rows = state.cache_rows.draw(renderer, bounds.size(), |frame| {
            self.draw_rows(frame, &painter, bounds, scroll_x, scroll_y);
        });
        let highlight = state
            .cache_highlight
            .draw(renderer, bounds.size(), |frame| {
                self.draw_highlight(
                    frame,
                    &painter,
                    bounds,
                    scroll_x,
                    scroll_y,
                    state.hovered_row,
                );
            });
        let overlay = state.cache_overlay.draw(renderer, bounds.size(), |frame| {
            self.draw_overlay(
                frame,
                &resolved,
                bounds.size(),
                &metrics,
                scroll_x,
                scroll_y,
                state,
            );
        });

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(header);
            renderer.draw_geometry(rows);
            renderer.draw_geometry(highlight);
            renderer.draw_geometry(overlay);
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
        let state = tree.state.downcast_mut::<State>();
        state.viewport = bounds.size();
        state.ensure_basis(&self.columns);

        let metrics = self.metrics(state);
        let (scroll_x, scroll_y) = self.scroll_offsets(state, &metrics);

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.shift_held = modifiers.shift();
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.position_over(bounds).is_none() {
                    return;
                }
                let (mut dx, mut dy) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => {
                        (x * self.scroll_line_height, y * self.scroll_line_height)
                    }
                    mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                };
                if state.shift_held && dx == 0.0 {
                    dx = dy;
                    dy = 0.0;
                }

                let body_height = self.body_height(bounds);
                let max_x = geometry::max_scroll_x(metrics.content_width, bounds.width);
                let max_y = geometry::max_scroll(self.rows.len(), self.row_height, body_height);
                let next_x = (scroll_x - dx).clamp(0.0, max_x);
                let next_y = (scroll_y - dy).clamp(0.0, max_y);
                if next_x != state.scroll_x || next_y != state.scroll_y {
                    state.scroll_x = next_x;
                    state.scroll_y = next_y;
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => match &mut state.drag {
                Some(Drag::Column {
                    border,
                    snapshot,
                    grab_dx,
                }) => {
                    let Some(position) = cursor.position_in(bounds) else {
                        return;
                    };
                    let desired = position.x + scroll_x - *grab_dx;
                    let widths =
                        geometry::resize_columns(snapshot, &metrics.mins, *border, desired);
                    *snapshot = widths.clone();
                    state.basis = widths;
                    shell.request_redraw();
                }
                Some(Drag::Scroll { axis, grab }) => {
                    let axis = *axis;
                    let grab = *grab;
                    let Some(position) = cursor.position_in(bounds) else {
                        return;
                    };
                    let (vertical, horizontal) =
                        self.scrollbars(bounds.size(), &metrics, scroll_x, scroll_y);
                    let bar = match axis {
                        Axis::Vertical => vertical,
                        Axis::Horizontal => horizontal,
                    };
                    if let Some(bar) = bar {
                        let (lead, content_len) = match axis {
                            Axis::Vertical => (position.y - grab, metrics.content_height),
                            Axis::Horizontal => (position.x - grab, metrics.content_width),
                        };
                        let offset = bar.offset_for_thumb(axis, content_len, lead);
                        match axis {
                            Axis::Vertical => state.scroll_y = offset,
                            Axis::Horizontal => state.scroll_x = offset,
                        }
                        state.hovered_thumb = Some(axis);
                        shell.request_redraw();
                    }
                }
                None => {
                    self.handle_hover(state, &metrics, scroll_x, scroll_y, bounds, cursor, shell);
                }
            },
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_in(bounds) else {
                    return;
                };

                let (vertical, horizontal) =
                    self.scrollbars(bounds.size(), &metrics, scroll_x, scroll_y);
                if let Some(bar) = vertical
                    && bar.thumb.contains(position)
                {
                    state.drag = Some(Drag::Scroll {
                        axis: Axis::Vertical,
                        grab: position.y - bar.thumb.y,
                    });
                    state.hovered_thumb = Some(Axis::Vertical);
                    shell.capture_event();
                    return;
                }
                if let Some(bar) = horizontal
                    && bar.thumb.contains(position)
                {
                    state.drag = Some(Drag::Scroll {
                        axis: Axis::Horizontal,
                        grab: position.x - bar.thumb.x,
                    });
                    state.hovered_thumb = Some(Axis::Horizontal);
                    shell.capture_event();
                    return;
                }

                if position.y < self.header_height && self.columns_resizable(&metrics, bounds.width)
                {
                    let content_x = position.x + scroll_x;
                    if let Some(border) =
                        geometry::divider_at(&metrics.widths, content_x, self.divider_grab)
                    {
                        let border_x: f32 = metrics.widths[..=border].iter().sum();
                        state.drag = Some(Drag::Column {
                            border,
                            snapshot: metrics.widths.clone(),
                            grab_dx: content_x - border_x,
                        });
                        shell.capture_event();
                        return;
                    }
                }

                let Some(index) = geometry::row_at(
                    position.y,
                    self.header_height,
                    self.row_height,
                    scroll_y,
                    self.rows.len(),
                ) else {
                    return;
                };

                let top_y = self.header_height + index as f32 * self.row_height - scroll_y;
                let content_point = Point::new(position.x + scroll_x, position.y);
                if let Some(zone) = self.chevron_zone(&metrics.widths, index, top_y)
                    && zone.contains(content_point)
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
        if let Some(drag) = &state.drag {
            return match drag {
                Drag::Column { .. } => mouse::Interaction::ResizingHorizontally,
                Drag::Scroll { .. } => mouse::Interaction::Pointer,
            };
        }

        let bounds = layout.bounds();
        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };

        let metrics = self.metrics(state);
        let (scroll_x, scroll_y) = self.scroll_offsets(state, &metrics);

        let (vertical, horizontal) = self.scrollbars(bounds.size(), &metrics, scroll_x, scroll_y);
        let over_thumb = vertical.is_some_and(|bar| bar.thumb.contains(position))
            || horizontal.is_some_and(|bar| bar.thumb.contains(position));
        if over_thumb {
            return mouse::Interaction::Pointer;
        }

        if position.y < self.header_height
            && self.columns_resizable(&metrics, bounds.width)
            && geometry::divider_at(&metrics.widths, position.x + scroll_x, self.divider_grab)
                .is_some()
        {
            return mouse::Interaction::ResizingHorizontally;
        }

        let over_row = geometry::row_at(
            position.y,
            self.header_height,
            self.row_height,
            scroll_y,
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
    /// Updates the hovered row and hovered thumb from a non-dragging cursor move.
    #[allow(clippy::too_many_arguments)]
    fn handle_hover(
        &self,
        state: &mut State,
        metrics: &Metrics,
        scroll_x: f32,
        scroll_y: f32,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) {
        let (vertical, horizontal) = self.scrollbars(bounds.size(), metrics, scroll_x, scroll_y);
        let thumb = cursor.position_in(bounds).and_then(|position| {
            if vertical.is_some_and(|bar| bar.thumb.contains(position)) {
                Some(Axis::Vertical)
            } else if horizontal.is_some_and(|bar| bar.thumb.contains(position)) {
                Some(Axis::Horizontal)
            } else {
                None
            }
        });
        if thumb != state.hovered_thumb {
            state.hovered_thumb = thumb;
            shell.request_redraw();
        }

        let next = cursor.position_in(bounds).and_then(|position| {
            geometry::row_at(
                position.y,
                self.header_height,
                self.row_height,
                scroll_y,
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

    /// Clears any cached layer whose inputs changed since the last draw.
    fn reconcile_caches(
        &self,
        state: &State,
        size: Size,
        metrics: &Metrics,
        scroll_x: f32,
        scroll_y: f32,
    ) {
        let mut keys = state.keys.borrow_mut();

        let rows_dirty = keys.revision != self.revision
            || keys.size != size
            || keys.scroll_y != scroll_y
            || keys.scroll_x != scroll_x
            || keys.widths != metrics.widths;
        let header_dirty =
            keys.size != size || keys.widths != metrics.widths || keys.scroll_x != scroll_x;
        let highlight_dirty =
            rows_dirty || keys.hover != state.hovered_row || keys.active != self.active_row;
        let overlay_dirty = keys.size != size
            || keys.scroll_x != scroll_x
            || keys.scroll_y != scroll_y
            || keys.content_width != metrics.content_width
            || keys.hovered_thumb != state.hovered_thumb;

        if rows_dirty {
            state.cache_rows.clear();
        }
        if header_dirty {
            state.cache_header.clear();
        }
        if highlight_dirty {
            state.cache_highlight.clear();
        }
        if overlay_dirty {
            state.cache_overlay.clear();
        }

        *keys = CacheKeys {
            revision: self.revision,
            scroll_x,
            scroll_y,
            size,
            widths: metrics.widths.clone(),
            content_width: metrics.content_width,
            hover: state.hovered_row,
            active: self.active_row,
            hovered_thumb: state.hovered_thumb,
        };
    }

    fn draw_header(&self, frame: &mut Frame, painter: &Painter, bounds: Rectangle, scroll_x: f32) {
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

        let region = Rectangle {
            x: 0.0,
            y: 0.0,
            width: bounds.width,
            height: self.header_height,
        };
        frame.with_clip(region, |frame| {
            frame.translate(Vector::new(-scroll_x, 0.0));

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
        });
    }

    fn draw_rows(
        &self,
        frame: &mut Frame,
        painter: &Painter,
        bounds: Rectangle,
        scroll_x: f32,
        scroll_y: f32,
    ) {
        let body = Rectangle {
            x: 0.0,
            y: self.header_height,
            width: bounds.width,
            height: self.body_height(bounds),
        };
        frame.with_clip(body, |frame| {
            frame.translate(Vector::new(-scroll_x, 0.0));

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
        scroll_x: f32,
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
            frame.translate(Vector::new(-scroll_x, 0.0));

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

    #[allow(clippy::too_many_arguments)]
    fn draw_overlay(
        &self,
        frame: &mut Frame,
        style: &Style,
        size: Size,
        metrics: &Metrics,
        scroll_x: f32,
        scroll_y: f32,
        state: &State,
    ) {
        let (vertical, horizontal) = self.scrollbars(size, metrics, scroll_x, scroll_y);
        for (axis, bar) in [(Axis::Vertical, vertical), (Axis::Horizontal, horizontal)] {
            let Some(bar) = bar else {
                continue;
            };
            let thumb = if state.hovered_thumb == Some(axis) {
                style.scrollbar_thumb_hover
            } else {
                style.scrollbar_thumb
            };
            scrollbar::draw(frame, &bar, style.scrollbar_track, thumb);
        }
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
