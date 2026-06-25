//! Column definitions for [`DataTable`](crate::DataTable).

/// A single table column, supplied fresh by the consumer every frame.
#[derive(Debug, Clone)]
pub struct Column {
    /// Text drawn in the header cell.
    pub header: String,
    /// How the column claims horizontal space.
    pub width: ColumnWidth,
    /// Lower bound the column is never shrunk below (also clamps resizing).
    pub min_width: f32,
    /// Horizontal alignment of the cell contents.
    pub align: CellAlign,
    /// Whether the divider on this column's right edge can be dragged to resize it.
    pub resizable: bool,
    /// Whether this column hosts the indent guides and the expand/collapse glyph.
    ///
    /// Usually the first column. The widget draws the affordance here; folder
    /// semantics remain a consumer concern.
    pub tree_column: bool,
}

impl Column {
    /// Creates a [`ColumnWidth::Fill`] column with sensible defaults.
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            width: ColumnWidth::Fill,
            min_width: 0.0,
            align: CellAlign::Start,
            resizable: false,
            tree_column: false,
        }
    }

    /// Sets a fixed pixel width.
    pub fn fixed(mut self, width: f32) -> Self {
        self.width = ColumnWidth::Fixed(width);
        self
    }

    /// Sets the minimum width.
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Sets the cell alignment.
    pub fn align(mut self, align: CellAlign) -> Self {
        self.align = align;
        self
    }

    /// Marks the column's right divider as draggable.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Marks this column as the tree column (indent + chevron host).
    pub fn tree_column(mut self, tree_column: bool) -> Self {
        self.tree_column = tree_column;
        self
    }
}

/// How a column claims horizontal space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    /// Shares leftover space equally with the other `Fill` columns.
    Fill,
    /// Occupies a fixed number of pixels.
    Fixed(f32),
}

/// Horizontal alignment of a cell's contents within its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellAlign {
    /// Left-aligned.
    #[default]
    Start,
    /// Centered.
    Center,
    /// Right-aligned.
    End,
}
