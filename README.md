# iced_table

A canvas-rendered `DataTable` widget for [iced](https://iced.rs) 0.14.

## Features

- Row virtualization
- Drag-to-resize columns
- Hover and active-row highlighting
- Tree/hierarchy support — indent guides, expand/collapse chevrons
- Zebra striping and themeable style
- Custom fonts per column or per cell

## Example

```rust
use iced_table::{Cell, Column, DataTable, Row, Toggle};

let columns = vec![
    Column::new("Name").width(200.0),
    Column::new("Value").width(120.0),
];

let rows: Vec<Row> = items
    .iter()
    .map(|item| Row {
        depth: 0,
        toggle: Toggle::None,
        cells: &[Cell::text(item.name.clone()), Cell::text(item.value.clone())],
    })
    .collect();

DataTable::new(columns, rows)
    .row_height(24.0)
    .on_row_press(Message::RowPressed)
    .into()
```

Run the full demo:

```
cargo run --example data_table_demo
```
