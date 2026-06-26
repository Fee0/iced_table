//! Minimal example: a flat, three-column table with no tree functionality.

use iced::widget::{column, container};
use iced::{Element, Length, Task};
use iced_table::{Cell, CellAlign, Column, DataTable, Row, TextRole};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Flat table")
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    RowPressed(usize),
}

struct App {
    selected: Option<usize>,
}

impl App {
    fn new() -> Self {
        Self { selected: None }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RowPressed(index) => self.selected = Some(index),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let columns = vec![
            Column::new("Name").width(200.0),
            Column::new("Status").width(120.0).align(CellAlign::Center),
            Column::new("Score").width(80.0).align(CellAlign::End),
        ];

        let data = [
            ("Alice", "Active", "98"),
            ("Bob", "Inactive", "74"),
            ("Charlie", "Active", "85"),
            ("Diana", "Pending", "61"),
            ("Eve", "Active", "92"),
        ];

        let rows = data
            .iter()
            .map(|(name, status, score)| {
                Row::new(vec![
                    Cell::text(name.to_string()),
                    Cell::text(status.to_string()).role(TextRole::Accent),
                    Cell::text(score.to_string()).role(TextRole::Muted),
                ])
            })
            .collect();

        let table = DataTable::new(columns, rows)
            .active_row(self.selected)
            .on_row_press(Message::RowPressed);

        container(column![table])
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .into()
    }
}
