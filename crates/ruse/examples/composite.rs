use ruse::prelude::*;

// --- A custom pane wrapping a List with some styling ---

struct ItemListPane {
    list: List,
}

impl ItemListPane {
    fn new(items: Vec<Box<dyn ListItem>>, width: usize, height: usize) -> Self {
        Self {
            list: List::new(items, width, height).with_title("Items"),
        }
    }
}

impl Pane for ItemListPane {
    fn update(&mut self, msg: &Msg) -> Cmd {
        self.list.update(msg)
    }

    fn view(&self) -> String {
        let border = if self.list.focused() {
            Style::new()
                .border(ROUNDED_BORDER, &[true])
                .border_foreground(Color::parse("#874BFD"))
        } else {
            Style::new()
                .border(ROUNDED_BORDER, &[true])
                .border_foreground(Color::parse("#555555"))
        };
        border.render(&[&self.list.view()])
    }

    fn focus(&mut self) -> Cmd {
        self.list.focus();
        None
    }

    fn blur(&mut self) {
        self.list.blur();
    }

    fn focused(&self) -> bool {
        self.list.focused()
    }
}

// --- A custom pane wrapping a Viewport ---

struct DetailPane {
    viewport: Viewport,
    is_focused: bool,
}

impl DetailPane {
    fn new(width: usize, height: usize) -> Self {
        let mut vp = Viewport::new(width, height);
        vp.set_content("Select an item to see details.");
        Self {
            viewport: vp,
            is_focused: false,
        }
    }
}

impl Pane for DetailPane {
    fn update(&mut self, msg: &Msg) -> Cmd {
        self.viewport.update(msg)
    }

    fn view(&self) -> String {
        let border = if self.is_focused {
            Style::new()
                .border(ROUNDED_BORDER, &[true])
                .border_foreground(Color::parse("#874BFD"))
        } else {
            Style::new()
                .border(ROUNDED_BORDER, &[true])
                .border_foreground(Color::parse("#555555"))
        };
        border.render(&[&self.viewport.view()])
    }

    fn focus(&mut self) -> Cmd {
        self.is_focused = true;
        None
    }

    fn blur(&mut self) {
        self.is_focused = false;
    }

    fn focused(&self) -> bool {
        self.is_focused
    }
}

// --- A modal search pane ---

struct SearchModal {
    input: TextInput,
}

impl SearchModal {
    fn new() -> Self {
        Self {
            input: TextInput::new()
                .with_prompt("Search: ")
                .with_placeholder("type to search...")
                .with_width(30),
        }
    }
}

impl Pane for SearchModal {
    fn update(&mut self, msg: &Msg) -> Cmd {
        self.input.update(msg)
    }

    fn view(&self) -> String {
        let border = Style::new()
            .border(DOUBLE_BORDER, &[true])
            .border_foreground(Color::parse("#ff6600"))
            .padding(&[0, 1]);
        border.render(&[&self.input.view()])
    }

    fn focus(&mut self) -> Cmd {
        self.input.focus()
    }

    fn blur(&mut self) {
        self.input.blur();
    }

    fn focused(&self) -> bool {
        self.input.focused()
    }
}

// --- Items for the list ---

struct DemoItem {
    title: String,
    detail: String,
}

impl ListItem for DemoItem {
    fn filter_value(&self) -> &str {
        &self.title
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn description(&self) -> &str {
        ""
    }
}

// --- App ---

struct App {
    scene: Scene,
    items: Vec<DemoItem>,
    width: u16,
    height: u16,
}

impl App {
    fn relayout(&mut self) {
        let full = Rect::full(self.width, self.height.saturating_sub(1));
        let (left, right) = full.split_horizontal(self.width / 2);
        self.scene.set_layout("list", PaneLayout::new(left, 0));
        self.scene.set_layout("detail", PaneLayout::new(right, 0));

        // Resize the viewport to fit within its border (2 chars each side)
        if let Some(detail_pane) = self.scene.pane_as_mut::<DetailPane>("detail") {
            detail_pane
                .viewport
                .set_width(right.width.saturating_sub(4) as usize);
            detail_pane
                .viewport
                .set_height(right.height.saturating_sub(4) as usize);
        }

        if self.scene.contains("search") {
            let modal_w = 36u16.min(self.width);
            let modal_h = 3u16.min(self.height);
            let modal_x = self.width.saturating_sub(modal_w) / 2;
            let modal_y = self.height.saturating_sub(modal_h) / 2;
            self.scene.set_layout(
                "search",
                PaneLayout::new(Rect::new(modal_x, modal_y, modal_w, modal_h), 1),
            );
        }
    }

    fn update_detail(&mut self) {
        if let Some(list_pane) = self.scene.pane_as::<ItemListPane>("list") {
            let idx = list_pane.list.selected_index();
            if idx < self.items.len() {
                if let Some(detail_pane) = self.scene.pane_as_mut::<DetailPane>("detail") {
                    detail_pane.viewport.set_content(&self.items[idx].detail);
                }
            }
        }
    }
}

impl Model for App {
    fn init(&mut self) -> Cmd {
        let cmd = self.scene.init_all();
        self.scene.set_focus("list");
        cmd
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        if let Msg::WindowSize { width, height } = &msg {
            self.width = *width;
            self.height = *height;
            self.relayout();
        }

        // Global keybindings (before scene routing)
        if let Msg::KeyPress(key) = &msg {
            match key.code {
                KeyCode::Char('q') if !self.scene.contains("search") => return quit(),
                KeyCode::Char('c') if key.modifiers.contains(Modifiers::CTRL) => return quit(),
                KeyCode::Char('/') if !self.scene.contains("search") => {
                    let modal_w = 36u16.min(self.width);
                    let modal_h = 3u16.min(self.height);
                    let modal_x = self.width.saturating_sub(modal_w) / 2;
                    let modal_y = self.height.saturating_sub(modal_h) / 2;
                    let layout = PaneLayout::new(Rect::new(modal_x, modal_y, modal_w, modal_h), 1);
                    self.scene.add("search", SearchModal::new(), layout);
                    return self.scene.set_focus("search");
                }
                KeyCode::Escape if self.scene.contains("search") => {
                    self.scene.remove("search");
                    return self.scene.set_focus("list");
                }
                KeyCode::Tab if !self.scene.contains("search") => {
                    let next = if self.scene.focused() == Some("list") {
                        "detail"
                    } else {
                        "list"
                    };
                    return self.scene.set_focus(next);
                }
                _ => {}
            }
        }

        let cmd = self.scene.update(&msg);
        self.update_detail();
        cmd
    }

    fn view(&self) -> View {
        let mut view = self.scene.view();

        // Add a status bar at the bottom
        let focus_label = self.scene.focused().unwrap_or("none");
        let status = Style::new().faint(true).render(&[&format!(
            " tab: switch pane | /: search | q: quit | focus: {}",
            focus_label
        )]);
        let status_rect = Rect::new(0, self.height.saturating_sub(1), self.width, 1);
        view.regions.push((status_rect, status));

        view
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = vec![
        DemoItem {
            title: "Pane Trait".into(),
            detail: "The Pane trait formalizes the component interface.\n\n\
                     Every component that implements Pane can be added\n\
                     to a Scene for automatic message routing and\n\
                     view composition."
                .into(),
        },
        DemoItem {
            title: "Scene Compositor".into(),
            detail: "Scene manages multiple Panes with:\n\n\
                     - Input routing to focused pane\n\
                     - Background event broadcast\n\
                     - Command tagging\n\
                     - Region-based view composition\n\
                     - Z-ordered overlapping"
                .into(),
        },
        DemoItem {
            title: "Message Routing".into(),
            detail: "Keyboard and paste events go to the\n\
                     focused pane only.\n\n\
                     Mouse events are hit-tested against\n\
                     pane rectangles.\n\n\
                     Timer ticks and resize events are\n\
                     broadcast to all visible panes."
                .into(),
        },
        DemoItem {
            title: "Command Tagging".into(),
            detail: "When a pane returns a Cmd, Scene wraps\n\
                     it so the resulting Msg routes back to\n\
                     that specific pane.\n\n\
                     This prevents cross-talk between panes\n\
                     that use Msg::Custom for their own ticks."
                .into(),
        },
        DemoItem {
            title: "Modal Overlays".into(),
            detail: "Press '/' to open a search modal.\n\n\
                     The modal appears at z=1, overlapping\n\
                     the main panes at z=0.\n\n\
                     Input routes to the modal while it's\n\
                     focused. Press Esc to close."
                .into(),
        },
    ];

    let list_items: Vec<Box<dyn ListItem>> = items
        .iter()
        .map(|item| {
            Box::new(SimpleItem {
                title: item.title.clone(),
                desc: String::new(),
                filter_val: item.title.clone(),
            }) as Box<dyn ListItem>
        })
        .collect();

    let mut scene = Scene::new();
    scene.add(
        "list",
        ItemListPane::new(list_items, 36, 10),
        PaneLayout::new(Rect::new(0, 0, 40, 23), 0),
    );
    scene.add(
        "detail",
        DetailPane::new(36, 10),
        PaneLayout::new(Rect::new(40, 0, 40, 23), 0),
    );

    let model = App {
        scene,
        items,
        width: 80,
        height: 24,
    };

    Program::new(model).with_alt_screen().run().await?;
    Ok(())
}
