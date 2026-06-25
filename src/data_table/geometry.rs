//! Pure layout math for [`DataTable`](crate::DataTable).
//!
//! These helpers are deliberately free of any iced rendering types so the
//! virtualization window, column-width distribution, transitive resizing, and
//! divider hit-testing can be unit-tested in isolation.

use std::ops::Range;

/// The half-extent, in pixels, of a divider's grab zone on either side.
pub(crate) const DIVIDER_GRAB: f32 = 4.0;

/// The horizontal content extent: the viewport, unless the columns' minimum
/// widths don't fit, in which case content overflows to that minimum total.
pub(crate) fn content_width(mins: &[f32], viewport: f32) -> f32 {
    let sum_min: f32 = mins.iter().sum();
    viewport.max(sum_min)
}

/// Water-fills `basis` to exactly fill the available width, honoring per-column
/// minimums.
///
/// When `viewport <= Σ mins` every column collapses to its minimum and the
/// result overflows the viewport (the caller shows a horizontal scrollbar).
/// Otherwise each column gets a share of `viewport` proportional to its basis,
/// clamped up to its minimum, with the clamp deficit redistributed across the
/// still-free columns. The result always sums to [`content_width`].
pub(crate) fn fit_widths(basis: &[f32], mins: &[f32], viewport: f32) -> Vec<f32> {
    let n = basis.len();
    if n == 0 {
        return Vec::new();
    }

    let sum_min: f32 = mins.iter().sum();
    if viewport <= sum_min {
        return mins.to_vec();
    }

    // Pin columns to their minimum one round at a time until a proportional
    // distribution of the remaining space clears every floor. Because
    // `viewport > Σ mins`, at least one column always stays free, so the loop
    // terminates by returning.
    let mut pinned = vec![false; n];
    loop {
        let fixed: f32 = (0..n).filter(|&i| pinned[i]).map(|i| mins[i]).sum();
        let free: Vec<usize> = (0..n).filter(|&i| !pinned[i]).collect();
        let basis_free: f32 = free.iter().map(|&i| basis[i]).sum();
        let remaining = viewport - fixed;

        let share = |i: usize| -> f32 {
            if basis_free > 0.0 {
                remaining * basis[i] / basis_free
            } else {
                remaining / free.len() as f32
            }
        };

        let mut clamped_any = false;
        for &i in &free {
            if share(i) < mins[i] {
                pinned[i] = true;
                clamped_any = true;
            }
        }

        if !clamped_any {
            let mut result = vec![0.0; n];
            for i in 0..n {
                result[i] = if pinned[i] { mins[i] } else { share(i) };
            }
            // Absorb sub-pixel rounding so the result sums to exactly `viewport`.
            let residual = viewport - result.iter().sum::<f32>();
            if let Some(&i) = free.last() {
                result[i] += residual;
            }
            return result;
        }
    }
}

/// The left edge (cumulative offset) of column `index`.
pub(crate) fn column_left(widths: &[f32], index: usize) -> f32 {
    widths[..index.min(widths.len())].iter().sum()
}

/// Rebuilds the right group of a divider drag so it occupies `total` pixels,
/// preferring each column's snapshot width, never below its minimum.
///
/// Shrinking shaves from the left (the immediate neighbor first); growing hands
/// the surplus to the immediate neighbor.
pub(crate) fn cascade_right(snapshot: &[f32], mins: &[f32], total: f32) -> Vec<f32> {
    if snapshot.is_empty() {
        return Vec::new();
    }

    let mut result = snapshot.to_vec();
    let current: f32 = snapshot.iter().sum();
    let delta = total - current;

    if delta < 0.0 {
        let mut remaining = -delta;
        for j in 0..result.len() {
            let take = remaining.min(snapshot[j] - mins[j]).max(0.0);
            result[j] = snapshot[j] - take;
            remaining -= take;
            if remaining <= 0.0 {
                break;
            }
        }
    } else if delta > 0.0 {
        result[0] += delta;
    }

    result
}

/// Rebuilds the left group of a divider drag so it occupies `total` pixels,
/// preferring each column's snapshot width, never below its minimum.
///
/// Shrinking shaves from the right (the dragged column gives first); growing
/// hands the surplus to the rightmost column (the dragged column).
pub(crate) fn cascade_left(snapshot: &[f32], mins: &[f32], total: f32) -> Vec<f32> {
    if snapshot.is_empty() {
        return Vec::new();
    }

    let mut result = snapshot.to_vec();
    let current: f32 = snapshot.iter().sum();
    let delta = total - current;

    if delta < 0.0 {
        let mut remaining = -delta;
        for j in (0..result.len()).rev() {
            let take = remaining.min(snapshot[j] - mins[j]).max(0.0);
            result[j] = snapshot[j] - take;
            remaining -= take;
            if remaining <= 0.0 {
                break;
            }
        }
    } else if delta > 0.0 {
        let last = result.len() - 1;
        result[last] += delta;
    }

    result
}

/// New display widths for dragging internal `border` (the right edge of column
/// `border`) so that edge lands at `desired_border_x` in content space.
///
/// Total width is conserved. The left group `0..=border` absorbs changes via
/// [`cascade_left`] (shrinks right-to-left) and the right group `border+1..n`
/// absorbs changes via [`cascade_right`] (shrinks left-to-right).
pub(crate) fn resize_columns(
    snapshot: &[f32],
    mins: &[f32],
    border: usize,
    desired_border_x: f32,
) -> Vec<f32> {
    let n = snapshot.len();
    if border + 1 >= n {
        return snapshot.to_vec();
    }

    let total: f32 = snapshot.iter().sum();
    let border_snap_x: f32 = snapshot[..=border].iter().sum();
    let capacity_left: f32 = (0..=border).map(|j| snapshot[j] - mins[j]).sum();
    let capacity_right: f32 = ((border + 1)..n).map(|j| snapshot[j] - mins[j]).sum();

    let clamped = desired_border_x.clamp(
        border_snap_x - capacity_left,
        border_snap_x + capacity_right,
    );

    let left = cascade_left(&snapshot[..=border], &mins[..=border], clamped);
    let right = cascade_right(
        &snapshot[border + 1..],
        &mins[border + 1..],
        total - clamped,
    );

    let mut result = snapshot.to_vec();
    result[..=border].copy_from_slice(&left);
    result[border + 1..].copy_from_slice(&right);
    result
}

/// The index of the internal divider (right edge of a column, `0..=n-2`) within
/// `grab` of `x`, if any. The outer left and right edges are never returned.
pub(crate) fn divider_at(widths: &[f32], x: f32, grab: f32) -> Option<usize> {
    if widths.is_empty() {
        return None;
    }
    let mut edge = 0.0;
    for (index, width) in widths.iter().enumerate().take(widths.len() - 1) {
        edge += width;
        if (x - edge).abs() <= grab {
            return Some(index);
        }
    }
    None
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

/// The maximum vertical scroll offset that still keeps content in view.
pub(crate) fn max_scroll(row_count: usize, row_height: f32, body_height: f32) -> f32 {
    let content = row_count as f32 * row_height;
    (content - body_height).max(0.0)
}

/// The maximum horizontal scroll offset that still keeps content in view.
pub(crate) fn max_scroll_x(content_width: f32, viewport: f32) -> f32 {
    (content_width - viewport).max(0.0)
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

    fn approx_eq(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    #[test]
    fn fit_widths_fills_viewport_proportionally() {
        let widths = fit_widths(&[100.0, 100.0, 200.0], &[0.0, 0.0, 0.0], 800.0);
        assert!(approx_eq(&widths, &[200.0, 200.0, 400.0]));
    }

    #[test]
    fn fit_widths_is_identity_on_already_fitted_input() {
        let basis = vec![220.0, 180.0, 100.0];
        let mins = vec![40.0, 40.0, 40.0];
        let viewport: f32 = basis.iter().sum();
        let widths = fit_widths(&basis, &mins, viewport);
        assert!(approx_eq(&widths, &basis));
    }

    #[test]
    fn fit_widths_redistributes_clamp_deficit_across_free_columns() {
        // Column 0 wants 10 but floors at 180; the other two split the rest.
        let widths = fit_widths(&[10.0, 100.0, 100.0], &[180.0, 0.0, 0.0], 580.0);
        assert!(approx_eq(&widths, &[180.0, 200.0, 200.0]));
    }

    #[test]
    fn fit_widths_returns_mins_when_viewport_below_sum_of_mins() {
        let mins = [120.0, 120.0, 120.0];
        let widths = fit_widths(&[100.0, 100.0, 100.0], &mins, 300.0);
        assert!(approx_eq(&widths, &mins));
    }

    #[test]
    fn fit_widths_sums_exactly_to_content_width() {
        let basis = [33.0, 33.0, 34.0, 7.0];
        let mins = [10.0, 10.0, 10.0, 10.0];
        let viewport = 777.0;
        let widths = fit_widths(&basis, &mins, viewport);
        assert_eq!(widths.iter().sum::<f32>(), content_width(&mins, viewport));
    }

    #[test]
    fn fit_widths_handles_single_column() {
        assert!(approx_eq(&fit_widths(&[1.0], &[40.0], 500.0), &[500.0]));
        assert!(approx_eq(&fit_widths(&[1.0], &[40.0], 20.0), &[40.0]));
    }

    #[test]
    fn fit_widths_pins_zero_basis_column_to_its_minimum() {
        let widths = fit_widths(&[0.0, 100.0], &[60.0, 0.0], 400.0);
        assert!(approx_eq(&widths, &[60.0, 340.0]));
    }

    #[test]
    fn fit_widths_all_zero_basis_falls_back_to_equal_split() {
        let widths = fit_widths(&[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0], 300.0);
        assert!(approx_eq(&widths, &[100.0, 100.0, 100.0]));
    }

    #[test]
    fn content_width_is_viewport_unless_mins_overflow() {
        assert_eq!(content_width(&[40.0, 40.0], 500.0), 500.0);
        assert_eq!(content_width(&[300.0, 300.0], 500.0), 600.0);
    }

    #[test]
    fn column_left_accumulates_preceding_widths() {
        let widths = [100.0, 150.0, 200.0];
        assert_eq!(column_left(&widths, 0), 0.0);
        assert_eq!(column_left(&widths, 2), 250.0);
        assert_eq!(column_left(&widths, 3), 450.0);
    }

    #[test]
    fn resize_columns_grows_left_and_cascades_right_to_mins() {
        let snapshot = [100.0, 100.0, 100.0];
        let mins = [40.0, 40.0, 40.0];
        // Drag border 0 far right: column 0 grows, both right columns hit min.
        let widths = resize_columns(&snapshot, &mins, 0, 500.0);
        assert!(approx_eq(&widths, &[220.0, 40.0, 40.0]));
        assert_eq!(widths.iter().sum::<f32>(), 300.0);
    }

    #[test]
    fn resize_columns_cascades_left_to_right_one_at_a_time() {
        let snapshot = [100.0, 100.0, 100.0];
        let mins = [40.0, 40.0, 40.0];
        // Grow column 0 by 80: neighbor shrinks 60 to min, then column 2 gives 20.
        let widths = resize_columns(&snapshot, &mins, 0, 180.0);
        assert!(approx_eq(&widths, &[180.0, 40.0, 80.0]));
    }

    #[test]
    fn resize_columns_stops_when_right_group_all_at_min() {
        let snapshot = [100.0, 100.0, 100.0];
        let mins = [40.0, 40.0, 40.0];
        let far = resize_columns(&snapshot, &mins, 0, 10_000.0);
        assert!(approx_eq(&far, &[220.0, 40.0, 40.0]));
    }

    #[test]
    fn resize_columns_drag_left_past_column_min_cascades_to_left_neighbor() {
        // border 1 = right edge of column 1 (snapshot x = 200)
        let snapshot = [100.0, 100.0, 100.0];
        let mins = [40.0, 40.0, 40.0];
        // Drag to x=120: column 1 gives 60 (to min=40), then column 0 gives 20.
        let widths = resize_columns(&snapshot, &mins, 1, 120.0);
        assert!(approx_eq(&widths, &[80.0, 40.0, 180.0]));
        assert_eq!(widths.iter().sum::<f32>(), 300.0);
    }

    #[test]
    fn resize_columns_drag_left_stops_when_left_group_all_at_min() {
        let snapshot = [100.0, 100.0, 100.0];
        let mins = [40.0, 40.0, 40.0];
        // Past full capacity: both left columns clamped to min.
        let widths = resize_columns(&snapshot, &mins, 1, 0.0);
        assert!(approx_eq(&widths, &[40.0, 40.0, 220.0]));
    }

    #[test]
    fn resize_columns_drag_left_then_back_is_reversible() {
        let snapshot = [120.0, 120.0, 160.0];
        let mins = [40.0, 40.0, 40.0];
        // Original border 0 sits at x = 120.
        let _moved = resize_columns(&snapshot, &mins, 0, 70.0);
        let restored = resize_columns(&snapshot, &mins, 0, 120.0);
        assert!(approx_eq(&restored, &snapshot));
    }

    #[test]
    fn divider_at_matches_internal_borders_only() {
        let widths = [100.0, 150.0, 200.0];
        // Right edge of column 0 at x=100, of column 1 at x=250.
        assert_eq!(divider_at(&widths, 102.0, DIVIDER_GRAB), Some(0));
        assert_eq!(divider_at(&widths, 250.0, DIVIDER_GRAB), Some(1));
        // The outer right edge at x=450 is not draggable.
        assert_eq!(divider_at(&widths, 450.0, DIVIDER_GRAB), None);
        // The outer left edge at x=0 is not draggable.
        assert_eq!(divider_at(&widths, 0.0, DIVIDER_GRAB), None);
        // Too far from any edge.
        assert_eq!(divider_at(&widths, 130.0, DIVIDER_GRAB), None);
    }

    #[test]
    fn visible_range_covers_partial_rows_at_both_edges() {
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
    fn max_scroll_x_is_zero_when_content_fits() {
        assert_eq!(max_scroll_x(400.0, 600.0), 0.0);
        assert_eq!(max_scroll_x(900.0, 600.0), 300.0);
    }

    #[test]
    fn row_at_skips_the_header_and_floors_to_row() {
        assert_eq!(row_at(20.0, 28.0, 20.0, 0.0, 10), None);
        assert_eq!(row_at(40.0, 28.0, 20.0, 0.0, 10), Some(0));
        assert_eq!(row_at(70.0, 28.0, 20.0, 0.0, 10), Some(2));
    }

    #[test]
    fn row_at_returns_none_past_the_last_row() {
        assert_eq!(row_at(1000.0, 28.0, 20.0, 0.0, 3), None);
    }
}
