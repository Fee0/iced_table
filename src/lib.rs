//! A generic, canvas-rendered `DataTable` widget for [iced](https://iced.rs).
//!
//! The table owns layout, row virtualization, column resizing, and hover/active
//! highlighting once, so consumers only feed it column/row data and event
//! callbacks. It is *prepared* for a tree/collapse use case — it renders
//! indentation and an expand/collapse affordance and emits a toggle event — but
//! it never walks a tree itself: the consumer flattens its data and passes an
//! already-filtered, flat row list.

pub mod data_table;

pub use data_table::DataTable;
pub use data_table::cell::{Cell, FontKind, TextRole, Weight};
pub use data_table::column::{CellAlign, Column, ColumnWidth};
pub use data_table::row::{Row, Toggle};
pub use data_table::style;
