use ruse::prelude::*;

struct TodoItem {
    title: String,
    done: bool,
}

impl ListItem for TodoItem {
    fn filter_value(&self) -> &str {
        &self.title
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn description(&self) -> &str {
        if self.done {
            "done"
        } else {
            "pending"
        }
    }
}

enum AppState {
    Browsing,
    Adding,
}

struct TodoApp {
    list: List,
    items: Vec<TodoItem>,
    input: TextInput,
    state: AppState,
    width: usize,
    height: usize,
}

impl TodoApp {
    fn rebuild_list(&mut self) {
        let list_items: Vec<Box<dyn ListItem>> = self
            .items
            .iter()
            .map(|item| {
                Box::new(SimpleItem {
                    title: format!(
                        "{} {}",
                        if item.done { "[x]" } else { "[ ]" },
                        item.title
                    ),
                    desc: if item.done {
                        "done".to_string()
                    } else {
                        "pending".to_string()
                    },
                    filter_val: item.title.clone(),
                }) as Box<dyn ListItem>
            })
            .collect();
        self.list.set_items(list_items);
    }
}

impl Model for TodoApp {
    fn init(&mut self) -> Cmd {
        self.rebuild_list();
        None
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        match self.state {
            AppState::Browsing => {
                if let Msg::KeyPress(key) = &msg {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Escape => return quit(),
                        KeyCode::Char('c') if key.modifiers.contains(Modifiers::CTRL) => {
                            return quit()
                        }
                        KeyCode::Char('a') => {
                            self.state = AppState::Adding;
                            self.input.set_value("");
                            return self.input.focus();
                        }
                        KeyCode::Char('x') | KeyCode::Char(' ') => {
                            let idx = self.list.selected_index();
                            if idx < self.items.len() {
                                self.items[idx].done = !self.items[idx].done;
                                self.rebuild_list();
                            }
                            return None;
                        }
                        KeyCode::Char('d') | KeyCode::Delete => {
                            let idx = self.list.selected_index();
                            if idx < self.items.len() {
                                self.items.remove(idx);
                                self.rebuild_list();
                            }
                            return None;
                        }
                        _ => {}
                    }
                }

                if let Msg::WindowSize { width, height } = &msg {
                    self.width = *width as usize;
                    self.height = *height as usize;
                }

                self.list.update(&msg)
            }
            AppState::Adding => {
                if let Msg::KeyPress(key) = &msg {
                    match key.code {
                        KeyCode::Escape => {
                            self.state = AppState::Browsing;
                            self.input.blur();
                            return None;
                        }
                        KeyCode::Enter => {
                            let val = self.input.value();
                            if !val.is_empty() {
                                self.items.push(TodoItem {
                                    title: val,
                                    done: false,
                                });
                                self.rebuild_list();
                            }
                            self.state = AppState::Browsing;
                            self.input.blur();
                            return None;
                        }
                        _ => {}
                    }
                }
                self.input.update(&msg)
            }
        }
    }

    fn view(&self) -> View {
        let title = Style::new()
            .bold(true)
            .foreground(Color::parse("#ff6600"))
            .render(&["Todo List"]);

        let mut content = vec![title, String::new()];

        match self.state {
            AppState::Browsing => {
                content.push(self.list.view());
                content.push(String::new());

                let done_count = self.items.iter().filter(|i| i.done).count();
                let total = self.items.len();
                let status = Style::new().faint(true).render(&[&format!(
                    "{}/{} done • a: add • x: toggle • d: delete • /: filter • q: quit",
                    done_count, total
                )]);
                content.push(status);
            }
            AppState::Adding => {
                content.push(format!("  {}", self.input.view()));
                content.push(String::new());
                content.push(
                    Style::new()
                        .faint(true)
                        .render(&["enter: add • esc: cancel"]),
                );
            }
        }

        let box_style = Style::new()
            .border(ROUNDED_BORDER, &[true])
            .border_foreground(Color::parse("#874BFD"))
            .padding(&[1, 2]);

        View::new(box_style.render(&[&content.join("\n")])).with_alt_screen()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = vec![
        TodoItem { title: "Buy groceries".into(), done: false },
        TodoItem { title: "Write Rust code".into(), done: true },
        TodoItem { title: "Read a book".into(), done: false },
        TodoItem { title: "Go for a walk".into(), done: false },
        TodoItem { title: "Learn ruse.sh".into(), done: true },
    ];

    let model = TodoApp {
        list: List::new(vec![], 60, 10).with_title("Tasks"),
        items,
        input: TextInput::new()
            .with_prompt("New task: ")
            .with_placeholder("What needs to be done?")
            .with_width(50),
        state: AppState::Browsing,
        width: 80,
        height: 24,
    };

    Program::new(model).with_alt_screen().run().await?;
    Ok(())
}
