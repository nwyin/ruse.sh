# ruse.sh

A Rust port of the [charm.sh](https://charm.sh) TUI ecosystem.

Ruse reimplements the core charm.sh Go libraries in idiomatic Rust, providing a complete terminal UI framework built on the Elm architecture.

## Crates

| Crate | Ports | Description |
|---|---|---|
| **ruse-runtime** | [bubbletea](https://github.com/charmbracelet/bubbletea) | Elm architecture runtime (Model/Update/View), async event loop, terminal management |
| **ruse-style** | [lipgloss](https://github.com/charmbracelet/lipgloss) | Terminal styling with 42+ properties, borders, layout, tables, tree rendering, layer compositing |
| **ruse-components** | [bubbles](https://github.com/charmbracelet/bubbles) | 14 UI components: text input, text area, viewport, list, table, file picker, spinner, progress, help, timer, stopwatch, cursor, paginator |
| **ruse-glamour** | [glamour](https://github.com/charmbracelet/glamour) | Markdown rendering with syntax highlighting, 4 built-in themes |
| **ruse-ansi** | [x/ansi](https://github.com/charmbracelet/x) | ANSI utilities: cell buffer with diff rendering, parser state machine, VT emulator, SGR builder, string operations |
| **ruse-colorprofile** | [colorprofile](https://github.com/charmbracelet/colorprofile) | Terminal color profile detection and color downsampling |
| **ruse-harmonica** | [harmonica](https://github.com/charmbracelet/harmonica) | Spring and projectile physics animations |
| **ruse** | - | Facade crate with prelude re-exporting all key types |

## Quick Start

```rust
use ruse::prelude::*;

struct Counter { count: i32 }

impl Model for Counter {
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
        let style = Style::new()
            .bold(true)
            .foreground(Color::parse("#04B575"))
            .border(ROUNDED_BORDER, &[true])
            .border_foreground(Color::parse("#874BFD"))
            .padding(&[1, 2]);

        View::new(style.render(&[&format!("Count: {}", self.count)]))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Counter { count: 0 };
    Program::new(model).with_alt_screen().run().await?;
    Ok(())
}
```

## Features

### Runtime
- Async event loop (tokio) with keyboard, mouse, paste, focus, and resize events
- Cell-buffer diff rendering with hash-based line matching and cursor optimization
- Synchronized output (mode 2026) for flicker-free updates
- Subprocess execution with terminal release/restore
- Suspend/resume (SIGTSTP/SIGCONT)
- External program control via `ProgramHandle` (send/quit/kill/wait)
- Message filtering middleware, external cancellation
- Panic recovery with terminal restoration

### Styling
- 42+ style properties with bitflags tracking
- 12-step render pipeline (tabs, wrap, SGR, padding, alignment, borders, margins)
- 10 border presets, per-side colors
- Horizontal/vertical joins, absolute placement
- Style inheritance, getter/unset methods
- `StyleRunes` / `StyleRanges` for fuzzy match highlighting
- Tree rendering with 8 enumerator types
- Layer/Compositor for overlapping content
- 2D gradient generation, LightDark/Complete color selection

### Components
- Customizable key bindings via KeyMap on every interactive component
- Text input with suggestions, word navigation, horizontal scrolling, validation
- Multi-line text area with soft wrapping
- Viewport with gutter function, highlights, horizontal scrolling
- List with fuzzy filtering, item delegate, status messages
- Table with keyboard navigation, half-page scrolling
- File picker with permissions, symlinks, type filtering
- Progress bar with gradient colors
- Help with column layout

### Terminal
- Full ANSI parser state machine (15 states, CSI/OSC/DCS/APC/SOS)
- VT102/xterm terminal emulator (~20 CSI commands)
- Kitty graphics protocol
- OSC sequences (clipboard, hyperlinks, window title, notifications)
- 256-color palette management
- Color profile detection (env vars, tmux subprocess, terminal-specific)

## Building

```sh
cargo build --workspace
cargo test --workspace
cargo run --example counter
```

## Examples

- `counter` - Simple counter with keyboard controls
- `spinner` - Animated spinner
- `textinput` - Text input with prompt
- `viewport` - Scrollable content viewer
- `todo` - Todo list with filtering

## License

MIT

## Acknowledgements

This project is a Rust port of the excellent [charm.sh](https://charm.sh) ecosystem by [Charmbracelet](https://github.com/charmbracelet). The original Go libraries are the foundation for all API design and architecture decisions.
