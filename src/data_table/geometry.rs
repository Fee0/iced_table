//! Pure layout math for [`DataTable`](crate::DataTable).
//!
//! These helpers are deliberately free of any iced rendering types so the
//! virtualization window, column-width distribution, and divider hit-testing
//! can be unit-tested in isolation.

use std::ops::Range;

use crate::data_table::column::{Column, ColumnWidth};

/// The half-extent, in pixels, of a divider's grab zone on either side.
pub(crate) const DIVIDER_GRAB: f32 = 4.0;

/// Distributes the available width across `columns`.
///
/// `Fixed` columns take their width, `Fill` columns share what's left equally;
/// every column is clamped up to its `min_width`.
pub(crate) fn distribute_widths(columns: &[Column], available: f32) -> Vec<f32> {
    let mut fixed_total = 0.0;
    let mut fill_count = 0;

    for column in columns {
        match column.width {
            ColumnWidth::Fixed(width) => fixed_total += width.max(column.min_width),
            ColumnWidth::Fill => fill_count += 1,
        }
    }

    let leftover = (available - fixed_total).max(0.0);
    let per_fill = if fill_count > 0 {
        leftover / fill_count as f32
    } else {
        0.0
    };

    columns
        .iter()
        .map(|column| match column.width {
            ColumnWidth::Fixed(width) => width.max(column.min_width),
            ColumnWidth::Fill => per_fill.max(column.min_width),
        })
        .collect()
}

/// The left edge (cumulative offset) of column `index`.
pub(crate) fn column_left(widths: &[f32], index: usize) -> f32 {
    widths[..index.min(widths.len())].iter().sum()
}

/// The half-open range of row indices intersecting the visible body.
///
/// `body_height` is the viewport height minus the header. The range is clamped
/// to `row_count`, so an empty table or a zero-height body yields `0..0`.
pub(crate) fn visible_rows(
    scroll_y: f32,
    body_height: f32,
    row_height: f32,
    row_count: usize,
) -> Range<usize> {
    if row_count == 0 || row_height <= 0.0 || body_height <= 0.0 {
        return 0..0;
    }

    let first = (scroll_y / row_height).floor().max(0.0) as usize;
    let last = ((scroll_y + body_height) / row_height).ceil() as usize;

    first.min(row_count)..last.min(row_count)
}

/// The maximum scroll offset that still keeps content in view.
pub(crate) fn max_scroll(row_count: usize, row_height: f32, body_height: f32) -> f32 {
    let content = row_count as f32 * row_height;
    (content - body_height).max(0.0)
}

/// The index of the resizable column whose right divider sits within `grab` of
/// `x`, if any.
pub(crate) fn divider_at(widths: &[f32], columns: &[Column], x: f32, grab: f32) -> Option<usize> {
    let mut edge = 0.0;
    for (index, width) in widths.iter().enumerate() {
        edge += width;
        if columns[index].resizable && (x - edge).abs() <= grab {
            return Some(index);
        }
    }
    None
}

/// The row index at local `y`, accounting for the header strip and scroll.
///
/// Returns `None` above the body or past the last row.
pub(crate) fn row_at(
    y: f32,
    header_height: f32,
    row_height: f32,
    scroll_y: f32,
    row_count: usize,
) -> Option<usize> {
    if y < header_height || row_height <= 0.0 {
        return None;
    }

    let index = ((y - header_height + scroll_y) / row_height).floor() as usize;
    (index < row_count).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_table::column::CellAlign;

    fn fill(min_width: f32) -> Column {
        Column {
            header: String::new(),
            width: ColumnWidth::Fill,
            min_width,
            align: CellAlign::Start,
            resizable: false,
            tree_column: false,
        }
    }

    fn fixed(width: f32, resizable: bool) -> Column {
        Column {
            header: String::new(),
            width: ColumnWidth::Fixed(width),
            min_width: 0.0,
            align: CellAlign::Start,
            resizable,
            tree_column: false,
        }
    }

    #[test]
    fn fill_columns_split_leftover_after_fixed() {
        let columns = [fill(0.0), fixed(100.0, false), fill(0.0)];
        let widths = distribute_widths(&columns, 500.0);
        assert_eq!(widths, vec![200.0, 100.0, 200.0]);
    }

    #[test]
    fn fill_columns_respect_their_minimum_width() {
        let columns = [fill(180.0), fill(0.0)];
        // Leftover per fill is 50, but the first column floors at 180.
        let widths = distribute_widths(&columns, 100.0);
        assert_eq!(widths, vec![180.0, 50.0]);
    }

    #[test]
    fn fixed_columns_clamp_up_to_minimum_width() {
        let mut column = fixed(20.0, false);
        column.min_width = 60.0;
        let widths = distribute_widths(&[column], 500.0);
        assert_eq!(widths, vec![60.0]);
    }

    #[test]
    fn fixed_only_table_does_not_stretch() {
        let columns = [fixed(100.0, false), fixed(150.0, false)];
        let widths = distribute_widths(&columns, 800.0);
        assert_eq!(widths, vec![100.0, 150.0]);
    }

    #[test]
    fn column_left_accumulates_preceding_widths() {
        let widths = [100.0, 150.0, 200.0];
        assert_eq!(column_left(&widths, 0), 0.0);
        assert_eq!(column_left(&widths, 2), 250.0);
        assert_eq!(column_left(&widths, 3), 450.0);
    }

    #[test]
    fn visible_range_covers_partial_rows_at_both_edges() {
        // scroll past 2.5 rows, body shows 4 rows worth.
        let range = visible_rows(50.0, 80.0, 20.0, 100);
        assert_eq!(range, 2..7);
    }

    #[test]
    fn visible_range_clamps_to_row_count() {
        let range = visible_rows(0.0, 1000.0, 20.0, 3);
        assert_eq!(range, 0..3);
    }

    #[test]
    fn visible_range_is_empty_without_rows() {
        assert_eq!(visible_rows(0.0, 100.0, 20.0, 0), 0..0);
        assert_eq!(visible_rows(0.0, 0.0, 20.0, 10), 0..0);
    }

    #[test]
    fn max_scroll_is_zero_when_content_fits() {
        assert_eq!(max_scroll(3, 20.0, 200.0), 0.0);
        assert_eq!(max_scroll(20, 20.0, 200.0), 200.0);
    }

    #[test]
    fn divider_hit_requires_a_resizable_column_near_the_edge() {
        let widths = [100.0, 150.0, 200.0];
        let columns = [fixed(100.0, true), fixed(150.0, false), fixed(200.0, true)];

        // Right edge of column 0 is at x=100, and it is resizable.
        assert_eq!(divider_at(&widths, &columns, 102.0, DIVIDER_GRAB), Some(0));
        // Right edge of column 1 is at x=250 but it is not resizable.
        assert_eq!(divider_at(&widths, &columns, 250.0, DIVIDER_GRAB), None);
        // Too far from any edge.
        assert_eq!(divider_at(&widths, &columns, 130.0, DIVIDER_GRAB), None);
    }

    #[test]
    fn row_at_skips_the_header_and_floors_to_row() {
        // header 28, row 20, no scroll: y=40 -> row 0, y=70 -> row 2.
        assert_eq!(row_at(20.0, 28.0, 20.0, 0.0, 10), None);
        assert_eq!(row_at(40.0, 28.0, 20.0, 0.0, 10), Some(0));
        assert_eq!(row_at(70.0, 28.0, 20.0, 0.0, 10), Some(2));
    }

    #[test]
    fn row_at_returns_none_past_the_last_row() {
        assert_eq!(row_at(1000.0, 28.0, 20.0, 0.0, 3), None);
    }
}
