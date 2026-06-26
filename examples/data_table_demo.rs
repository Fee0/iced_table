//! A standalone demo exercising [`DataTable`] the way a hierarchy pane would:
//! the consumer owns the collapse state and column widths, flattens its tree
//! into a flat row list each frame, and maps row indices back to its domain.

use iced::advanced::svg;
use iced::widget::{column, container, text};
use iced::{Element, Length, Task};
use iced_table::data_table::style::Status;
use iced_table::{Cell, CellAlign, Column, DataTable, FontKind, Row, TextRole, Toggle, Weight};

fn main() -> iced::Result {
    iced::application(Demo::new, Demo::update, Demo::view)
        .title("DataTable demo")
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    RowPressed(usize),
    TogglePressed(usize),
    Hovered(Option<usize>),
}

struct Demo {
    roots: Vec<Node>,
    visible: Vec<VisibleRow>,
    selected: Option<Vec<usize>>,
    hovered: Option<usize>,
    revision: u64,
}

/// A node in the consumer's own tree. The widget never sees this type.
struct Node {
    name: String,
    kind: Option<&'static str>,
    address: u64,
    collapsed: bool,
    children: Vec<Node>,
}

/// A flattened, currently-visible row, keyed by its tree path.
struct VisibleRow {
    path: Vec<usize>,
    depth: u16,
    toggle: Toggle,
    cells: [Cell<'static>; 3],
}

impl Demo {
    fn new() -> Self {
        let mut demo = Self {
            roots: sample_tree(),
            visible: Vec::new(),
            selected: None,
            hovered: None,
            revision: 0,
        };
        demo.rebuild();
        demo
    }

    /// Recomputes the flattened visible rows after any change to the tree or its
    /// collapse state.
    fn rebuild(&mut self) {
        self.visible.clear();
        flatten(&self.roots, 0, &mut Vec::new(), &mut self.visible);
    }

    fn node_at_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        let (first, rest) = path.split_first()?;
        let mut node = self.roots.get_mut(*first)?;
        for index in rest {
            node = node.children.get_mut(*index)?;
        }
        Some(node)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RowPressed(index) => {
                self.selected = self.visible.get(index).map(|row| row.path.clone());
            }
            Message::TogglePressed(index) => {
                if let Some(path) = self.visible.get(index).map(|row| row.path.clone()) {
                    if let Some(node) = self.node_at_mut(&path) {
                        node.collapsed = !node.collapsed;
                    }
                    self.rebuild();
                    self.revision += 1;
                }
            }
            Message::Hovered(row) => self.hovered = row,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let columns = vec![
            Column::new("Name")
                .width(220.0)
                .min_width(20.0)
                .tree_column(true),
            Column::new("Type")
                .width(140.0)
                .min_width(80.0)
                .align(CellAlign::Start),
            Column::new("Address")
                .width(120.0)
                .min_width(90.0)
                .align(CellAlign::End),
        ];

        let rows = self
            .visible
            .iter()
            .map(|row| Row {
                depth: row.depth,
                toggle: row.toggle,
                cells: row.cells.to_vec(),
            })
            .collect();

        let active = self
            .selected
            .as_ref()
            .and_then(|path| self.visible.iter().position(|row| &row.path == path));

        let table = DataTable::new(columns, rows)
            .row_height(26.0)
            .header_height(30.0)
            .active_row(active)
            .revision(self.revision)
            .on_row_press(Message::RowPressed)
            .on_toggle_press(Message::TogglePressed)
            .on_hover(Message::Hovered)
            .chevron_svg(
                svg::Handle::from_path("assets/svg/chevron_right.svg"),
                svg::Handle::from_path("assets/svg/chevron_down.svg"),
            )
            .style(table_style);

        let status = text(format!(
            "rows: {}   hovered: {}   selected: {}",
            self.visible.len(),
            self.hovered
                .map_or_else(|| "-".to_string(), |row| row.to_string()),
            active.map_or_else(|| "-".to_string(), |row| row.to_string()),
        ))
        .size(13);

        container(column![status, table].spacing(8))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .into()
    }
}

/// A custom style with a visible zebra band, to show role-aware text colors.
fn table_style(theme: &iced::Theme, status: Status) -> iced_table::style::Style {
    let palette = theme.extended_palette();
    let mut style = iced_table::style::default(theme, status);
    style.row_background_alternate = Some(palette.background.weakest.color);
    style.row_divider = Some(palette.background.weak.color);
    style.border = Some(palette.background.weak.color);
    style
}

/// Flattens the tree into visible rows with owned cells, honoring collapse state.
fn flatten(nodes: &[Node], depth: u16, path: &mut Vec<usize>, visible: &mut Vec<VisibleRow>) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);

        let has_children = !node.children.is_empty();
        let toggle = match (has_children, node.collapsed) {
            (false, _) => Toggle::None,
            (true, true) => Toggle::Collapsed,
            (true, false) => Toggle::Expanded,
        };

        let name_weight = if has_children {
            Weight::Bold
        } else {
            Weight::Regular
        };
        let type_cell = match node.kind {
            Some(kind) => Cell::text(kind.to_string()).role(TextRole::Accent),
            None => Cell::Empty,
        };

        visible.push(VisibleRow {
            path: path.clone(),
            depth,
            toggle,
            cells: [
                Cell::text(node.name.clone()).weight(name_weight),
                type_cell,
                Cell::text(format!("0x{:08X}", node.address))
                    .font_kind(FontKind::Editor)
                    .role(TextRole::Muted),
            ],
        });

        if has_children && !node.collapsed {
            flatten(&node.children, depth + 1, path, visible);
        }

        path.pop();
    }
}

fn leaf(name: &str, kind: &'static str, address: u64) -> Node {
    Node {
        name: name.to_string(),
        kind: Some(kind),
        address,
        collapsed: false,
        children: Vec::new(),
    }
}

fn folder(name: &str, address: u64, children: Vec<Node>) -> Node {
    Node {
        name: name.to_string(),
        kind: None,
        address,
        collapsed: false,
        children,
    }
}

fn sample_tree() -> Vec<Node> {
    let mut roots = Vec::new();
    for section in 0..6 {
        let base = (section as u64) * 0x1000;
        let leaves = (0..40)
            .map(|i| leaf(&format!("field_{section}_{i}"), "u32", base + i * 4))
            .collect();
        roots.push(folder(
            &format!("section_{section}"),
            base,
            vec![
                folder(
                    &format!("header_{section}"),
                    base,
                    vec![leaf("magic", "u32", base), leaf("version", "u16", base + 4)],
                ),
                folder(&format!("entries_{section}"), base + 0x100, leaves),
            ],
        ));
    }
    roots
}
