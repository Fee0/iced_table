//! A generic, canvas-rendered data table widget.
//!
//! See the [crate] documentation for the overall design. This module holds the
//! [`DataTable`] widget itself: its builder, persistent [`State`], and the
//! [`advanced::Widget`](iced::advanced::Widget) implementation that owns layout,
//! virtualization, column resizing, scrolling, and hover/active highlighting.

pub mod cell;
mod chevron;
pub mod column;
mod geometry;
pub mod row;
mod scrollbar;
pub mod sort;
pub mod style;

use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use iced::advanced::Clipboard;
use iced::advanced::Renderer as _;
use iced::advanced::Shell;
use iced::advanced::Widget;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text::Alignment as TextAlignment;
use iced::advanced::widget::{Tree, tree};
use iced::alignment::Vertical;
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{Cache, Frame, Text};
use iced::{Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Vector, font};

use crate::data_table::cell::{Cell, FontKind, TextRole, Weight};
use crate::data_table::column::{CellAlign, Column};
use crate::data_table::row::{Row, Toggle};
use crate::data_table::scrollbar::{Axis, Scrollbar};
use crate::data_table::sort::Sort;
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
    row_offset: usize,
    total_rows: usize,
    on_visible_rows: Option<Box<dyn Fn(Range<usize>) -> Message + 'a>>,
    row_height: f32,
    header_height: f32,
    text_size: f32,
    cell_padding_x: f32,
    indent_step: f32,
    chevron_box: f32,
    chevron_glyph: f32,
    scrollbar_thickness: f32,
    scrollbar_min_thumb: f32,
    scrollbar_thumb_thickness: Option<f32>,
    divider_grab: f32,
    divider_width: f32,
    indent_guide_width: f32,
    reserve_scrollbar_gutter: bool,
    font_ui: Font,
    font_editor: Font,
    active_row: Option<usize>,
    sort: Option<Sort>,
    target_scroll_row: Option<usize>,
    on_row_press: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_toggle_press: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_hover: Option<Box<dyn Fn(Option<usize>) -> Message + 'a>>,
    on_sort: Option<Box<dyn Fn(Sort) -> Message + 'a>>,
    chevron_svg_collapsed: Option<svg::Handle>,
    chevron_svg_expanded: Option<svg::Handle>,
    sort_chevron_svg_ascending: Option<svg::Handle>,
    sort_chevron_svg_descending: Option<svg::Handle>,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme> DataTable<'a, Message, Theme>
where
    Theme: Catalog,
{
    /// Creates a table from the given columns and (already-filtered, flat) rows.
    pub fn new(columns: Vec<Column>, rows: Vec<Row<'a>>) -> Self {
        let total_rows = rows.len();
        Self {
            columns,
            rows,
            row_offset: 0,
            total_rows,
            on_visible_rows: None,
            row_height: DEFAULT_ROW_HEIGHT,
            header_height: DEFAULT_HEADER_HEIGHT,
            text_size: DEFAULT_TEXT_SIZE,
            cell_padding_x: DEFAULT_CELL_PADDING_X,
            indent_step: DEFAULT_INDENT_STEP,
            chevron_box: DEFAULT_CHEVRON_BOX,
            chevron_glyph: DEFAULT_CHEVRON_GLYPH,
            scrollbar_thickness: DEFAULT_SCROLLBAR_THICKNESS,
            scrollbar_min_thumb: DEFAULT_SCROLLBAR_MIN_THUMB,
            scrollbar_thumb_thickness: None,
            divider_grab: DEFAULT_DIVIDER_GRAB,
            divider_width: DEFAULT_DIVIDER_WIDTH,
            indent_guide_width: DEFAULT_INDENT_GUIDE_WIDTH,
            reserve_scrollbar_gutter: false,
            font_ui: Font::DEFAULT,
            font_editor: Font::MONOSPACE,
            active_row: None,
            sort: None,
            target_scroll_row: None,
            on_row_press: None,
            on_toggle_press: None,
            on_hover: None,
            on_sort: None,
            chevron_svg_collapsed: None,
            chevron_svg_expanded: None,
            sort_chevron_svg_ascending: None,
            sort_chevron_svg_descending: None,
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

    /// Sets the drawn thumb thickness, centered in the track. Defaults to the
    /// full track thickness. The whole track width stays grabbable.
    pub fn scrollbar_thumb_thickness(mut self, scrollbar_thumb_thickness: f32) -> Self {
        self.scrollbar_thumb_thickness = Some(scrollbar_thumb_thickness);
        self
    }

    /// When set, columns are fitted into `viewport_width - scrollbar_thickness`
    /// whenever the vertical scrollbar is showing, so the scrollbar occupies its
    /// own gutter instead of overlapping the rightmost column's content.
    pub fn reserve_scrollbar_gutter(mut self, reserve: bool) -> Self {
        self.reserve_scrollbar_gutter = reserve;
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

    /// Sets the font used for [`FontKind::Ui`](crate::data_table::cell::FontKind::Ui) cells and headers.
    pub fn font_ui(mut self, font: Font) -> Self {
        self.font_ui = font;
        self
    }

    /// Sets the font used for [`FontKind::Editor`](crate::data_table::cell::FontKind::Editor) cells.
    pub fn font_editor(mut self, font: Font) -> Self {
        self.font_editor = font;
        self
    }

    /// Sets the consumer-resolved active (selected) row.
    pub fn active_row(mut self, active_row: Option<usize>) -> Self {
        self.active_row = active_row;
        self
    }

    /// Sets the column the consumer has sorted its rows by, if any.
    ///
    /// Only drives the header indicator — the consumer reorders its own rows.
    pub fn sort(mut self, sort: Option<Sort>) -> Self {
        self.sort = sort;
        self
    }

    /// Scrolls to make `row` visible the first time this target is applied.
    /// Subsequent frames with the same target are ignored so the user can
    /// scroll freely after the initial jump. Pass `None` to clear the target.
    pub fn scroll_to_row(mut self, row: Option<usize>) -> Self {
        self.target_scroll_row = row;
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

    /// Sets the callback fired when a [`sortable`](crate::Column::sortable)
    /// column's header is pressed.
    ///
    /// The payload is the sort the press implies: the first press on a column
    /// sorts it descending, a further press on the same column reverses it. The
    /// consumer reorders its rows and feeds the value back through
    /// [`sort`](Self::sort).
    pub fn on_sort(mut self, callback: impl Fn(Sort) -> Message + 'a) -> Self {
        self.on_sort = Some(Box::new(callback));
        self
    }

    /// Sets the callback fired when the hovered row changes.
    pub fn on_hover(mut self, callback: impl Fn(Option<usize>) -> Message + 'a) -> Self {
        self.on_hover = Some(Box::new(callback));
        self
    }

    /// Sets the index of the first row in `rows` within the full dataset.
    ///
    /// Use together with [`total_rows`](Self::total_rows) and
    /// [`on_visible_rows`](Self::on_visible_rows) to pass a windowed subset of rows while keeping
    /// the scrollbar correct.
    pub fn row_offset(mut self, offset: usize) -> Self {
        self.row_offset = offset;
        self
    }

    /// Sets the total number of rows in the full dataset.
    ///
    /// Defaults to `rows.len()`. When passing a windowed subset, set this to the
    /// full count so the scrollbar reflects the real content height.
    pub fn total_rows(mut self, count: usize) -> Self {
        self.total_rows = count;
        self
    }

    /// Sets a callback fired with the dataset rows on screen whenever they change: on scrolling,
    /// resizing, a clamp after the row count shrank, or a widget state reset. A windowed consumer
    /// builds its row window around this range.
    pub fn on_visible_rows(mut self, callback: impl Fn(Range<usize>) -> Message + 'a) -> Self {
        self.on_visible_rows = Some(Box::new(callback));
        self
    }

    /// Replaces the path-drawn chevron with SVG icons.
    ///
    /// `collapsed` is shown when the row can be expanded; `expanded` when it can be collapsed.
    pub fn chevron_svg(mut self, collapsed: svg::Handle, expanded: svg::Handle) -> Self {
        self.chevron_svg_collapsed = Some(collapsed);
        self.chevron_svg_expanded = Some(expanded);
        self
    }

    /// Replaces the path-drawn sort indicator with SVG icons.
    ///
    /// `ascending` is shown on a column sorted smallest-first; `descending` on
    /// one sorted largest-first.
    pub fn sort_chevron_svg(mut self, ascending: svg::Handle, descending: svg::Handle) -> Self {
        self.sort_chevron_svg_ascending = Some(ascending);
        self.sort_chevron_svg_descending = Some(descending);
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

    /// Whether the vertical scrollbar is showing. Depends only on row count
    /// and body height, never on column widths, so it can be resolved before
    /// [`metrics`](Self::metrics) fits the columns.
    fn vscroll_needed(&self, bounds: Rectangle) -> bool {
        let content_height = self.total_rows as f32 * self.row_height;
        scrollbar::visible(content_height, self.body_height(bounds))
    }

    /// The width available to fit columns into: the full bounds, minus the
    /// vertical scrollbar's gutter when [`reserve_scrollbar_gutter`]
    /// (Self::reserve_scrollbar_gutter) is set and the scrollbar is showing.
    fn content_viewport_width(&self, bounds: Rectangle) -> f32 {
        if self.reserve_scrollbar_gutter && self.vscroll_needed(bounds) {
            (bounds.width - self.scrollbar_thickness).max(0.0)
        } else {
            bounds.width
        }
    }

    /// The per-frame layout metrics shared by drawing and event handling.
    fn metrics(&self, state: &State, viewport_width: f32) -> Metrics {
        let mins: Vec<f32> = self.columns.iter().map(|column| column.min_width).collect();
        let basis = if state.basis.len() == self.columns.len() {
            state.basis.clone()
        } else {
            self.columns.iter().map(|column| column.width).collect()
        };
        Metrics {
            widths: geometry::fit_widths(&basis, &mins, viewport_width),
            content_width: geometry::content_width(&mins, viewport_width),
            content_height: self.total_rows as f32 * self.row_height,
            mins,
        }
    }

    /// The clamped scroll offsets for the given metrics.
    fn scroll_offsets(&self, state: &State, metrics: &Metrics, viewport: Size) -> (f32, f32) {
        let body_height = (viewport.height - self.header_height).max(0.0);
        let max_x = geometry::max_scroll_x(metrics.content_width, viewport.width);
        let max_y = geometry::max_scroll(self.total_rows, self.row_height, body_height);
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

    /// Every pixel knob the cached layers lay themselves out with.
    fn sizing(&self) -> Sizing {
        Sizing {
            row_height: self.row_height,
            header_height: self.header_height,
            text_size: self.text_size,
            cell_padding_x: self.cell_padding_x,
            indent_step: self.indent_step,
            chevron_box: self.chevron_box,
            chevron_glyph: self.chevron_glyph,
            scrollbar_thickness: self.scrollbar_thickness,
            scrollbar_min_thumb: self.scrollbar_min_thumb,
            scrollbar_thumb_thickness: self.scrollbar_thumb_thickness,
            divider_width: self.divider_width,
            indent_guide_width: self.indent_guide_width,
            reserve_scrollbar_gutter: self.reserve_scrollbar_gutter,
        }
    }

    /// Hashes the column properties the header and cells are drawn from.
    ///
    /// `width` and `min_width` are left out: they already reach the cache keys
    /// through the fitted [`Metrics::widths`] and [`Metrics::content_width`].
    fn columns_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for column in &self.columns {
            column.header.hash(&mut hasher);
            column.align.hash(&mut hasher);
            column.tree_column.hash(&mut hasher);
            column.sortable.hash(&mut hasher);
            column.font.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Hashes the sort state and the glyphs its indicator is drawn with, so the
    /// header layer repaints when the indicator moves, flips, or changes artwork.
    fn sort_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.sort.hash(&mut hasher);
        self.sort_chevron_svg_ascending.hash(&mut hasher);
        self.sort_chevron_svg_descending.hash(&mut hasher);
        hasher.finish()
    }

    /// Hashes the row content that is actually drawn, so a change to a visible
    /// cell invalidates the row layers without the consumer signalling it.
    ///
    /// Only the `drawn` window is hashed — a handful of rows, not the dataset —
    /// which keeps this cheap enough to run every frame. Rows outside the window
    /// are clipped away, so a change there cannot show up on screen.
    fn rows_hash(&self, drawn: Range<usize>) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.row_offset.hash(&mut hasher);
        self.chevron_svg_collapsed.hash(&mut hasher);
        self.chevron_svg_expanded.hash(&mut hasher);
        for global in drawn {
            self.rows[global - self.row_offset].hash(&mut hasher);
        }
        hasher.finish()
    }

    /// The half-open range of dataset row indices this frame paints: the
    /// virtualized visible range narrowed to the supplied row window.
    fn drawn_rows(&self, bounds: Rectangle, scroll_y: f32) -> Range<usize> {
        let visible = geometry::visible_rows(
            scroll_y,
            self.body_height(bounds),
            self.row_height,
            self.total_rows,
        );
        geometry::drawn_rows(visible, self.row_offset, self.rows.len())
    }

    /// The tree column index, if any column hosts the collapse affordance.
    fn tree_column(&self) -> Option<usize> {
        self.columns.iter().position(|column| column.tree_column)
    }

    /// The index of the sortable column at content-space `x`, if any.
    fn sortable_column_at(&self, widths: &[f32], x: f32) -> Option<usize> {
        geometry::column_at(widths, x).filter(|&column| self.columns[column].sortable)
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
    applied_target_row: Option<usize>,
    /// The visible rows last sent to [`DataTable::on_visible_rows`].
    reported_rows: Option<Range<usize>>,
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
            applied_target_row: None,
            reported_rows: None,
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

/// How much room the table has, and how much of that room is on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Extent {
    size: Size,
    visible: Rectangle,
}

/// Every pixel knob the cached layers lay themselves out with, bundled so they
/// can be compared as one cache key. Consumers rarely change these, so treating
/// a difference as invalidating all four layers costs nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sizing {
    row_height: f32,
    header_height: f32,
    text_size: f32,
    cell_padding_x: f32,
    indent_step: f32,
    chevron_box: f32,
    chevron_glyph: f32,
    scrollbar_thickness: f32,
    scrollbar_min_thumb: f32,
    scrollbar_thumb_thickness: Option<f32>,
    divider_width: f32,
    indent_guide_width: f32,
    reserve_scrollbar_gutter: bool,
}

/// The inputs each cached layer was last drawn against, for invalidation.
struct CacheKeys {
    /// Hash of the column properties the header and cells are drawn from.
    columns: u64,
    /// Hash of the row content that was actually painted.
    rows: u64,
    /// Hash of the sort state and the glyphs its indicator is drawn with.
    sort: u64,
    sizing: Sizing,
    total_rows: usize,
    row_offset: usize,
    scroll_x: f32,
    scroll_y: f32,
    size: Size,
    /// The slice of the table that is actually on screen, relative to its own bounds.
    ///
    /// Distinct from `size`: an ancestor that gives the table its full content height and scrolls
    /// it — rather than sizing it to the viewport and letting it scroll itself — leaves `size`
    /// fixed while this moves. The cached layers are drawn through it, and `iced_wgpu` only
    /// re-uploads cached text when the geometry's version changes, so a layer kept across a
    /// change here keeps text prepared for the old slice.
    visible: Rectangle,
    widths: Vec<f32>,
    content_width: f32,
    hover: Option<usize>,
    active: Option<usize>,
    hovered_thumb: Option<Axis>,
    /// The resolved style the layers were drawn with. A theme switch changes
    /// this, so every cached layer must be repainted when it differs.
    style: Option<Style>,
    font_ui: Font,
    font_editor: Font,
}

impl CacheKeys {
    /// Keys that never match a real frame, forcing the first draw to populate.
    fn stale() -> Self {
        Self {
            columns: 0,
            rows: 0,
            sort: 0,
            // NaN never compares equal, so this alone guarantees the first draw.
            sizing: Sizing {
                row_height: f32::NAN,
                header_height: f32::NAN,
                text_size: f32::NAN,
                cell_padding_x: f32::NAN,
                indent_step: f32::NAN,
                chevron_box: f32::NAN,
                chevron_glyph: f32::NAN,
                scrollbar_thickness: f32::NAN,
                scrollbar_min_thumb: f32::NAN,
                scrollbar_thumb_thickness: Some(f32::NAN),
                divider_width: f32::NAN,
                indent_guide_width: f32::NAN,
                reserve_scrollbar_gutter: false,
            },
            total_rows: 0,
            row_offset: 0,
            scroll_x: f32::NAN,
            scroll_y: f32::NAN,
            size: Size::ZERO,
            visible: Rectangle::new(Point::new(f32::NAN, f32::NAN), Size::ZERO),
            widths: Vec::new(),
            content_width: f32::NAN,
            hover: None,
            active: None,
            hovered_thumb: None,
            style: None,
            font_ui: Font::DEFAULT,
            font_editor: Font::MONOSPACE,
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
    font_ui: Font,
    font_editor: Font,
    chevron_svg_collapsed: Option<svg::Handle>,
    chevron_svg_expanded: Option<svg::Handle>,
    sort_chevron_svg_ascending: Option<svg::Handle>,
    sort_chevron_svg_descending: Option<svg::Handle>,
}

impl Painter<'_> {
    /// Draws a full row: its background fill, dividers-aware cells, chevron, and
    /// indent guides, all in the given [`Status`]. Cell content is clipped to
    /// `clip` (the layer region) intersected with each cell's own rectangle.
    #[allow(clippy::too_many_arguments)]
    fn row(
        &self,
        frame: &mut Frame,
        row: &Row,
        row_index: usize,
        top_y: f32,
        status: Status,
        scroll_x: f32,
        clip: Rectangle,
    ) {
        debug_assert!(
            row.cells.len() == self.columns.len(),
            "row {} has {} cells but table has {} columns",
            row_index,
            row.cells.len(),
            self.columns.len()
        );
        if let Some(background) = self.style.row_background(status, row_index) {
            // Wrap in a sub-frame so the background is flushed to a mesh immediately.
            // `paste` puts sub-frame meshes before the parent's own buffer, so anything
            // drawn via `with_clip` in the cells below will come *after* this mesh —
            // the correct draw order (background beneath cell content).
            frame.with_clip(clip, |frame| {
                frame.fill_rectangle(
                    Point::new(0.0, top_y),
                    Size::new(self.total_width(), self.row_height),
                    background,
                );
            });
        }

        let center_y = top_y + self.row_height / 2.0;
        for (index, column) in self.columns.iter().enumerate() {
            let left = geometry::column_left(self.widths, index);
            let width = self.widths[index];
            if column.tree_column {
                self.tree_cell(frame, row, index, center_y, status, scroll_x, clip);
            } else {
                self.cell(
                    frame,
                    &row.cells[index],
                    column.align,
                    left,
                    width,
                    center_y,
                    status,
                    column.font,
                    scroll_x,
                    clip,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn tree_cell(
        &self,
        frame: &mut Frame,
        row: &Row,
        index: usize,
        center_y: f32,
        status: Status,
        scroll_x: f32,
        clip: Rectangle,
    ) {
        let left = geometry::column_left(self.widths, index);
        let width = self.widths[index];
        let cell = &row.cells[index];
        let indent = f32::from(row.depth) * self.indent_step;
        let content_left = left + self.cell_padding_x + indent;

        let text_left = if row.toggle == Toggle::None {
            content_left
        } else {
            content_left + self.chevron_box
        };
        self.clipped_cell(
            frame,
            left,
            width,
            center_y,
            scroll_x,
            clip,
            |painter, frame| {
                painter.indent_guides(frame, left, row.depth, center_y);
                if row.toggle != Toggle::None {
                    let (direction, handle) = match row.toggle {
                        Toggle::Collapsed => {
                            (chevron::Direction::Right, &painter.chevron_svg_collapsed)
                        }
                        Toggle::Expanded => {
                            (chevron::Direction::Down, &painter.chevron_svg_expanded)
                        }
                        Toggle::None => unreachable!(),
                    };
                    chevron::draw(
                        frame,
                        painter.glyph_bounds(content_left, painter.chevron_box, center_y),
                        direction,
                        painter.style.text_color(TextRole::Primary, status),
                        handle.as_ref(),
                    );
                }
                painter.text(
                    frame,
                    cell,
                    text_left,
                    TextAlignment::Left,
                    center_y,
                    status,
                    painter.columns[index].font,
                );
            },
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
        font_override: Option<Font>,
        scroll_x: f32,
        clip: Rectangle,
    ) {
        let (x, alignment) = match align {
            CellAlign::Start => (left + self.cell_padding_x, TextAlignment::Left),
            CellAlign::Center => (left + width / 2.0, TextAlignment::Center),
            CellAlign::End => (left + width - self.cell_padding_x, TextAlignment::Right),
        };
        self.clipped_cell(
            frame,
            left,
            width,
            center_y,
            scroll_x,
            clip,
            |painter, frame| {
                painter.text(frame, cell, x, alignment, center_y, status, font_override);
            },
        );
    }

    /// Clips `f`'s drawing to a single cell rectangle, intersected with `clip`
    /// (the region the layer is already clipped to).
    ///
    /// `Frame::with_clip` builds a fresh sub-frame at the identity transform, so
    /// the cell rectangle is expressed in screen coordinates (with `scroll_x`
    /// applied) and the horizontal scroll translation is re-applied inside the
    /// closure to keep the callers' logical coordinates correct.
    #[allow(clippy::too_many_arguments)]
    fn clipped_cell(
        &self,
        frame: &mut Frame,
        left: f32,
        width: f32,
        center_y: f32,
        scroll_x: f32,
        clip: Rectangle,
        f: impl FnOnce(&Self, &mut Frame),
    ) {
        let cell = Rectangle {
            x: left - scroll_x,
            y: center_y - self.row_height / 2.0,
            width,
            height: self.row_height,
        };
        let Some(region) = cell.intersection(&clip) else {
            return;
        };
        frame.with_clip(region, |frame| {
            frame.translate(Vector::new(-scroll_x, 0.0));
            f(self, frame);
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &self,
        frame: &mut Frame,
        cell: &Cell,
        x: f32,
        alignment: TextAlignment,
        center_y: f32,
        status: Status,
        font_override: Option<Font>,
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
        let font = match font_override {
            Some(mut f) => {
                if *weight == Weight::Bold {
                    f.weight = font::Weight::Bold;
                }
                f
            }
            None => self.font_for(*font_kind, *weight),
        };
        frame.fill_text(Text {
            content: text.to_string(),
            position: Point::new(x, center_y),
            color: self.style.text_color(*role, status),
            size: Pixels(self.text_size),
            font,
            align_x: alignment,
            align_y: Vertical::Center,
            max_width: f32::INFINITY,
            ..Text::default()
        });
    }

    /// Draws the sort indicator into the box every sortable header cell
    /// reserves at its trailing edge.
    #[allow(clippy::too_many_arguments)]
    fn sort_indicator(
        &self,
        frame: &mut Frame,
        left: f32,
        width: f32,
        center_y: f32,
        direction: sort::Direction,
        scroll_x: f32,
        clip: Rectangle,
    ) {
        let box_left = left + width - self.chevron_box;
        let (glyph, handle) = match direction {
            sort::Direction::Ascending => {
                (chevron::Direction::Up, &self.sort_chevron_svg_ascending)
            }
            sort::Direction::Descending => {
                (chevron::Direction::Down, &self.sort_chevron_svg_descending)
            }
        };
        self.clipped_cell(
            frame,
            box_left,
            self.chevron_box,
            center_y,
            scroll_x,
            clip,
            |painter, frame| {
                chevron::draw(
                    frame,
                    painter.glyph_bounds(box_left, painter.chevron_box, center_y),
                    glyph,
                    painter.style.text_color(TextRole::Primary, Status::Regular),
                    handle.as_ref(),
                );
            },
        );
    }

    /// The square the glyph occupies, centered within a `box_width`-wide slot
    /// starting at `box_left`.
    fn glyph_bounds(&self, box_left: f32, box_width: f32, center_y: f32) -> Rectangle {
        Rectangle {
            x: box_left + (box_width - self.chevron_glyph) / 2.0,
            y: center_y - self.chevron_glyph / 2.0,
            width: self.chevron_glyph,
            height: self.chevron_glyph,
        }
    }

    fn indent_guides(&self, frame: &mut Frame, cell_left: f32, depth: u16, center_y: f32) {
        for ancestor in 0..depth {
            let x = cell_left
                + self.cell_padding_x
                + f32::from(ancestor) * self.indent_step
                + self.chevron_box / 2.0;
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

    fn font_for(&self, kind: FontKind, weight: Weight) -> Font {
        let mut resolved = match kind {
            FontKind::Ui => self.font_ui,
            FontKind::Editor => self.font_editor,
        };
        if weight == Weight::Bold {
            resolved.weight = font::Weight::Bold;
        }
        resolved
    }
}

/// Whether the on-screen slice grew beyond the one the layers were drawn against.
///
/// `iced_wgpu` bakes each glyph's scissor at prepare time as
/// `layer_bounds ∩ text_clip` and only re-prepares when the geometry version or
/// the layer transformation changes — never when the layer clip alone moves. A
/// *shrinking* slice is still enforced by the render pass scissor, which is taken
/// from the current layer bounds, so only a growing slice can leave cached text
/// clipped to an area that is now too tight.
///
/// A `previous` holding NaN (see [`CacheKeys::stale`]) never contains anything,
/// which is what forces the first draw.
fn clip_grew(previous: Rectangle, current: Rectangle) -> bool {
    let contained = current.x >= previous.x
        && current.y >= previous.y
        && current.x + current.width <= previous.x + previous.width
        && current.y + current.height <= previous.y + previous.height;
    !contained
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
        tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        match self.target_scroll_row {
            Some(row) if state.applied_target_row != Some(row) => {
                state.scroll_y = row as f32 * self.row_height;
                state.applied_target_row = Some(row);
            }
            Some(_) => {}
            // Forgotten once the consumer drops the target, so a later jump to the same row
            // scrolls again.
            None => state.applied_target_row = None,
        }
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
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }

        let state = tree.state.downcast_ref::<State>();
        let resolved = <Theme as Catalog>::style(theme, &self.class, Status::Regular);

        let viewport_width = self.content_viewport_width(bounds);
        let metrics = self.metrics(state, viewport_width);
        let (scroll_x, scroll_y) =
            self.scroll_offsets(state, &metrics, Size::new(viewport_width, bounds.height));

        // Relative to the table's own origin, so scrolling the *table* past a fixed window and
        // moving that window over a fixed table are told apart by `scroll_y` and this in turn.
        let visible = bounds
            .intersection(viewport)
            .map(|slice| Rectangle {
                x: slice.x - bounds.x,
                y: slice.y - bounds.y,
                ..slice
            })
            .unwrap_or(Rectangle::new(Point::ORIGIN, Size::ZERO));

        self.reconcile_caches(
            state,
            Extent {
                size: bounds.size(),
                visible,
            },
            &metrics,
            scroll_x,
            scroll_y,
            self.rows_hash(self.drawn_rows(bounds, scroll_y)),
            &resolved,
        );

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
            font_ui: self.font_ui,
            font_editor: self.font_editor,
            chevron_svg_collapsed: self.chevron_svg_collapsed.clone(),
            chevron_svg_expanded: self.chevron_svg_expanded.clone(),
            sort_chevron_svg_ascending: self.sort_chevron_svg_ascending.clone(),
            sort_chevron_svg_descending: self.sort_chevron_svg_descending.clone(),
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
        self.handle_event(state, event, bounds, cursor, shell);
        self.report_visible_rows(state, bounds, shell);
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

        let viewport_width = self.content_viewport_width(bounds);
        let metrics = self.metrics(state, viewport_width);
        let (scroll_x, scroll_y) =
            self.scroll_offsets(state, &metrics, Size::new(viewport_width, bounds.height));

        let (vertical, horizontal) = self.scrollbars(bounds.size(), &metrics, scroll_x, scroll_y);
        let over_thumb = vertical.is_some_and(|bar| bar.thumb.contains(position))
            || horizontal.is_some_and(|bar| bar.thumb.contains(position));
        if over_thumb {
            return mouse::Interaction::Pointer;
        }

        if position.y < self.header_height {
            let content_x = position.x + scroll_x;
            if self.columns_resizable(&metrics, viewport_width)
                && geometry::divider_at(&metrics.widths, content_x, self.divider_grab).is_some()
            {
                return mouse::Interaction::ResizingHorizontally;
            }
            if self.on_sort.is_some()
                && self
                    .sortable_column_at(&metrics.widths, content_x)
                    .is_some()
            {
                return mouse::Interaction::Pointer;
            }
            return mouse::Interaction::Idle;
        }

        let over_row = geometry::row_at(
            position.y,
            self.header_height,
            self.row_height,
            scroll_y,
            self.total_rows,
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
    fn handle_event(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) {
        state.ensure_basis(&self.columns);

        // The events that need no layout math at all.
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.shift_held = modifiers.shift();
                return;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag.is_some() =>
            {
                state.drag = None;
                shell.capture_event();
                return;
            }
            _ => {}
        }

        // Everything below fits the columns, which allocates. Drop the events that
        // cannot move this table first, so ordinary pointer traffic elsewhere in
        // the window costs nothing.
        let concerns_table = match event {
            Event::Mouse(mouse::Event::WheelScrolled { .. }) => {
                cursor.position_over(bounds).is_some()
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                cursor.position_in(bounds).is_some()
            }
            // A cursor outside the table still matters while dragging, or when it
            // left a highlight behind that has to be cleared.
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                state.drag.is_some()
                    || cursor.position_in(bounds).is_some()
                    || state.hovered_row.is_some()
                    || state.hovered_thumb.is_some()
            }
            _ => false,
        };
        if !concerns_table {
            return;
        }

        let viewport_width = self.content_viewport_width(bounds);
        let metrics = self.metrics(state, viewport_width);
        let (scroll_x, scroll_y) =
            self.scroll_offsets(state, &metrics, Size::new(viewport_width, bounds.height));

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let (mut dx, mut dy) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => {
                        (x * self.row_height, y * self.row_height)
                    }
                    mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                };
                if state.shift_held && dx == 0.0 {
                    dx = dy;
                    dy = 0.0;
                }

                let body_height = self.body_height(bounds);
                let max_x = geometry::max_scroll_x(metrics.content_width, viewport_width);
                let max_y = geometry::max_scroll(self.total_rows, self.row_height, body_height);
                let next_x = (scroll_x - dx).clamp(0.0, max_x);
                let next_y = (scroll_y - dy).clamp(0.0, max_y);
                if next_x != state.scroll_x || next_y != state.scroll_y {
                    state.scroll_x = next_x;
                    state.scroll_y = next_y;
                    shell.capture_event();
                    shell.request_redraw();
                    self.handle_hover(state, &metrics, next_x, next_y, bounds, cursor, shell);
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
                    shell.capture_event();
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
                        shell.capture_event();
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
                if let Some(bar) = vertical
                    && bar.track.contains(position)
                {
                    let thumb_half = bar.thumb.height / 2.0;
                    let lead = position.y - thumb_half;
                    let new_y = bar.offset_for_thumb(Axis::Vertical, metrics.content_height, lead);
                    state.scroll_y = new_y;
                    state.drag = Some(Drag::Scroll {
                        axis: Axis::Vertical,
                        grab: thumb_half,
                    });
                    state.hovered_thumb = Some(Axis::Vertical);
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
                if let Some(bar) = horizontal
                    && bar.track.contains(position)
                {
                    let thumb_half = bar.thumb.width / 2.0;
                    let lead = position.x - thumb_half;
                    state.scroll_x =
                        bar.offset_for_thumb(Axis::Horizontal, metrics.content_width, lead);
                    state.drag = Some(Drag::Scroll {
                        axis: Axis::Horizontal,
                        grab: thumb_half,
                    });
                    state.hovered_thumb = Some(Axis::Horizontal);
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }

                if position.y < self.header_height {
                    let content_x = position.x + scroll_x;
                    if self.columns_resizable(&metrics, viewport_width)
                        && let Some(border) =
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

                    if let Some(callback) = &self.on_sort
                        && let Some(column) = self.sortable_column_at(&metrics.widths, content_x)
                    {
                        shell.publish(callback(Sort::toggled(self.sort, column)));
                        shell.capture_event();
                    }
                    return;
                }

                let Some(global) = geometry::row_at(
                    position.y,
                    self.header_height,
                    self.row_height,
                    scroll_y,
                    self.total_rows,
                ) else {
                    return;
                };

                let top_y = self.header_height + global as f32 * self.row_height - scroll_y;
                let content_point = Point::new(position.x + scroll_x, position.y);
                // chevron_zone indexes into self.rows, so convert to local index
                if let Some(local) = global
                    .checked_sub(self.row_offset)
                    .filter(|&li| li < self.rows.len())
                    && let Some(zone) = self.chevron_zone(&metrics.widths, local, top_y)
                    && zone.contains(content_point)
                {
                    if let Some(callback) = &self.on_toggle_press {
                        shell.publish(callback(global));
                        shell.capture_event();
                    }
                    return;
                }

                if let Some(callback) = &self.on_row_press {
                    shell.publish(callback(global));
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    /// Tells [`on_visible_rows`](Self::on_visible_rows) which rows are on screen whenever that
    /// changes. Checked after every event, redraws included, so it also catches what no
    /// interaction caused: a resize, a scroll clamped after the rows shrank, a scroll-to-row
    /// jump, or state reset by a rebuilt widget tree.
    fn report_visible_rows(
        &self,
        state: &mut State,
        bounds: Rectangle,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(callback) = &self.on_visible_rows else {
            return;
        };
        let body_height = self.body_height(bounds);
        let max_y = geometry::max_scroll(self.total_rows, self.row_height, body_height);
        let visible = geometry::visible_rows(
            state.scroll_y.clamp(0.0, max_y),
            body_height,
            self.row_height,
            self.total_rows,
        );
        if state.reported_rows.as_ref() != Some(&visible) {
            state.reported_rows = Some(visible.clone());
            shell.publish(callback(visible));
        }
    }

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

        if cursor.position_in(bounds).is_some_and(|p| {
            vertical.is_some_and(|bar| bar.track.contains(p))
                || horizontal.is_some_and(|bar| bar.track.contains(p))
        }) {
            if state.hovered_row.is_some() {
                state.hovered_row = None;
                if let Some(callback) = &self.on_hover {
                    shell.publish(callback(None));
                }
                shell.request_redraw();
            }
            return;
        }

        let next = cursor.position_in(bounds).and_then(|position| {
            geometry::row_at(
                position.y,
                self.header_height,
                self.row_height,
                scroll_y,
                self.total_rows,
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
    #[allow(clippy::too_many_arguments)]
    fn reconcile_caches(
        &self,
        state: &State,
        extent: Extent,
        metrics: &Metrics,
        scroll_x: f32,
        scroll_y: f32,
        rows_hash: u64,
        style: &Style,
    ) {
        let Extent { size, visible } = extent;
        let sizing = self.sizing();
        let columns_hash = self.columns_hash();
        let sort_hash = self.sort_hash();
        let mut keys = state.keys.borrow_mut();

        // A style change (e.g. a theme switch) recolors every layer.
        let style_dirty = keys.style != Some(*style);
        // Only the layers that carry text or images care about the visible slice.
        let clip_dirty = clip_grew(keys.visible, visible);

        let rows_dirty = style_dirty
            || clip_dirty
            || keys.rows != rows_hash
            || keys.columns != columns_hash
            || keys.sizing != sizing
            || keys.total_rows != self.total_rows
            || keys.row_offset != self.row_offset
            || keys.size != size
            || keys.scroll_y != scroll_y
            || keys.scroll_x != scroll_x
            || keys.widths != metrics.widths
            || keys.font_ui != self.font_ui
            || keys.font_editor != self.font_editor;
        let header_dirty = style_dirty
            || clip_dirty
            || keys.columns != columns_hash
            || keys.sort != sort_hash
            || keys.sizing != sizing
            || keys.size != size
            || keys.widths != metrics.widths
            || keys.scroll_x != scroll_x
            || keys.font_ui != self.font_ui;
        let highlight_dirty =
            rows_dirty || keys.hover != state.hovered_row || keys.active != self.active_row;
        // The overlay is pure meshes, so it has no prepared clip that can go stale
        // and does not need `clip_grew`.
        let overlay_dirty = style_dirty
            || keys.sizing != sizing
            || keys.total_rows != self.total_rows
            || keys.size != size
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
            columns: columns_hash,
            rows: rows_hash,
            sort: sort_hash,
            sizing,
            total_rows: self.total_rows,
            row_offset: self.row_offset,
            scroll_x,
            scroll_y,
            size,
            visible,
            widths: metrics.widths.clone(),
            content_width: metrics.content_width,
            hover: state.hovered_row,
            active: self.active_row,
            hovered_thumb: state.hovered_thumb,
            style: Some(*style),
            font_ui: self.font_ui,
            font_editor: self.font_editor,
        };
    }

    fn draw_header(&self, frame: &mut Frame, painter: &Painter, bounds: Rectangle, scroll_x: f32) {
        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: bounds.width,
                height: self.header_height,
            },
            |frame| {
                frame.fill_rectangle(
                    Point::ORIGIN,
                    Size::new(bounds.width, self.header_height),
                    painter.style.header_background,
                );
            },
        );

        let header_style = Style {
            text_primary: painter.style.header_text,
            ..*painter.style
        };
        let header_painter = Painter {
            style: &header_style,
            chevron_svg_collapsed: painter.chevron_svg_collapsed.clone(),
            chevron_svg_expanded: painter.chevron_svg_expanded.clone(),
            sort_chevron_svg_ascending: painter.sort_chevron_svg_ascending.clone(),
            sort_chevron_svg_descending: painter.sort_chevron_svg_descending.clone(),
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
                // A sortable column always reserves the indicator box, so its
                // header text does not shift when the column becomes sorted.
                let text_width = if column.sortable {
                    (width - self.chevron_box).max(0.0)
                } else {
                    width
                };
                let header_cell = Cell::text(column.header.clone());
                header_painter.cell(
                    frame,
                    &header_cell,
                    column.align,
                    left,
                    text_width,
                    center_y,
                    Status::Regular,
                    column.font,
                    scroll_x,
                    region,
                );

                if let Some(sort) = self
                    .sort
                    .filter(|sort| sort.column == index && column.sortable)
                {
                    header_painter.sort_indicator(
                        frame,
                        left,
                        width,
                        center_y,
                        sort.direction,
                        scroll_x,
                        region,
                    );
                }
            }

            painter.dividers(frame, 0.0, self.header_height);
        });

        if let Some(color) = painter.style.header_divider {
            let y = self.header_height - painter.divider_width / 2.0;
            frame.fill_rectangle(
                Point::new(0.0, y),
                Size::new(bounds.width, painter.divider_width),
                color,
            );
        }
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

            for global in self.drawn_rows(bounds, scroll_y) {
                let local = global - self.row_offset;
                let top_y = self.header_height + global as f32 * self.row_height - scroll_y;
                painter.row(
                    frame,
                    &self.rows[local],
                    global,
                    top_y,
                    Status::Regular,
                    scroll_x,
                    body,
                );
                if let Some(color) = painter.style.row_divider {
                    let y = top_y + self.row_height;
                    frame.fill_rectangle(
                        Point::new(0.0, y - painter.divider_width / 2.0),
                        Size::new(painter.total_width(), painter.divider_width),
                        color,
                    );
                }
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
        let window_end = self.row_offset + self.rows.len();
        let in_window = |i: usize| i >= self.row_offset && i < window_end;
        let active = self
            .active_row
            .filter(|&i| i < self.total_rows && in_window(i));
        let hovered = hovered_row.filter(|&i| i < self.total_rows && in_window(i));

        frame.with_clip(body, |frame| {
            frame.translate(Vector::new(-scroll_x, 0.0));

            // Active first so a row that is both hovered and active shows hover on top.
            if let Some(index) = active.filter(|index| Some(*index) != hovered) {
                let local = index - self.row_offset;
                let top_y = self.header_height + index as f32 * self.row_height - scroll_y;
                painter.row(
                    frame,
                    &self.rows[local],
                    index,
                    top_y,
                    Status::Active,
                    scroll_x,
                    body,
                );
            }
            if let Some(index) = hovered {
                let local = index - self.row_offset;
                let top_y = self.header_height + index as f32 * self.row_height - scroll_y;
                painter.row(
                    frame,
                    &self.rows[local],
                    index,
                    top_y,
                    Status::Hovered,
                    scroll_x,
                    body,
                );
            }
            if active.is_some() || hovered.is_some() {
                painter.dividers(frame, self.header_height, self.body_height(bounds));
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
            scrollbar::draw(
                frame,
                axis,
                &bar,
                self.scrollbar_thumb_thickness,
                style.scrollbar_track,
                thumb,
            );
        }
        if let Some(color) = style.border {
            let w = self.divider_width;
            frame.fill_rectangle(Point::new(0.0, 0.0), Size::new(size.width, w), color);
            frame.fill_rectangle(
                Point::new(0.0, size.height - w),
                Size::new(size.width, w),
                color,
            );
            frame.fill_rectangle(Point::new(0.0, 0.0), Size::new(w, size.height), color);
            frame.fill_rectangle(
                Point::new(size.width - w, 0.0),
                Size::new(w, size.height),
                color,
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn clip_is_unchanged_when_the_slice_is_identical() {
        let slice = rect(0.0, 0.0, 200.0, 400.0);
        assert!(!clip_grew(slice, slice));
    }

    #[test]
    fn clip_shrinking_does_not_count_as_growth() {
        // The render pass scissor already enforces a tighter slice.
        let previous = rect(0.0, 0.0, 200.0, 400.0);
        assert!(!clip_grew(previous, rect(10.0, 20.0, 100.0, 200.0)));
    }

    #[test]
    fn clip_growing_on_any_edge_counts_as_growth() {
        let previous = rect(10.0, 20.0, 100.0, 200.0);
        assert!(clip_grew(previous, rect(0.0, 20.0, 110.0, 200.0)));
        assert!(clip_grew(previous, rect(10.0, 0.0, 100.0, 220.0)));
        assert!(clip_grew(previous, rect(10.0, 20.0, 101.0, 200.0)));
        assert!(clip_grew(previous, rect(10.0, 20.0, 100.0, 201.0)));
    }

    #[test]
    fn clip_sliding_sideways_counts_as_growth() {
        // An ancestor scrolling the table past a fixed window reveals rows the
        // cached layers were never prepared for.
        let previous = rect(0.0, 0.0, 200.0, 400.0);
        assert!(clip_grew(previous, rect(0.0, 100.0, 200.0, 400.0)));
    }

    #[test]
    fn stale_keys_always_report_growth() {
        assert!(clip_grew(
            CacheKeys::stale().visible,
            rect(0.0, 0.0, 200.0, 400.0)
        ));
    }
}
