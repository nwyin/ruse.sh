use rouge_runtime::{Cmd, KeyCode, Msg};
use rouge_style::Style;

pub struct TableColumn {
    pub title: String,
    pub width: usize,
}

pub struct TableStyles {
    pub header: Style,
    pub cell: Style,
    pub selected: Style,
}

impl Default for TableStyles {
    fn default() -> Self {
        Self {
            header: Style::new().bold(true),
            cell: Style::new(),
            selected: Style::new().reverse(true),
        }
    }
}

pub struct Table {
    columns: Vec<TableColumn>,
    rows: Vec<Vec<String>>,
    cursor: usize,
    focus: bool,
    height: usize,
    y_offset: usize,
    styles: TableStyles,
}

impl Table {
    pub fn new(columns: Vec<TableColumn>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            cursor: 0,
            focus: false,
            height: 10,
            y_offset: 0,
            styles: TableStyles::default(),
        }
    }

    pub fn with_rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn with_height(mut self, h: usize) -> Self {
        self.height = h;
        self
    }

    pub fn with_styles(mut self, styles: TableStyles) -> Self {
        self.styles = styles;
        self
    }

    pub fn set_rows(&mut self, rows: Vec<Vec<String>>) {
        self.rows = rows;
        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
        self.ensure_cursor_visible();
    }

    pub fn selected_row(&self) -> usize {
        self.cursor
    }

    pub fn selected_row_data(&self) -> Option<&Vec<String>> {
        self.rows.get(self.cursor)
    }

    pub fn focus(&mut self) {
        self.focus = true;
    }

    pub fn blur(&mut self) {
        self.focus = false;
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if !self.focus {
            return None;
        }

        if let Msg::KeyPress(key) = msg { match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.ensure_cursor_visible();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor < self.rows.len().saturating_sub(1) {
                    self.cursor += 1;
                    self.ensure_cursor_visible();
                }
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(self.visible_rows());
                self.ensure_cursor_visible();
            }
            KeyCode::PageDown => {
                self.cursor = (self.cursor + self.visible_rows()).min(self.rows.len().saturating_sub(1));
                self.ensure_cursor_visible();
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.cursor = 0;
                self.ensure_cursor_visible();
            }
            KeyCode::End | KeyCode::Char('G') => {
                if !self.rows.is_empty() {
                    self.cursor = self.rows.len() - 1;
                    self.ensure_cursor_visible();
                }
            }
            _ => {}
        } }
        None
    }

    pub fn view(&self) -> String {
        if self.columns.is_empty() {
            return String::new();
        }

        let mut out = String::new();

        // Header
        let header = self.render_row_cells(
            &self.columns.iter().map(|c| c.title.clone()).collect::<Vec<_>>(),
            &self.styles.header,
        );
        out.push_str(&header);

        // Separator
        out.push('\n');
        let sep: String = self
            .columns
            .iter()
            .map(|c| "─".repeat(c.width))
            .collect::<Vec<_>>()
            .join("─");
        out.push_str(&sep);

        // Rows
        let visible = self.visible_rows();
        let end = (self.y_offset + visible).min(self.rows.len());

        for i in self.y_offset..end {
            out.push('\n');
            let style = if i == self.cursor && self.focus {
                &self.styles.selected
            } else {
                &self.styles.cell
            };
            out.push_str(&self.render_row_cells(&self.rows[i], style));
        }

        // Pad remaining height
        for _ in (end - self.y_offset)..visible {
            out.push('\n');
            let empty_row: Vec<String> = self.columns.iter().map(|_| String::new()).collect();
            out.push_str(&self.render_row_cells(&empty_row, &self.styles.cell));
        }

        out
    }

    fn render_row_cells(&self, cells: &[String], style: &Style) -> String {
        let mut parts = Vec::new();
        for (i, col) in self.columns.iter().enumerate() {
            let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
            let cell_width = rouge_ansi::string_width(cell);
            let rendered = if cell_width > col.width {
                rouge_ansi::truncate(cell, col.width, "")
            } else {
                let padding = col.width - cell_width;
                format!("{}{}", cell, " ".repeat(padding))
            };
            parts.push(rendered);
        }
        let row = parts.join(" ");
        style.render(&[&row])
    }

    fn visible_rows(&self) -> usize {
        // Subtract 2 for header + separator
        self.height.saturating_sub(2)
    }

    fn ensure_cursor_visible(&mut self) {
        let visible = self.visible_rows();
        if visible == 0 {
            return;
        }
        if self.cursor < self.y_offset {
            self.y_offset = self.cursor;
        } else if self.cursor >= self.y_offset + visible {
            self.y_offset = self.cursor - visible + 1;
        }
    }
}
