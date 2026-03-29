# Rouge.sh — Rust Reimplementation of the Charm.sh TUI Ecosystem

A complete Rust port of the Charm.sh terminal UI stack: bubbletea (Elm architecture runtime), lipgloss (styling/layout), bubbles (components), glamour (markdown), harmonica (animations), colorprofile (terminal detection), and x/ansi (ANSI utilities).

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Workspace Structure](#2-workspace-structure)
3. [Core Runtime (rouge-runtime)](#3-core-runtime)
4. [ANSI Utilities (rouge-ansi)](#4-ansi-utilities)
5. [Color Profile Detection (rouge-colorprofile)](#5-color-profile-detection)
6. [Styling & Layout (rouge-style)](#6-styling--layout)
7. [Animation (rouge-harmonica)](#7-animation)
8. [Components (rouge-components)](#8-components)
9. [Markdown Rendering (rouge-glamour)](#9-markdown-rendering)
10. [Dependencies](#10-dependencies)
11. [Implementation Phases](#11-implementation-phases)
12. [Key Design Decisions](#12-key-design-decisions)

---

## 1. Architecture Overview

### What We're Porting

| Go Library | Purpose | Rust Crate |
|---|---|---|
| `bubbletea` | Elm architecture TUI runtime | `rouge-runtime` |
| `x/ansi` | ANSI sequence parsing/generation, string measurement | `rouge-ansi` |
| `colorprofile` | Terminal color capability detection | `rouge-colorprofile` |
| `lipgloss` | Styling, layout, borders, tables | `rouge-style` |
| `harmonica` | Spring/projectile physics animation | `rouge-harmonica` |
| `bubbles` | 14 reusable UI components | `rouge-components` |
| `glamour` | Markdown rendering to ANSI | `rouge-glamour` |

### The Five Fundamental Design Tensions

| Tension | Go Approach | Rust Approach | Rationale |
|---|---|---|---|
| **Msg polymorphism** | `interface{}` + type switch | Enum with `Custom(Box<dyn Any>)` variant | Framework messages get exhaustive matching; user messages use Any for open extension |
| **Model ownership** | `Update() -> (Model, Cmd)` returns new interface | `update(&mut self) -> Cmd` mutates in place | `&mut self` avoids ownership dance; Go returns new Model only because interfaces require it |
| **Cmd representation** | `func() Msg` | `Box<dyn FnOnce() -> Msg + Send>` + async variant | Supports both sync and async side effects |
| **Concurrency** | Goroutines + channels | tokio tasks + mpsc channels | tokio is ecosystem standard; `select!` maps to Go `select` |
| **Style/rendering** | Styled strings with embedded ANSI | Same: styled strings with embedded ANSI | Components produce `String` output, framework-agnostic |

---

## 2. Workspace Structure

```
rouge.sh/
  Cargo.toml                    # workspace root
  crates/
    rouge-ansi/                 # ANSI parsing, SGR generation, string width, wrap, truncate, strip
    rouge-colorprofile/         # Terminal color profile detection + conversion
    rouge-harmonica/            # Spring + projectile physics animation
    rouge-style/                # Style builder, borders, layout, tables (the lipgloss port)
    rouge-runtime/              # Elm architecture runtime (the bubbletea port)
    rouge-components/           # All 14 UI components (feature-gated)
    rouge-glamour/              # Markdown rendering
    rouge/                      # Facade crate re-exporting everything
```

### Dependency Graph

```
rouge-ansi          (standalone)
rouge-colorprofile  -> rouge-ansi
rouge-harmonica     (standalone)
rouge-style         -> rouge-ansi, rouge-colorprofile
rouge-runtime       -> rouge-ansi, rouge-colorprofile, rouge-style
rouge-components    -> rouge-runtime, rouge-style, rouge-harmonica
rouge-glamour       -> rouge-style, rouge-ansi
rouge               -> all of the above (feature-gated re-exports)
```

---

## 3. Core Runtime (rouge-runtime)

Port of: `charm.sh/bubbletea`

### 3.1 Model Trait

```rust
pub trait Model: Send + 'static {
    /// Called once at startup. Return an optional command for initial I/O.
    fn init(&mut self) -> Cmd { None }

    /// Process a message. Modify self in place, optionally return a command.
    fn update(&mut self, msg: Msg) -> Cmd;

    /// Render the current state. Called after every update.
    fn view(&self) -> View;
}
```

Key difference from Go: `&mut self` instead of consuming and returning the model. Go's `Update(Msg) -> (Model, Cmd)` exists because Go interfaces require returning a possibly-different concrete type. Rust's generics eliminate this need.

### 3.2 Msg Type

```rust
#[non_exhaustive]
pub enum Msg {
    // Keyboard
    KeyPress(KeyEvent),
    KeyRelease(KeyEvent),

    // Mouse
    MouseClick(MouseEvent),
    MouseRelease(MouseEvent),
    MouseWheel(MouseEvent),
    MouseMotion(MouseEvent),

    // Clipboard/Paste
    Paste(String),
    PasteStart,
    PasteEnd,

    // Window/Terminal
    WindowSize { width: u16, height: u16 },
    Focus,
    Blur,
    ColorProfile(ColorProfile),

    // Lifecycle
    Quit,
    Interrupt,
    Suspend,
    Resume,

    // Terminal queries
    CursorPosition { x: u16, y: u16 },
    ForegroundColor(Color),
    BackgroundColor(Color),
    CursorColor(Color),

    // Internal
    Batch(Vec<Cmd>),
    Sequence(Vec<Cmd>),
    ClearScreen,
    PrintLine(String),

    /// User-defined messages. Use `Msg::custom(val)` to create,
    /// `msg.downcast_ref::<T>()` to extract.
    Custom(Box<dyn Any + Send + 'static>),
}

impl Msg {
    pub fn custom<T: Send + 'static>(val: T) -> Self {
        Msg::Custom(Box::new(val))
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        match self {
            Msg::Custom(any) => any.downcast_ref::<T>(),
            _ => None,
        }
    }

    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        match self {
            Msg::Custom(any) => any.downcast::<T>().map(|b| *b).map_err(Msg::Custom),
            other => Err(other),
        }
    }
}
```

`#[non_exhaustive]` allows adding framework variants in minor versions without breaking downstream `match` statements.

### 3.3 Cmd Type

```rust
pub type Cmd = Option<CmdInner>;

pub enum CmdInner {
    Sync(Box<dyn FnOnce() -> Msg + Send + 'static>),
    Async(Pin<Box<dyn Future<Output = Msg> + Send + 'static>>),
}

// Constructors
pub fn cmd<F: FnOnce() -> Msg + Send + 'static>(f: F) -> Cmd { Some(CmdInner::Sync(Box::new(f))) }
pub fn cmd_async<F: Future<Output = Msg> + Send + 'static>(fut: F) -> Cmd { Some(CmdInner::Async(Box::pin(fut))) }

pub fn batch(cmds: Vec<Cmd>) -> Cmd { /* filter nils, optimize 0/1 cases */ }
pub fn sequence(cmds: Vec<Cmd>) -> Cmd { /* filter nils, optimize 0/1 cases */ }
pub fn quit() -> Cmd { cmd(|| Msg::Quit) }
pub fn tick(duration: Duration, f: impl FnOnce(Instant) -> Msg + Send + 'static) -> Cmd { /* sleep then send */ }
pub fn every(duration: Duration, f: impl FnOnce(Instant) -> Msg + Send + 'static) -> Cmd { /* clock-aligned tick */ }
```

### 3.4 View Type

```rust
pub struct View {
    pub content: String,
    pub cursor: Option<Cursor>,
    pub background_color: Option<Color>,
    pub foreground_color: Option<Color>,
    pub window_title: Option<String>,
    pub alt_screen: bool,
    pub report_focus: bool,
    pub mouse_mode: MouseMode,
    pub keyboard_enhancements: KeyboardEnhancements,
    pub disable_bracketed_paste: bool,
}

pub struct Cursor {
    pub position: Position,
    pub color: Option<Color>,
    pub shape: CursorShape,  // Block, Underline, Bar
    pub blink: bool,
}

pub enum MouseMode { None, CellMotion, AllMotion }
```

### 3.5 Program Runtime

```rust
pub struct Program<M: Model> {
    model: M,
    options: ProgramOptions,
}

impl<M: Model> Program<M> {
    pub fn new(model: M) -> Self;

    // Builder options
    pub fn with_fps(self, fps: u32) -> Self;
    pub fn with_alt_screen(self) -> Self;
    pub fn with_input(self, input: impl AsyncRead + Unpin + Send + 'static) -> Self;
    pub fn with_output(self, output: impl Write + Send + 'static) -> Self;
    pub fn with_color_profile(self, profile: ColorProfile) -> Self;
    pub fn with_filter(self, f: impl Fn(&M, Msg) -> Option<Msg> + Send + 'static) -> Self;
    pub fn with_window_size(self, w: u16, h: u16) -> Self;  // for testing
    pub fn without_renderer(self) -> Self;
    pub fn without_signal_handler(self) -> Self;

    /// Run the program. Blocks until exit.
    pub async fn run(self) -> Result<M, ProgramError>;

    /// Inject a message from outside.
    pub fn send(&self, msg: Msg);
}
```

`Program` is generic over `M: Model` -- full monomorphization, zero dynamic dispatch on the hot path.

### 3.6 Runtime Architecture (Internal)

The runtime spawns 5 concurrent tasks, all coordinated via an `mpsc` message channel and a `CancellationToken`:

```
┌─────────────────────────────────────────────────────┐
│                    Program::run()                    │
│                                                     │
│  1. Setup terminal (raw mode, alt screen)           │
│  2. Detect color profile, window size               │
│  3. Call model.init()                               │
│  4. Spawn tasks:                                    │
│     ┌──────────────┐  ┌────────────────┐            │
│     │ Input Reader │  │ Signal Handler │            │
│     │ (crossterm)  │  │ (tokio signal) │            │
│     └──────┬───────┘  └───────┬────────┘            │
│            │ Msg              │ Msg                  │
│            ▼                  ▼                      │
│     ┌──────────────────────────────┐                │
│     │      Message Queue (mpsc)    │◄── Cmd results │
│     └──────────────┬───────────────┘                │
│                    │                                 │
│                    ▼                                 │
│     ┌──────────────────────────────┐                │
│     │        Event Loop            │                │
│     │  1. Apply filter             │                │
│     │  2. Handle internal msgs     │                │
│     │  3. model.update(msg)        │                │
│     │  4. Submit returned Cmd      │                │
│     │  5. renderer.render(view)    │                │
│     └──────────────────────────────┘                │
│                                                     │
│     ┌──────────────┐  ┌────────────────┐            │
│     │ Cmd Handler  │  │ Render Ticker  │            │
│     │ (spawn tasks)│  │ (FPS interval) │            │
│     └──────────────┘  └────────────────┘            │
│                                                     │
│  5. On exit: cancel all, flush, restore terminal    │
└─────────────────────────────────────────────────────┘
```

**Input Reader**: Uses `crossterm::event::EventStream` for async event reading. Translates crossterm events to `Msg` variants.

**Signal Handler**: Uses `tokio::signal::unix::signal` for SIGINT, SIGTERM, SIGWINCH, SIGTSTP.

**Command Handler**: Reads from command channel, spawns each command as a tokio task (async) or `spawn_blocking` (sync). Results sent back to message queue.

**Render Ticker**: `tokio::time::interval` at configured FPS (default 60, max 120). Calls `renderer.flush()` on each tick.

**Event Loop**: Main `tokio::select!` loop. Sequential message processing, calls `model.update()`, triggers render.

**Terminal Cleanup**: Via a `Drop` guard that restores raw mode, cursor visibility, alt screen, and mouse mode even on panic.

### 3.7 Renderer (Internal)

```rust
trait Renderer: Send {
    fn start(&mut self);
    fn close(&mut self) -> io::Result<()>;
    fn render(&mut self, view: View);
    fn flush(&mut self, closing: bool) -> io::Result<()>;
    fn resize(&mut self, width: u16, height: u16);
    fn clear_screen(&mut self);
    fn insert_above(&mut self, text: &str) -> io::Result<()>;
    fn set_color_profile(&mut self, profile: ColorProfile);
}
```

Two implementations:
- **FullRenderer**: Cell-level diffing, synchronized output (mode 2026), alt screen, cursor management, mouse mode switching, bracketed paste, focus reporting, Kitty keyboard protocol. Uses crossterm's command queue for batched output.
- **NilRenderer**: No-op for headless/testing mode.

### 3.8 Module Layout

```
rouge-runtime/src/
  lib.rs              # re-exports
  model.rs            # Model trait
  msg.rs              # Msg enum, KeyEvent, MouseEvent, etc.
  cmd.rs              # Cmd, CmdInner, batch(), sequence(), tick(), every()
  view.rs             # View, Cursor, MouseMode
  program.rs          # Program struct, builder, run()
  event_loop.rs       # Core event loop
  error.rs            # ProgramError
  input.rs            # crossterm event -> Msg translation
  signal.rs           # Signal handler task
  terminal.rs         # TerminalGuard (raw mode setup/teardown)
  renderer/
    mod.rs            # Renderer trait
    full.rs           # Full terminal renderer
    nil.rs            # No-op renderer
  commands/
    mod.rs            # Built-in command constructors
    timer.rs          # tick(), every()
    clipboard.rs      # set_clipboard(), read_clipboard()
    exec.rs           # exec_process()
  keys.rs             # KeyCode, Modifiers
  mouse.rs            # MouseButton
```

---

## 4. ANSI Utilities (rouge-ansi)

Port of: `charmbracelet/x/ansi`

### 4.1 SGR Style Builder

```rust
/// SGR (Select Graphic Rendition) sequence builder.
#[derive(Clone, Default)]
pub struct SgrStyle {
    params: Vec<String>,
}

impl SgrStyle {
    pub fn new() -> Self;
    pub fn bold(self) -> Self;
    pub fn italic(self) -> Self;
    pub fn underline(self) -> Self;
    pub fn strikethrough(self) -> Self;
    pub fn reverse(self) -> Self;
    pub fn blink(self) -> Self;
    pub fn faint(self) -> Self;
    pub fn foreground(self, c: Color) -> Self;    // SGR 30-37, 38;5;N, 38;2;R;G;B
    pub fn background(self, c: Color) -> Self;    // SGR 40-47, 48;5;N, 48;2;R;G;B
    pub fn underline_color(self, c: Color) -> Self; // SGR 58;...

    pub fn styled(&self, s: &str) -> String;      // wrap text with open/reset
    pub fn open(&self) -> String;                  // just the opening ESC[...m
    pub fn reset() -> &'static str;               // ESC[0m
}
```

### 4.2 String Operations (ANSI-aware)

```rust
pub fn string_width(s: &str) -> usize;                          // visual width ignoring ANSI
pub fn strip(s: &str) -> String;                                 // remove all ANSI sequences
pub fn truncate(s: &str, width: usize, tail: &str) -> String;   // truncate to width
pub fn truncate_left(s: &str, width: usize, prefix: &str) -> String;
pub fn cut(s: &str, start: usize, end: usize) -> String;        // visual index substring
pub fn wrap(s: &str, width: usize) -> String;                   // word wrap preserving ANSI
pub fn height(s: &str) -> usize;                                 // line count
```

### 4.3 ANSI Parser

State machine for parsing ANSI escape sequences from byte streams. Handles CSI, OSC, DCS sequences. Used by the color profile writer for color downsampling.

### 4.4 WrapWriter

Wraps an `io::Write`, tracking current SGR pen state. On newlines, resets style, writes newline, reapplies style -- ensuring style continuity across wrapped lines.

---

## 5. Color Profile Detection (rouge-colorprofile)

Port of: `charmbracelet/colorprofile`

### 5.1 Profile Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Profile {
    NoTty,      // not a terminal
    Ascii,      // no color support
    Ansi,       // 16 colors (4-bit)
    Ansi256,    // 256 colors (8-bit)
    TrueColor,  // 16.7M colors (24-bit)
}
```

Derives `Ord` so the Go pattern `max(envp, tip, tmuxp)` maps to `envp.max(tip).max(tmuxp)`.

### 5.2 Detection Logic

```rust
impl Profile {
    pub fn detect(output: &impl IsTerminal, env: &[(String, String)]) -> Self;
}
```

Detection pipeline (from `env.go`):
1. Check TTY status
2. Check `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`
3. Check `COLORTERM` for `truecolor`/`24bit`
4. Check `TERM` for known terminals (alacritty, kitty, wezterm, ghostty, st, foot, contour, rio -> TrueColor)
5. Check `TERM` suffix `256color` -> Ansi256
6. Check tmux capabilities (`Tc`, `RGB`)
7. Platform-specific: Windows `WT_SESSION` -> TrueColor

### 5.3 Color Conversion

```rust
impl Profile {
    /// Downsample a color to match this profile's capability.
    pub fn convert(&self, color: Color) -> Color;
}
```

- TrueColor: passthrough
- Ansi256: RGB quantization to 6x6x6 cube + grayscale ramp
- Ansi: nearest-match to 16 basic colors
- Ascii/NoTty: strip color

Conversions cached in a `RwLock<HashMap<(Profile, Color), Color>>`.

### 5.4 Downsampling Writer

```rust
pub struct Writer<W: io::Write> {
    pub inner: W,
    pub profile: Profile,
}

impl<W: io::Write> io::Write for Writer<W> { /* parse SGR sequences, downsample colors */ }
```

Parses ANSI SGR sequences on-the-fly, replaces color parameters with profile-appropriate values while preserving text attributes (bold, italic, etc.).

---

## 6. Styling & Layout (rouge-style)

Port of: `charmbracelet/lipgloss`

### 6.1 Color Type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    NoColor,
    Basic(u8),            // 4-bit ANSI (0-15)
    Indexed(u8),          // 8-bit ANSI256 (0-255)
    Rgb { r: u8, g: u8, b: u8 },
}

impl Color {
    pub fn parse(s: &str) -> Self;      // "#rgb", "#rrggbb", "0"-"255"
    pub fn to_rgb(self) -> Option<(u8, u8, u8)>;
    pub fn is_dark(&self) -> bool;      // HSL luminance < 0.5
    pub fn darken(&self, percent: f64) -> Self;
    pub fn lighten(&self, percent: f64) -> Self;
    pub fn complementary(&self) -> Self; // rotate hue 180 degrees
}
```

### 6.2 Color Blending

```rust
/// Linear gradient in CIELAB color space.
pub fn blend_1d(steps: usize, stops: &[Color]) -> Vec<Color>;

/// 2D gradient with rotatable angle.
pub fn blend_2d(width: usize, height: usize, angle: f64, stops: &[Color]) -> Vec<Color>;
```

Uses the `palette` crate for CIELAB interpolation.

### 6.3 Style Type

~50 properties tracked via a `bitflags` u64 bitfield for "is-set" state, with dedicated typed fields for values.

```rust
bitflags! {
    pub struct StyleProps: u64 {
        const BOLD              = 1 << 0;
        const ITALIC            = 1 << 1;
        const STRIKETHROUGH     = 1 << 2;
        const REVERSE           = 1 << 3;
        const BLINK             = 1 << 4;
        const FAINT             = 1 << 5;
        const UNDERLINE_SPACES  = 1 << 6;
        const STRIKETHROUGH_SPACES = 1 << 7;
        const COLOR_WHITESPACE  = 1 << 8;
        const UNDERLINE         = 1 << 9;
        const FOREGROUND        = 1 << 10;
        const BACKGROUND        = 1 << 11;
        const WIDTH             = 1 << 13;
        const HEIGHT            = 1 << 14;
        const ALIGN_H           = 1 << 15;
        const ALIGN_V           = 1 << 16;
        const PADDING_TOP       = 1 << 17;
        // ... padding (4), margin (6), border (14), constraints, transform, link
        // Total: ~50 bits, fits in u64
    }
}

#[derive(Clone)]
pub struct Style {
    props: StyleProps,          // which properties are set
    attrs: StyleProps,          // bool values for text attributes
    fg_color: Color,
    bg_color: Color,
    ul_color: Color,
    underline: UnderlineStyle,  // None, Single, Double, Curly, Dotted, Dashed
    width: u16,
    height: u16,
    align_horizontal: Position,
    align_vertical: Position,
    padding_top: u16,
    padding_right: u16,
    padding_bottom: u16,
    padding_left: u16,
    padding_char: char,
    margin_top: u16,
    margin_right: u16,
    margin_bottom: u16,
    margin_left: u16,
    margin_bg_color: Color,
    margin_char: char,
    border_style: Border,
    border_top_fg: Color,      // ... per-side fg/bg colors
    border_blend_fg: Vec<Color>,
    border_fg_blend_offset: i32,
    max_width: u16,
    max_height: u16,
    tab_width: i8,             // -1 = no conversion, 0 = strip, N = spaces
    transform: Option<Arc<dyn Fn(&str) -> String + Send + Sync>>,
    link: String,
    link_params: String,
}
```

### 6.4 Builder Pattern

All setters consume `self` and return `Self` (move semantics, avoids unnecessary clones):

```rust
impl Style {
    pub fn new() -> Self;

    // Text attributes
    pub fn bold(self, v: bool) -> Self;
    pub fn italic(self, v: bool) -> Self;
    pub fn underline(self, style: UnderlineStyle) -> Self;
    pub fn strikethrough(self, v: bool) -> Self;
    pub fn reverse(self, v: bool) -> Self;
    pub fn blink(self, v: bool) -> Self;
    pub fn faint(self, v: bool) -> Self;

    // Colors
    pub fn foreground(self, c: Color) -> Self;
    pub fn background(self, c: Color) -> Self;

    // Dimensions
    pub fn width(self, w: u16) -> Self;
    pub fn height(self, h: u16) -> Self;
    pub fn max_width(self, w: u16) -> Self;
    pub fn max_height(self, h: u16) -> Self;

    // CSS-like shorthand: 1-4 values
    pub fn padding(self, values: &[u16]) -> Self;
    pub fn margin(self, values: &[u16]) -> Self;

    // Border
    pub fn border(self, b: Border, sides: &[bool]) -> Self;
    pub fn border_foreground(self, colors: &[Color]) -> Self;
    pub fn border_background(self, colors: &[Color]) -> Self;

    // Alignment
    pub fn align(self, h: Position, v: Position) -> Self;

    // Advanced
    pub fn transform(self, f: impl Fn(&str) -> String + Send + Sync + 'static) -> Self;
    pub fn link(self, url: &str) -> Self;
    pub fn inline(self, v: bool) -> Self;
    pub fn tab_width(self, w: i8) -> Self;

    // Inherit unset properties from another style (margins/padding NOT inherited)
    pub fn inherit(self, other: &Style) -> Self;

    // Unset any property
    pub fn unset_bold(self) -> Self;
    // ... one per property

    // Render
    pub fn render(&self, strs: &[&str]) -> String;
}
```

### 6.5 Rendering Pipeline

The `render()` method follows this exact order (from `lipgloss/style.go`):

```
Input string(s)
  |
  v
1. Transform (optional user function)
2. Convert tabs (tab_width)
3. Normalize line endings (\r\n -> \n)
4. Inline mode? Strip newlines
5. Word wrap (if width > 0 and not inline)
6. Apply text styling (SGR sequences for colors, bold, italic, etc.)
   - Space-specific styling (underline/strikethrough spaces separately)
   - Hyperlinks (OSC 8)
7. Apply padding (left/right with spaces, top/bottom with newlines)
8. Apply height (vertical alignment)
9. Apply horizontal alignment + width
10. Apply borders (4 edges + corners, with optional gradient colors)
11. Apply margins (4 sides, optional background color)
12. Apply max_width truncation
13. Apply max_height truncation
  |
  v
Output: ANSI-styled string
```

### 6.6 Border System

```rust
#[derive(Debug, Clone, Default)]
pub struct Border {
    pub top: Cow<'static, str>,
    pub bottom: Cow<'static, str>,
    pub left: Cow<'static, str>,
    pub right: Cow<'static, str>,
    pub top_left: Cow<'static, str>,
    pub top_right: Cow<'static, str>,
    pub bottom_left: Cow<'static, str>,
    pub bottom_right: Cow<'static, str>,
    pub middle_left: Cow<'static, str>,
    pub middle_right: Cow<'static, str>,
    pub middle: Cow<'static, str>,
    pub middle_top: Cow<'static, str>,
    pub middle_bottom: Cow<'static, str>,
}
```

Predefined borders as `const`:
- `NORMAL_BORDER` (single line: `┌─┐│└─┘`)
- `ROUNDED_BORDER` (rounded corners: `╭─╮│╰─╯`)
- `BLOCK_BORDER` (solid: `█`)
- `THICK_BORDER` (heavy: `┏━┓┃┗━┛`)
- `DOUBLE_BORDER` (`╔═╗║╚═╝`)
- `HIDDEN_BORDER` (spaces preserving layout)
- `ASCII_BORDER` (`+-+|+-+`)
- `MARKDOWN_BORDER` (`|-|`)
- `OUTER_HALF_BLOCK_BORDER`, `INNER_HALF_BLOCK_BORDER`

Border rendering supports per-character gradient coloring along the perimeter using `blend_1d`.

### 6.7 Layout Functions

```rust
/// Position along an axis, clamped [0.0, 1.0].
#[derive(Clone, Copy, Default)]
pub struct Position(f64);

impl Position {
    pub const TOP: Self = Self(0.0);
    pub const BOTTOM: Self = Self(1.0);
    pub const LEFT: Self = Self(0.0);
    pub const RIGHT: Self = Self(1.0);
    pub const CENTER: Self = Self(0.5);
}

pub fn place(width: usize, height: usize, h: Position, v: Position, s: &str) -> String;
pub fn place_horizontal(width: usize, pos: Position, s: &str) -> String;
pub fn place_vertical(height: usize, pos: Position, s: &str) -> String;
pub fn join_horizontal(pos: Position, strs: &[&str]) -> String;
pub fn join_vertical(pos: Position, strs: &[&str]) -> String;

pub fn width(s: &str) -> usize;   // visual width (max line width)
pub fn height(s: &str) -> usize;  // line count
```

### 6.8 Table Module

```rust
pub const HEADER_ROW: i32 = -1;

pub trait Data {
    fn at(&self, row: usize, col: usize) -> &str;
    fn rows(&self) -> usize;
    fn columns(&self) -> usize;
}

pub struct Table {
    base_style: Style,
    style_func: Box<dyn Fn(i32, usize) -> Style>,  // (row, col) -> Style
    border: Border,
    border_top: bool,
    border_bottom: bool,
    border_left: bool,
    border_right: bool,
    border_header: bool,
    border_column: bool,
    border_row: bool,
    border_style: Style,
    headers: Vec<String>,
    data: Box<dyn Data>,
    width: usize,
    height: usize,
    wrap: bool,
    // ... computed widths, heights, viewport offset
}

impl Table {
    pub fn new() -> Self;
    pub fn headers(self, h: Vec<String>) -> Self;
    pub fn row(self, r: Vec<String>) -> Self;
    pub fn rows(self, rs: Vec<Vec<String>>) -> Self;
    pub fn border(self, b: Border) -> Self;
    pub fn width(self, w: usize) -> Self;
    pub fn height(self, h: usize) -> Self;
    pub fn style_func(self, f: impl Fn(i32, usize) -> Style + 'static) -> Self;
    pub fn render(&mut self) -> String;
}
```

The resizing algorithm (from `lipgloss/table/resizing.go`) handles column width optimization: calculates min/max/median widths, expands or shrinks columns to fit target width.

---

## 7. Animation (rouge-harmonica)

Port of: `charmbracelet/harmonica`

### 7.1 Spring Animation

Damped harmonic oscillator with pre-computed coefficients for efficient per-frame updates.

```rust
pub fn fps(n: u32) -> f64 { 1.0 / n as f64 }

pub struct Spring {
    pos_pos_coef: f64,
    pos_vel_coef: f64,
    vel_pos_coef: f64,
    vel_vel_coef: f64,
}

impl Spring {
    /// Create with time delta, angular frequency, and damping ratio.
    /// - damping < 1.0: under-damped (bouncy)
    /// - damping = 1.0: critically damped (fastest, no overshoot)
    /// - damping > 1.0: over-damped (slow convergence)
    pub fn new(delta_time: f64, angular_frequency: f64, damping_ratio: f64) -> Self;

    /// Returns (new_position, new_velocity).
    pub fn update(&self, pos: f64, vel: f64, target: f64) -> (f64, f64);
}
```

### 7.2 Projectile

```rust
pub struct Point { pub x: f64, pub y: f64, pub z: f64 }
pub type Vector = Point;

pub const GRAVITY: Vector = Vector { x: 0.0, y: -9.81, z: 0.0 };
pub const TERMINAL_GRAVITY: Vector = Vector { x: 0.0, y: 9.81, z: 0.0 };

pub struct Projectile { pos: Point, vel: Vector, acc: Vector, dt: f64 }

impl Projectile {
    pub fn new(dt: f64, pos: Point, vel: Vector, acc: Vector) -> Self;
    pub fn update(&mut self) -> Point;
}
```

Self-contained math, zero dependencies, easy to port and test.

---

## 8. Components (rouge-components)

Port of: `charmbracelet/bubbles` (14 components)

### 8.1 Component Architecture

Components are concrete structs with conventional methods -- no shared `Component` trait needed. This mirrors Go where bubbles components do NOT implement `tea.Model`. Each component has:

- `new()` constructor
- `update(&mut self, msg: &dyn Any) -> Option<Command>` -- handles messages via `Any` downcasting
- `view(&self) -> String` -- renders to styled string
- `Focus()/Blur()` for focusable components
- `KeyMap` struct for customizable keybindings
- Own message types (e.g., `spinner::TickMsg`, `cursor::BlinkMsg`)

Components produce `String` output with embedded ANSI codes, making them framework-agnostic.

### 8.2 Key Binding System (Foundation)

```rust
pub struct Binding {
    keys: Vec<String>,
    help: Option<Help>,
    enabled: bool,
}

pub struct Help { pub key: String, pub desc: String }

impl Binding {
    pub fn new() -> BindingBuilder;
    pub fn keys(&self) -> &[String];
    pub fn enabled(&self) -> bool;
    pub fn set_enabled(&mut self, v: bool);
    pub fn unbind(&mut self);
}

pub fn matches(key_str: &str, bindings: &[&Binding]) -> bool;

pub trait KeyMap {
    fn short_help(&self) -> Vec<&Binding>;
    fn full_help(&self) -> Vec<Vec<&Binding>>;
}
```

### 8.3 Component Catalog

#### TextInput — Single-line text input
- Echo modes: Normal, Password, None
- Cursor with scrolling viewport
- Autocomplete suggestions
- Clipboard paste support
- Validation function
- KeyMap: character/word navigation, deletion, line start/end, paste, suggestions

#### TextArea — Multi-line text editor
- Line wrapping, line numbers, prompts
- Word/character operations (capitalize, transpose, case conversion)
- Configurable max height/width
- Viewport-based scrolling
- KeyMap: extends TextInput with line navigation, page up/down, case transforms

#### List — Scrollable filterable list
- `Item` trait: `fn filter_value(&self) -> &str`
- `ItemDelegate` trait for custom rendering
- Fuzzy filtering (using `sublime_fuzzy` or `fuzzy-matcher`)
- Paginator integration for page navigation
- Spinner for loading state
- Status messages with timeout
- Infinite scrolling option
- KeyMap: up/down, page navigation, filter, help toggle

#### Table — Data table
- Column definitions with widths
- Row selection/highlighting
- Viewport-based scrolling
- Focus states with different styles
- Help integration
- KeyMap: line/page/half-page navigation, goto top/bottom

#### Viewport — Scrollable content viewer
- Soft wrapping, fill height
- Mouse wheel support (configurable delta)
- Left gutter function (for line numbers, etc.)
- Highlight ranges with next/prev navigation
- Per-line styling via `StyleLineFunc`
- Horizontal scrolling
- KeyMap: line/page/half-page up/down, left/right, goto top/bottom

#### Spinner — Animated loading indicator
- 12 predefined animations: Line, Dot, MiniDot, Jump, Pulse, Points, Globe, Moon, Monkey, Meter, Hamburger, Ellipsis
- Custom frame sequences and FPS
- Tick-based animation via `TickMsg`

#### Progress — Progress bar
- Customizable fill/empty characters and colors
- Color gradients (single or blended via `blend_1d`)
- Spring-based smooth animation (uses `rouge-harmonica`)
- Percentage display (optional)
- Configurable width

#### Paginator — Page navigation
- Arabic mode: `"1/10"`
- Dots mode: `"*oooo"`
- Pagination math helpers: `slice_bounds()`, `items_on_page()`

#### Help — Key binding help display
- Short help: single-line with separator
- Full help: multi-column grouped layout
- Takes any `&dyn KeyMap` to render

#### Cursor — Blinking cursor
- Modes: Blink, Static, Hidden
- Configurable blink speed (~530ms default)
- Focus management
- Style for cursor and text under cursor

#### Timer — Countdown timer
- Configurable timeout and interval
- Start/Stop/Toggle
- TimeoutMsg when expired
- View renders as `"1m30s"`

#### Stopwatch — Elapsed time counter
- Configurable interval
- Start/Stop/Toggle/Reset

#### FilePicker — File system browser
- Directory navigation with breadcrumbs
- File type filtering by extension
- Permission/size display
- Hidden file toggle
- Dir-allowed/File-allowed selection modes

### 8.4 Component Message Routing

Parent applications route messages to child components via sequential `update()` calls:

```rust
fn update(&mut self, msg: Msg) -> Cmd {
    // Route to child components
    if let Some(cmd) = self.spinner.update(&msg) { return Some(cmd); }
    if let Some(cmd) = self.text_input.update(&msg) { return Some(cmd); }

    // Handle own messages
    match msg {
        Msg::KeyPress(key) => { /* ... */ }
        _ => None,
    }
}
```

### 8.5 Feature Flags

Each component is behind a feature flag in the facade crate:

```toml
[features]
default = ["full"]
full = ["textinput", "textarea", "list", "table", "viewport", "spinner",
        "progress", "paginator", "help", "cursor", "timer", "stopwatch", "filepicker"]
textinput = ["cursor", "key"]
textarea = ["cursor", "key", "viewport"]
list = ["paginator", "spinner", "textinput", "help", "key"]
table = ["viewport", "help", "key"]
progress = ["harmonica"]
# ... etc
```

---

## 9. Markdown Rendering (rouge-glamour)

Port of: `charmbracelet/glamour`

### 9.1 Architecture

```rust
pub struct TermRenderer {
    options: RenderOptions,
}

impl TermRenderer {
    pub fn builder() -> TermRendererBuilder;
    pub fn render(&self, markdown: &str) -> Result<String, Error>;
}

impl TermRendererBuilder {
    pub fn word_wrap(self, w: usize) -> Self;
    pub fn style(self, config: StyleConfig) -> Self;
    pub fn style_path(self, path: &str) -> Result<Self, Error>;
    pub fn syntax_theme(self, theme: &str) -> Self;
    pub fn base_url(self, url: &str) -> Self;
    pub fn build(self) -> TermRenderer;
}

/// Convenience
pub fn render(markdown: &str, style: &str) -> Result<String, Error>;
```

### 9.2 Parser & Renderer

- **Parser**: `pulldown-cmark` (CommonMark + GFM extensions: tables, strikethrough, tasklists)
- **Syntax highlighting**: `syntect` (replaces Go's Chroma)
- **Rendering**: Event-driven walk of pulldown-cmark events, emitting styled ANSI output using `rouge-style`

### 9.3 Style System

Deserializable via `serde` from JSON/TOML (same schema as Go's `glamour/ansi/style.go`):

```rust
#[derive(Deserialize)]
pub struct StyleConfig {
    pub document: StyleBlock,
    pub block_quote: StyleBlock,
    pub paragraph: StyleBlock,
    pub heading: StyleBlock,
    pub h1: StyleBlock, pub h2: StyleBlock, /* ... h6 */
    pub text: StylePrimitive,
    pub emph: StylePrimitive,
    pub strong: StylePrimitive,
    pub strikethrough: StylePrimitive,
    pub horizontal_rule: StylePrimitive,
    pub code: StyleBlock,
    pub code_block: StyleCodeBlock,
    pub link: StylePrimitive,
    pub link_text: StylePrimitive,
    pub image: StylePrimitive,
    pub table: StyleTable,
    pub task: StyleTask,
    // ...
}

#[derive(Deserialize)]
pub struct StylePrimitive {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub color: Option<String>,
    pub background_color: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    // ...
}
```

Builtin themes (dracula, tokyo-night, dark, light) embedded via `include_str!` from JSON files ported from Go.

---

## 10. Dependencies

### Core

| Crate | Purpose | Used By |
|---|---|---|
| `crossterm` | Terminal I/O, raw mode, events, cursor, screen | `rouge-runtime` |
| `tokio` (full) | Async runtime, tasks, channels, signals, timers | `rouge-runtime` |
| `tokio-util` | CancellationToken | `rouge-runtime` |
| `bitflags` | Efficient property bitfield | `rouge-style` |
| `unicode-width` | Character cell width measurement | `rouge-ansi` |
| `unicode-segmentation` | Grapheme cluster handling | `rouge-ansi` |
| `palette` | CIELAB color space for blending | `rouge-style` |
| `thiserror` | Error derives | all crates |

### Components

| Crate | Purpose | Used By |
|---|---|---|
| `sublime_fuzzy` | Fuzzy string matching | `rouge-components` (list) |

### Glamour

| Crate | Purpose | Used By |
|---|---|---|
| `pulldown-cmark` | Markdown parsing (CommonMark + GFM) | `rouge-glamour` |
| `syntect` | Syntax highlighting | `rouge-glamour` |
| `serde` + `serde_json` | Style config deserialization | `rouge-glamour` |

### What We Don't Need

- **`termion`**: crossterm covers everything and is cross-platform
- **`ansi_term`/`owo-colors`/`colored`**: rouge IS the styling library
- **`textwrap`**: needs ANSI-aware wrapping, built custom in `rouge-ansi`
- **`ratatui`**: rouge is a standalone framework, not built on ratatui

---

## 11. Implementation Phases

### Phase 1: Foundation (Weeks 1-2)

| Task | Crate | Notes |
|---|---|---|
| ANSI parser, SGR builder | `rouge-ansi` | Core dependency for everything |
| `string_width`, `strip`, `truncate`, `wrap` | `rouge-ansi` | ANSI-aware string ops |
| Spring + Projectile | `rouge-harmonica` | Self-contained math, easy to test |
| Key binding system | `rouge-components` | Foundation for all interactive components |

### Phase 2: Colors & Style (Weeks 3-4)

| Task | Crate | Notes |
|---|---|---|
| Color enum, parsing, helpers | `rouge-style` | |
| Profile detection | `rouge-colorprofile` | Environment + terminfo |
| Color conversion + Writer | `rouge-colorprofile` | Downsampling |
| Style struct, bitfield, all setters/getters | `rouge-style` | ~50 properties |
| Render pipeline (all 13 steps) | `rouge-style` | Core rendering |
| Border system + predefined borders | `rouge-style` | |
| Position, alignment functions | `rouge-style` | |

### Phase 3: Layout & Table (Weeks 5-6)

| Task | Crate | Notes |
|---|---|---|
| `join_horizontal`, `join_vertical` | `rouge-style` | |
| `place`, `place_horizontal`, `place_vertical` | `rouge-style` | |
| Whitespace renderer | `rouge-style` | |
| Color blending (Blend1D, Blend2D) | `rouge-style` | CIELAB via `palette` |
| Gradient border rendering | `rouge-style` | |
| Table data trait + builder | `rouge-style` | |
| Table resizer algorithm | `rouge-style` | Complex column optimization |

### Phase 4: Runtime (Weeks 7-9)

| Task | Crate | Notes |
|---|---|---|
| Msg, Cmd, View types | `rouge-runtime` | |
| Model trait | `rouge-runtime` | |
| Program struct + builder | `rouge-runtime` | |
| Terminal guard (raw mode setup/teardown) | `rouge-runtime` | |
| Input reader (crossterm events -> Msg) | `rouge-runtime` | |
| Signal handler | `rouge-runtime` | |
| Event loop | `rouge-runtime` | |
| Command handler (sync + async) | `rouge-runtime` | |
| Batch + Sequence execution | `rouge-runtime` | |
| Render ticker + FullRenderer | `rouge-runtime` | FPS-based, syncd output |
| Alt screen, cursor, mouse mode management | `rouge-runtime` | |
| Built-in commands: tick, every, clipboard, exec | `rouge-runtime` | |
| Counter example, working end-to-end | `rouge-runtime` | Milestone! |

### Phase 5: Simple Components (Weeks 10-11)

| Task | Crate | Notes |
|---|---|---|
| Spinner (12 predefined animations) | `rouge-components` | First component, validates tick pattern |
| Cursor (blink, static, hidden) | `rouge-components` | Needed by textinput/textarea |
| Paginator (Arabic + Dots) | `rouge-components` | Pure logic |
| Timer + Stopwatch | `rouge-components` | Tick-based |
| Progress bar (with spring animation) | `rouge-components` | Validates harmonica integration |
| Help (short + full modes) | `rouge-components` | Validates KeyMap trait |

### Phase 6: Complex Components (Weeks 12-14)

| Task | Crate | Notes |
|---|---|---|
| Viewport (scrolling, wrapping, gutter, highlights) | `rouge-components` | Foundation for table/textarea/list |
| TextInput (cursor, scrolling, suggestions, paste) | `rouge-components` | |
| TextArea (multiline, line numbers, word ops) | `rouge-components` | Most complex component |
| Table (columns, rows, selection, viewport scroll) | `rouge-components` | |
| List (filtering, delegate, pagination, spinner) | `rouge-components` | Most composed component |
| FilePicker (directory nav, filtering, permissions) | `rouge-components` | |

### Phase 7: Markdown & Polish (Weeks 15-16)

| Task | Crate | Notes |
|---|---|---|
| Markdown parser integration | `rouge-glamour` | pulldown-cmark |
| AST -> ANSI renderer | `rouge-glamour` | |
| Syntax highlighting | `rouge-glamour` | syntect |
| Theme system + builtin themes | `rouge-glamour` | serde JSON deserialization |
| Facade crate + feature flags | `rouge` | |
| Examples for each component | all | |
| Documentation | all | |

---

## 12. Key Design Decisions

### Why `&mut self` instead of consuming/returning Model

Go's `Update(Msg) -> (Model, Cmd)` returns a new interface value because Go interfaces require it. Rust's `&mut self` is more natural, avoids unnecessary moves, and is what Rust users expect. The model is never replaced with a different type at runtime.

### Why `Msg::Custom(Box<dyn Any>)` instead of generic `Msg<T>`

Making `Msg` generic (e.g., `Msg<AppMsg>`) infects the entire API: `Cmd<AppMsg>`, `Model<AppMsg>`, `Program<AppMsg>`. It creates a poor experience for composing sub-models with different message types. The `Any`-based approach costs one allocation and one downcast per custom message -- negligible for a TUI running at 60fps.

### Why tokio

Signal handling, timers, event streams, and I/O all benefit from structured async. Crossterm provides `EventStream` for async event reading. Tokio's `select!` maps directly to Go's `select` on channels. The alternative (OS threads + crossbeam) would work but requires manual thread management.

### Why crossterm

Cross-platform (Windows + Unix), actively maintained, already handles Kitty keyboard protocol, synchronized output, and modern terminal features. Replaces the functionality of both `ultraviolet` and `x/term` from Go.

### Why components are not trait objects

Go bubbles components don't implement `tea.Model`. They're concrete types with conventional methods. Rust should follow the same pattern -- concrete structs, no shared `Component` trait. Components are composed by embedding, not by polymorphism.

### Why `#[non_exhaustive]` on Msg

Allows adding new framework messages in minor versions without breaking downstream `match` statements. Users must have a catch-all `_ =>` arm, which is good practice.

### Why separate crates instead of one big crate

Matches Go's module boundaries. Users who only need styling don't pay for the runtime. Users who only need the runtime don't pay for markdown rendering. Feature-gated re-exports via the facade crate provide the convenience of a single dependency.

---

## Appendix: Example Usage

```rust
use rouge::prelude::*;

struct Counter {
    count: i32,
}

impl Model for Counter {
    fn update(&mut self, msg: Msg) -> Cmd {
        match msg {
            Msg::KeyPress(key) => match key.code {
                KeyCode::Char('q') => quit(),
                KeyCode::Char('j') | KeyCode::Down => { self.count += 1; None }
                KeyCode::Char('k') | KeyCode::Up => { self.count -= 1; None }
                _ => None,
            },
            _ => None,
        }
    }

    fn view(&self) -> View {
        let style = Style::new()
            .foreground(Color::parse("#ff6600"))
            .bold(true)
            .padding(&[1, 2])
            .border(ROUNDED_BORDER, &[true]);

        View::new(style.render(&[
            &format!("Count: {}", self.count),
            "\n\nPress j/k to change, q to quit",
        ]))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Counter { count: 0 };
    let final_model = Program::new(model)
        .with_alt_screen()
        .run()
        .await?;
    println!("Final count: {}", final_model.count);
    Ok(())
}
```

---

## Critical Reference Files

These Go source files are the primary references for the port:

| File | What to Port |
|---|---|
| `charm.sh/bubbletea/tea.go` | Program struct, Run(), event loop, shutdown |
| `charm.sh/bubbletea/commands.go` | Batch, Sequence, Tick, Every patterns |
| `charm.sh/bubbletea/cursed_renderer.go` | Full renderer with cell diffing |
| `charm.sh/bubbletea/input.go` | Event translation |
| `charm.sh/lipgloss/style.go` | Style struct, property bitfield, render pipeline |
| `charm.sh/lipgloss/set.go` | All setter methods |
| `charm.sh/lipgloss/borders.go` | Border struct, predefined borders, gradient rendering |
| `charm.sh/lipgloss/table/resizing.go` | Column width optimization algorithm |
| `charm.sh/colorprofile/env.go` | Profile detection from environment |
| `charm.sh/harmonica/spring.go` | Spring coefficient math |
| `charm.sh/bubbles/viewport/viewport.go` | Most composed-into component |
| `charm.sh/bubbles/key/key.go` | Key binding system (foundation for all components) |
| `charm.sh/glamour/ansi/renderer.go` | Markdown AST -> ANSI rendering |
| `charm.sh/glamour/ansi/style.go` | StyleConfig/StyleBlock/StylePrimitive schema |
