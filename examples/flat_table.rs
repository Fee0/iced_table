//! Minimal example: a flat, three-column table with no tree functionality.
//!
//! Name and Score are sortable; Status is not, so clicking its header does
//! nothing. No sort chevron SVGs are supplied, so the indicator falls back to
//! the path-drawn triangle.

use iced::widget::{column, container};
use iced::{Element, Length, Task};
use iced_table::style::Status;
use iced_table::{Cell, CellAlign, Column, DataTable, Row, Sort, TextRole, sort};

/// Index of the sortable score column, shared by the header and the comparator.
const SCORE_COLUMN: usize = 2;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Flat table")
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    RowPressed(usize),
    Sorted(Sort),
}

struct Person {
    name: &'static str,
    status: &'static str,
    score: u32,
}

struct App {
    people: Vec<Person>,
    selected: Option<usize>,
    sort: Option<Sort>,
}

impl App {
    fn new() -> Self {
        Self {
            people: sample_people(),
            selected: None,
            sort: None,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RowPressed(index) => self.selected = Some(index),
            Message::Sorted(sort) => {
                self.sort = Some(sort);
                self.people.sort_by(|a, b| {
                    let order = match sort.column {
                        SCORE_COLUMN => a.score.cmp(&b.score),
                        _ => a.name.cmp(b.name),
                    };
                    match sort.direction {
                        sort::Direction::Ascending => order,
                        sort::Direction::Descending => order.reverse(),
                    }
                });
                self.selected = None;
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let columns = vec![
            Column::new("Name").width(200.0).sortable(true),
            Column::new("Status").width(120.0).align(CellAlign::Center),
            Column::new("Score")
                .width(80.0)
                .align(CellAlign::End)
                .sortable(true),
        ];

        let rows = self
            .people
            .iter()
            .map(|person| {
                Row::new(vec![
                    Cell::text(person.name),
                    Cell::text(person.status).role(TextRole::Accent),
                    Cell::text(person.score.to_string()).role(TextRole::Muted),
                ])
            })
            .collect();

        let table = DataTable::new(columns, rows)
            .active_row(self.selected)
            .sort(self.sort)
            .on_row_press(Message::RowPressed)
            .on_sort(Message::Sorted)
            .style(table_style);

        container(column![table])
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .into()
    }
}

/// A style with a border around the whole table and a divider under the header.
fn table_style(theme: &iced::Theme, status: Status) -> iced_table::style::Style {
    let palette = theme.extended_palette();
    let mut style = iced_table::style::default(theme, status);
    style.border = Some(palette.background.strong.color);
    style.header_divider = Some(palette.background.strong.color);
    style
}

fn sample_people() -> Vec<Person> {
    [
        ("Alice", "Active", 98),
        ("Bob", "Inactive", 74),
        ("Charlie", "Active", 85),
        ("Diana", "Pending", 61),
        ("Eve", "Active", 92),
    ]
    .into_iter()
    .map(|(name, status, score)| Person {
        name,
        status,
        score,
    })
    .collect()
}
