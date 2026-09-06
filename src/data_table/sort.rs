//! Sort state for [`DataTable`](crate::DataTable).

/// The column a table is sorted by, and the direction it is sorted in.
///
/// The widget never sorts anything itself: the consumer owns this value, hands
/// it back through [`DataTable::sort`](crate::DataTable::sort), and receives the
/// next one from [`DataTable::on_sort`](crate::DataTable::on_sort).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sort {
    /// Index of the sorted column.
    pub column: usize,
    /// The order that column is arranged in.
    pub direction: Direction,
}

impl Sort {
    /// The sort a header click on `column` produces, given the `current` one.
    ///
    /// Clicking the already-sorted column reverses it; any other column starts
    /// out [`Direction::Descending`].
    pub fn toggled(current: Option<Self>, column: usize) -> Self {
        match current {
            Some(sort) if sort.column == column => Self {
                column,
                direction: sort.direction.reversed(),
            },
            _ => Self {
                column,
                direction: Direction::Descending,
            },
        }
    }
}

/// The order a sorted column is arranged in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// Largest first.
    #[default]
    Descending,
    /// Smallest first.
    Ascending,
}

impl Direction {
    /// The opposite direction.
    pub fn reversed(self) -> Self {
        match self {
            Self::Descending => Self::Ascending,
            Self::Ascending => Self::Descending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_column_starts_descending() {
        assert_eq!(
            Sort::toggled(None, 2),
            Sort {
                column: 2,
                direction: Direction::Descending,
            }
        );
    }

    #[test]
    fn clicking_the_sorted_column_reverses_it() {
        let descending = Sort {
            column: 1,
            direction: Direction::Descending,
        };
        let ascending = Sort::toggled(Some(descending), 1);

        assert_eq!(ascending.direction, Direction::Ascending);
        assert_eq!(Sort::toggled(Some(ascending), 1), descending);
    }

    #[test]
    fn switching_columns_restarts_descending() {
        let current = Sort {
            column: 0,
            direction: Direction::Ascending,
        };

        assert_eq!(
            Sort::toggled(Some(current), 1),
            Sort {
                column: 1,
                direction: Direction::Descending,
            }
        );
    }
}
