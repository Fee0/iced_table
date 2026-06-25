//! Column definitions for [`DataTable`](crate::DataTable).

/// Default preferred width of a freshly created column.
const DEFAULT_WIDTH: f32 = 120.0;
/// Default minimum width — a real floor so a column can never collapse away.
const DEFAULT_MIN_WIDTH: f32 = 40.0;

/// A single table column, supplied fresh by the consumer every frame.
///
/// `width` is only a *preferred* size: the widget owns the live column widths
/// (it persists them across the per-frame rebuild and adjusts them as the user
/// drags the header dividers), so changing `width` after the first layout has no
/// effect unless the column *count* changes.
#[derive(Debug, Clone)]
pub struct Column {
    /// Text drawn in the header cell.
    pub header: String,
    /// Preferred pixel width, used to seed the widget's live width.
    pub width: f32,
    /// Lower bound the column is never shrunk below (also clamps resizing).
    pub min_width: f32,
    /// Horizontal alignment of the cell contents.
    pub align: CellAlign,
    /// Whether this column hosts the indent guides and the expand/collapse glyph.
    ///
    /// Usually the first column. The widget draws the affordance here; folder
    /// semantics remain a consumer concern.
    pub tree_column: bool,
}

impl Column {
    /// Creates a column with sensible default widths.
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            width: DEFAULT_WIDTH,
            min_width: DEFAULT_MIN_WIDTH,
            align: CellAlign::Start,
            tree_column: false,
        }
    }

    /// Sets the preferred pixel width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
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

    /// Marks this column as the tree column (indent + chevron host).
    pub fn tree_column(mut self, tree_column: bool) -> Self {
        self.tree_column = tree_column;
        self
    }
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
