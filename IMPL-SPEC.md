# Rouge.sh Implementation Spec

## Requirements

- Full port of all 8 crates: rouge-ansi, rouge-colorprofile, rouge-harmonica, rouge-style, rouge-runtime, rouge-components (all 14), rouge-glamour, rouge facade
- Rust 2024 edition, latest stable, no MSRV constraint
- Tokio-only async runtime (no sync/blocking mode)
- Unix-first (macOS/Linux). Windows may compile via crossterm but not a target
- Both static table (lipgloss port in rouge-style) and interactive table (bubbles port in rouge-components)
- Port Go JSON theme files for glamour (dracula, tokyo-night, dark, light) via serde
- Parallel agent implementation for independent crates

## Verification

- `cargo build --workspace` compiles cleanly
- `cargo clippy --workspace` passes (no errors)
- Example apps run interactively in terminal:
  - `cargo run --example counter` — keyboard-driven counter (up/down/quit)
  - `cargo run --example todo` — todo list with filtering
  - `cargo run --example textinput` — text input form
  - `cargo run --example viewport` — scrollable content viewer
  - `cargo run --example spinner` — spinner animation demo

## Success Criteria

- All verification commands pass
- Workspace has 8 crates with correct dependency graph
- Examples exercise: runtime event loop, styling/borders, at least 3 components, keyboard input, mouse support, alt screen

---

## Spec 1: Foundation Crates (parallel)

### Requirements

- **rouge-ansi**: SgrStyle builder (bold/italic/colors/etc -> ESC sequences), string_width (ANSI-aware), strip, truncate, wrap, height. Uses unicode-width + unicode-segmentation.
- **rouge-harmonica**: Spring (damped harmonic oscillator with 3 damping branches), Projectile (3D point/vector physics), fps() helper. Zero dependencies, pure math.
- **rouge-colorprofile**: Profile enum (NoTty/Ascii/Ansi/Ansi256/TrueColor) with Ord, detect() from env vars + TERM + COLORTERM + NO_COLOR, convert() for color downsampling, Writer for transparent SGR rewriting. Depends on rouge-ansi.

### Success Criteria

- `cargo build -p rouge-ansi -p rouge-harmonica -p rouge-colorprofile`
- Unit tests for: SGR generation, string_width with ANSI codes, strip, wrap, spring math (all 3 damping modes), profile detection from env vars, color conversion

---

## Spec 2: Styling & Layout (rouge-style)

### Prerequisites: Spec 1

### Requirements

- Color enum (NoColor/Basic/Indexed/Rgb) with parse(), darken/lighten/complementary, blend_1d/blend_2d (CIELAB via palette crate)
- Style struct with ~50 properties via bitflags u64, builder pattern (consume self -> Self), inherit(), all setters/unsetters
- Full 13-step render pipeline: transform -> tabs -> normalize -> inline -> wrap -> SGR styling -> padding -> height -> alignment -> borders -> margins -> max truncation
- Border struct with 10 predefined borders (Normal, Rounded, Block, Thick, Double, Hidden, ASCII, Markdown, OuterHalfBlock, InnerHalfBlock), gradient border support
- Layout: Position type, place/place_horizontal/place_vertical, join_horizontal/join_vertical, width/height measurement
- Static Table: Data trait, StringData, Table builder with style_func, border config, column resizer algorithm

### Success Criteria

- `cargo build -p rouge-style`
- Unit tests for: color parsing, style rendering, border rendering, layout join/place, table rendering

---

## Spec 3: Runtime (rouge-runtime)

### Prerequisites: Spec 1, Spec 2

### Requirements

- Model trait: init(&mut self)->Cmd, update(&mut self, Msg)->Cmd, view(&self)->View
- Msg enum: KeyPress/KeyRelease, Mouse*, Paste, WindowSize, Focus/Blur, ColorProfile, Quit/Interrupt/Suspend/Resume, Custom(Box<dyn Any>)
- Cmd type: Option<CmdInner> with Sync/Async variants, batch(), sequence(), quit(), tick(), every()
- View struct: content, cursor, colors, alt_screen, mouse_mode, etc.
- Program<M: Model>: builder pattern, run() async method
- Event loop: tokio::select! on msg channel, calls update+view, submits cmds
- 5 concurrent tasks: input reader (crossterm EventStream), signal handler (SIGINT/SIGTERM/SIGWINCH), command handler (spawn tasks), render ticker (FPS interval), event loop
- FullRenderer: crossterm command queue, alt screen, cursor management, syncd output (mode 2026)
- NilRenderer for testing
- TerminalGuard (Drop-based raw mode cleanup)
- Input translation: crossterm::Event -> Msg

### Success Criteria

- `cargo build -p rouge-runtime`
- Counter example runs: displays count, j/k changes it, q quits, alt screen works

---

## Spec 4: Components (rouge-components)

### Prerequisites: Spec 3

### Requirements

All 14 components with new(), update(&mut self, msg), view(&self) -> String:

- **Key binding system**: Binding struct, BindingBuilder, matches(), KeyMap trait
- **Cursor**: Blink/Static/Hidden modes, configurable blink speed, focus management
- **Spinner**: 12 predefined animations (Line, Dot, MiniDot, Jump, Pulse, Points, Globe, Moon, Monkey, Meter, Hamburger, Ellipsis), TickMsg
- **Paginator**: Arabic + Dots modes, pagination math
- **Timer**: countdown with interval, Start/Stop/Toggle, TimeoutMsg
- **Stopwatch**: elapsed time, Start/Stop/Toggle/Reset
- **Progress**: fill/empty chars, color gradients, spring animation (rouge-harmonica), percentage display
- **Help**: short + full help views, takes &dyn KeyMap
- **Viewport**: scrolling, soft wrap, mouse wheel, gutter func, highlights, per-line styling
- **TextInput**: cursor, scrolling, echo modes (Normal/Password/None), suggestions, paste, validation
- **TextArea**: multiline, line numbers, word/character ops, viewport scrolling
- **Table**: columns, rows, selection, viewport scrolling, focus styles
- **List**: Item trait, ItemDelegate, fuzzy filtering, paginator + spinner + textinput integration
- **FilePicker**: directory navigation, type filtering, permissions display

Feature-gated in facade crate.

### Success Criteria

- `cargo build -p rouge-components`
- Spinner, textinput, list, table, viewport examples run interactively

---

## Spec 5: Glamour + Examples + Polish

### Prerequisites: Spec 4

### Requirements

- **rouge-glamour**: TermRenderer with pulldown-cmark parser, syntect syntax highlighting, serde StyleConfig, 4 embedded themes (dracula, tokyo-night, dark, light), render() convenience function
- **rouge facade crate**: re-exports all crates, feature flags per component
- **Examples** (ported from bubbletea canonical examples):
  - counter: keyboard-driven counter with styled view
  - todo: list with filtering, add/remove items
  - textinput: multi-field form with validation
  - viewport: scrollable markdown content (uses glamour)
  - spinner: multiple spinner styles demo

### Success Criteria

- `cargo build --workspace` compiles
- `cargo clippy --workspace` no errors
- All 5 examples run interactively
- Glamour renders markdown with syntax highlighting in viewport example
