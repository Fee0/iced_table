//! A single, already-flattened table row.

use crate::data_table::cell::Cell;

/// One row of the table. Rows arrive as a flat list that the consumer has
/// already filtered for collapse — the widget never walks a tree.
///
/// The row owns its cells, so the consumer can build them transiently each
/// frame (e.g. inside `view`) without holding them in long-lived storage.
#[derive(Debug, Clone)]
pub struct Row<'a> {
    /// Indentation level for the tree column (the collapse hook).
    pub depth: u16,
    /// Whether this row shows an expand/collapse chevron, and in which state.
    pub toggle: Toggle,
    /// One [`Cell`] per column.
    pub cells: Vec<Cell<'a>>,
}

impl<'a> Row<'a> {
    /// A depth-0 row with no chevron.
    pub fn new(cells: Vec<Cell<'a>>) -> Self {
        Self {
            depth: 0,
            toggle: Toggle::None,
            cells,
        }
    }

    /// Sets the indentation depth.
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// Sets the chevron state.
    pub fn toggle(mut self, toggle: Toggle) -> Self {
        self.toggle = toggle;
        self
    }
}

/// The expand/collapse affordance state of a row's tree column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Toggle {
    /// No chevron is drawn (a leaf, or a non-tree row).
    #[default]
    None,
    /// A collapsed node — chevron points right.
    Collapsed,
    /// An expanded node — chevron points down.
    Expanded,
}
