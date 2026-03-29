use ruse::prelude::*;

struct ViewportDemo {
    viewport: Viewport,
    ready: bool,
}

const SAMPLE_MARKDOWN: &str = r#"# Ruse.sh

A complete **Rust** port of the Charm.sh terminal UI stack.

## Features

- **Elm Architecture** — Model, Update, View pattern
- **Styling** — Borders, colors, padding, margins
- **Components** — TextInput, TextArea, List, Table, Viewport, Spinner, Progress
- **Markdown** — Render markdown to styled ANSI output
- **Animation** — Spring physics for smooth transitions

## Getting Started

```rust
use ruse::prelude::*;

struct MyApp {
    count: i32,
}

impl Model for MyApp {
    fn update(&mut self, msg: Msg) -> Cmd {
        if let Msg::KeyPress(key) = msg {
            match key.code {
                KeyCode::Up => self.count += 1,
                KeyCode::Down => self.count -= 1,
                KeyCode::Char('q') => return quit(),
                _ => {}
            }
        }
        None
    }

    fn view(&self) -> View {
        View::new(format!("Count: {}", self.count))
    }
}
```

## Architecture

Ruse is organized as a Cargo workspace with 8 crates:

| Crate | Purpose |
|-------|---------|
| ruse-ansi | ANSI parsing & generation |
| ruse-colorprofile | Terminal color detection |
| ruse-harmonica | Physics animations |
| ruse-style | Styling, layout, borders |
| ruse-runtime | Elm architecture runtime |
| ruse-components | UI components |
| ruse-glamour | Markdown rendering |
| ruse | Facade crate |

## Color Support

Ruse automatically detects terminal color capabilities:

- **TrueColor** — 16.7M colors (24-bit)
- **ANSI256** — 256 colors (8-bit)
- **ANSI** — 16 colors (4-bit)

Colors are automatically downsampled to match the terminal's capability.

## Border Styles

- Normal: ┌─┐│└─┘
- Rounded: ╭─╮│╰─╯
- Thick: ┏━┓┃┗━┛
- Double: ╔═╗║╚═╝
- Block: ████████
- ASCII: +-+|+-+

## Components

### Spinner
Animated loading indicators with 7 predefined styles.

### Progress
Smooth progress bars with spring-based animation.

### TextInput
Single-line text input with echo modes (normal, password, hidden).

### TextArea
Multi-line text editor with line numbers.

### Viewport
Scrollable content viewer (you're looking at one right now!).

### List
Filterable, scrollable list with fuzzy matching.

### Table
Interactive data table with selection and scrolling.

### Timer & Stopwatch
Time tracking components.

### FilePicker
File system browser with directory navigation.

---

*Built with Ruse.sh — the Rust TUI framework*
"#;

impl Model for ViewportDemo {
    fn init(&mut self) -> Cmd {
        // Set content immediately so first render has something to show
        let rendered = ruse::glamour::render_dark(SAMPLE_MARKDOWN);
        self.viewport.set_content(&rendered);
        self.ready = true;
        None
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        if let Msg::KeyPress(key) = &msg {
            match key.code {
                KeyCode::Char('q') | KeyCode::Escape => return quit(),
                KeyCode::Char('c') if key.modifiers.contains(Modifiers::CTRL) => return quit(),
                _ => {}
            }
        }

        if let Msg::WindowSize { width, height } = &msg {
            self.viewport.set_width(*width as usize - 4); // account for border
            self.viewport.set_height(*height as usize - 6); // account for border + header/footer
            if !self.ready {
                let rendered = ruse::glamour::render_dark(SAMPLE_MARKDOWN);
                self.viewport.set_content(&rendered);
                self.ready = true;
            }
        }

        self.viewport.update(&msg)
    }

    fn view(&self) -> View {
        let title = Style::new()
            .bold(true)
            .foreground(Color::parse("#ff6600"))
            .render(&["Viewport Demo — Ruse.sh Readme"]);

        let percent = (self.viewport.scroll_percent() * 100.0) as u32;
        let footer = Style::new().faint(true).render(&[&format!(
            "↑/↓: scroll • pgup/pgdn: page • q: quit • {}%",
            percent
        )]);

        let content = format!("{}\n{}\n{}", title, self.viewport.view(), footer);

        let box_style = Style::new()
            .border(ROUNDED_BORDER, &[true])
            .border_foreground(Color::parse("#874BFD"))
            .padding(&[0, 1]);

        View::new(box_style.render(&[&content])).with_alt_screen()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = ViewportDemo {
        viewport: Viewport::new(80, 24),
        ready: false,
    };
    Program::new(model).with_alt_screen().run().await?;
    Ok(())
}
