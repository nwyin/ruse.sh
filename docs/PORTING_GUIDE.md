# Porting charm.sh to Another Language

This guide documents the architecture, design decisions, and feature inventory of the [charm.sh](https://charm.sh) Go TUI ecosystem, written for anyone building their own port. It was created during the development of [ruse.sh](https://github.com/nwyin/ruse.sh), a Rust port.

The charm.sh ecosystem is ~118,755 lines of Go across 7 libraries. A faithful port in a systems language with a good terminal library (like crossterm for Rust) lands around 20K lines — the difference is explained by Go's terminal infrastructure layer being replaced by the host language's ecosystem.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Library-by-Library Breakdown](#2-library-by-library-breakdown)
3. [Key Design Decisions](#3-key-design-decisions)
4. [Feature Inventory](#4-feature-inventory)
5. [Implementation Order](#5-implementation-order)

---

## 1. Architecture Overview

### What You're Porting

| Go Library | Lines | Purpose | What It Becomes |
|---|---|---|---|
| `bubbletea` | 13,839 | Elm architecture TUI runtime | Core runtime crate |
| `x/ansi` | 68,458 | ANSI sequences, cell buffer, VT emulator, terminal infra | ANSI utilities crate (much of this is replaced by your terminal library) |
| `lipgloss` | 15,442 | Styling, layout, borders, tables, tree rendering | Styling crate |
| `bubbles` | 13,792 | 14 reusable UI components | Components crate |
| `glamour` | 4,799 | Markdown rendering to ANSI | Markdown crate |
| `colorprofile` | 1,441 | Terminal color capability detection | Color profile crate |
| `harmonica` | 984 | Spring/projectile physics animation | Animation crate |

### The Elm Architecture

The core pattern is Model/Update/View:

```
Model  — your application state (a struct)
Update — fn(model, message) -> (model, command)
View   — fn(model) -> string (rendered output)
```

The runtime:
1. Calls `View()` to get initial content
2. Reads terminal events (keyboard, mouse, resize)
3. Translates events to messages
4. Calls `Update()` with each message
5. Calls `View()` again to get new content
6. Diffs old vs new content, writes minimal ANSI to terminal
7. Repeats until quit

### Five Fundamental Design Tensions

| Tension | Go Approach | Notes for Porters |
|---|---|---|
| **Msg polymorphism** | `interface{}` + type switch | Use your language's sum type / enum / tagged union. Add a `Custom(Box<dyn Any>)` escape hatch for user-defined messages. |
| **Model ownership** | `Update() -> (Model, Cmd)` returns new model | In languages with mutability, `update(&mut self) -> Cmd` is simpler. Go returns a new model because interfaces require it. |
| **Cmd representation** | `func() Msg` (closure returning message) | Support both sync and async variants. Commands are how side effects happen. |
| **Concurrency** | Goroutines + channels | Use your async runtime. The event loop uses select/poll on multiple channels. |
| **Style rendering** | Styled strings with embedded ANSI | Components produce string output. Style is applied by wrapping strings in ANSI escape sequences. |

---

## 2. Library-by-Library Breakdown

### bubbletea (Runtime)

**Core types:**
- `Model` trait/interface: `init() -> Cmd`, `update(Msg) -> Cmd`, `view() -> String`
- `Msg` enum: KeyPress, MouseClick, WindowSize, Focus, Blur, Quit, Paste, Custom(...)
- `Cmd`: `Option<fn() -> Msg>` with sync and async variants
- `Program`: the runtime that owns the model and runs the event loop

**Event loop architecture** (5 concurrent tasks):
1. **Input reader** — reads terminal events, translates to Msg, sends to channel
2. **Command executor** — receives Cmd, executes them, sends resulting Msg back
3. **Render ticker** — flushes output at target FPS (default 60)
4. **Signal handler** — SIGWINCH (resize), SIGINT, SIGTSTP
5. **Main loop** — select! on all channels, calls model.update(), triggers re-render

**Key features to implement:**
- `batch()` / `sequence()` — run multiple commands concurrently or sequentially
- `quit()` / `tick()` — built-in commands
- `Exec()` — release terminal, spawn subprocess, restore terminal
- Suspend/resume — SIGTSTP/SIGCONT handling
- `Send()` — inject messages from external code (thread-safe)
- `WithInput/WithOutput` — custom I/O for testing and SSH
- `WithFilter` — message middleware
- Panic recovery — catch panics, restore terminal, log to file

**Renderer:**
The diff-based renderer is the most complex piece. Go uses a cell buffer with:
- Double-buffering (current vs desired state)
- Hash-based line matching to detect moved lines
- Scroll optimization (use IL/DL instead of rewriting)
- Per-line cell diff to minimize ANSI output
- Cursor movement optimization (try absolute, relative, CR+relative, pick shortest)
- Synchronized output (mode 2026) wrapping

### lipgloss (Styling)

**Style struct** with ~42 properties tracked via bitflags:
- Text attributes: bold, italic, underline (5 styles), strikethrough, reverse, blink, faint
- Colors: foreground, background, underline color, margin background
- Dimensions: width, height, max_width, max_height
- Spacing: padding (4 sides), margin (4 sides)
- Alignment: horizontal (left/center/right as 0.0-1.0), vertical
- Borders: 10 presets, per-side toggle, per-side fg/bg colors
- Misc: tab_width, inline mode

**Render pipeline** (12 steps, order matters):
1. Transform function (if set)
2. Tab → spaces conversion
3. Strip carriage returns
4. Strip newlines (if inline mode)
5. Word wrap to target width
6. Apply SGR text styling
7. Add padding
8. Pad to target height
9. Horizontal alignment
10. Render borders
11. Add margins
12. Truncate to max_width/max_height

**Layout functions:** `join_horizontal`, `join_vertical`, `place`, `place_horizontal`, `place_vertical`

**Advanced features:**
- Tree rendering with 8 enumerator types (Default ├──/└──, Rounded, Bullet, Arabic, Roman, Alphabet, etc.)
- Layer/Compositor/Canvas for overlapping content with z-ordering
- `StyleRunes` / `StyleRanges` for applying different styles to character indices (fuzzy match highlighting)
- `Whitespace` system with character cycling
- `blend_1d` / `blend_2d` for color gradients (use CIELAB for perceptual uniformity)
- `LightDark` / `Complete` for terminal-adaptive colors
- Static table with headers, rows, column widths, style functions

### bubbles (Components)

14 components, each following the same pattern: struct with `update(msg) -> Cmd` and `view() -> String`.

**Cross-cutting pattern: KeyMap**
Every interactive component has a `KeyMap` struct with `Binding` entries. Each binding has keys (e.g., "up", "k"), help text, and enabled flag. Users can customize all keybindings. Components implement a `KeyMap` trait so the Help component can render their bindings.

| Component | Key Features |
|---|---|
| **TextInput** | Cursor, echo modes, suggestions with tab-complete, word navigation, horizontal scrolling, validation callback |
| **TextArea** | Multi-line editing, line numbers, soft wrapping with memoization (LRU cache), case transforms, transpose |
| **Viewport** | Vertical scrolling, left gutter function (line numbers), highlight system (search matches), per-line styling, horizontal scrolling |
| **List** | Items with ItemDelegate for custom rendering, fuzzy matching with match index tracking, filter states (Unfiltered/Filtering/Applied), status messages, pagination |
| **Table** | Column definitions, row selection, half-page navigation, header/cell/selected styles |
| **FilePicker** | Async directory reading, hidden file toggle, symlink detection, permission display, type filtering, navigation history stack |
| **Spinner** | 12 preset animations, customizable frames and speed |
| **Progress** | Fill/empty characters, percentage display, spring animation, multi-color gradient |
| **Help** | Short/full views, column layout, width-aware truncation with ellipsis |
| **Paginator** | Arabic ("1 of 5") and dots modes |
| **Timer** | Countdown with timeout |
| **Stopwatch** | Elapsed time tracking |
| **Cursor** | Blink/static/hidden modes |

### glamour (Markdown)

Uses a markdown parser (pulldown-cmark in Rust, goldmark in Go) + syntax highlighter (syntect/chroma).

**What to render:** headings (h1-h6), paragraphs, emphasis, strong, strikethrough, code blocks with syntax highlighting, inline code, block quotes, lists (ordered/unordered with nesting), links, images (alt text + URL), horizontal rules, tables, task checkboxes, definition lists, footnotes.

**Theme system:** JSON-serializable style config with per-element styling (colors, bold, italic, prefixes, suffixes, indentation).

### x/ansi (ANSI Utilities)

Much of this is replaced by your terminal library. What you still need:
- **SGR builder** — construct ANSI style sequences
- **ANSI stripping** — remove escape sequences from strings
- **String width** — visual width ignoring ANSI, using grapheme clusters + Unicode width
- **Text operations** — truncate, truncate_left, word wrap, pad (all ANSI-aware)
- **Cell buffer** — Cell struct, Buffer (2D grid), Screen (double-buffered diff renderer)
- **ANSI parser** — state machine for CSI/OSC/DCS/APC/SOS sequences
- **VT emulator** — for terminal recording/testing (P3 priority)
- **Kitty graphics** — inline terminal images (P3 priority)

### colorprofile (Detection)

Detect terminal color capability from environment:
- Check `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`
- Check `COLORTERM` (truecolor/24bit)
- Check `TERM` for known terminals (alacritty, kitty, wezterm, etc.)
- Check for tmux/screen (suppress COLORTERM, check via `tmux info` subprocess)
- 5 profiles: NoTty < Ascii < Ansi (16) < Ansi256 < TrueColor
- Color downsampling: convert RGB to 256-color (6x6x6 cube + grayscale ramp) or 16-color

### harmonica (Animation)

- **Spring** — damped harmonic oscillator with 3 branches (underdamped, critically damped, overdamped). Pre-compute coefficients, then `update(dt)` is cheap.
- **Projectile** — basic physics with position, velocity, acceleration, gravity.

---

## 3. Key Design Decisions

### Message typing
Framework messages (keyboard, mouse, resize) should be enum variants for exhaustive matching. User messages need an escape hatch — Go uses `interface{}`, Rust uses `Box<dyn Any>`. This lets components define their own internal messages without polluting the global enum.

### Rendering: strings vs cells
Charm uses styled strings (not a cell buffer) as the primary abstraction between components and the renderer. Components return `String` from `view()`, and the renderer parses ANSI to build the cell buffer for diffing. This is intentional — it makes components composable without coupling to a specific renderer.

### Style property tracking
Use bitflags to track which properties have been explicitly set vs left at default. This enables inheritance (`inherit()` copies unset properties from parent) and is critical for the render pipeline to know what to apply.

### Raw mode and newlines
In raw terminal mode, `\n` only moves the cursor down (LF) without returning to column 0. You must either:
- Replace `\n` with `\r\n` before writing, or
- Use a cell buffer that positions content explicitly

### Border rendering
Borders are rendered as separate characters around the content. Each border style defines 8 characters: top-left, top, top-right, right, bottom-right, bottom, bottom-left, left. The render pipeline adds borders after padding but before margins.

---

## 4. Feature Inventory

### Priority levels
- **P0** — Blocking for real applications (diff renderer, exec, keymaps, custom I/O)
- **P1** — Important for production (fuzzy filtering, suggestions, highlights, focused/blurred styles)
- **P2** — Nice to have (layer compositing, tree rendering, gradients, clipboard)
- **P3** — Specialized (VT emulator, Kitty graphics, Pony layout framework)

### What crossterm / your terminal library gives you for free
~35,000 lines of Go's x/ package is terminal infrastructure that your terminal library already provides:
- Cursor movement, screen operations, terminal mode control
- Input event reading and parsing (keyboard, mouse, paste, focus, resize)
- TTY detection, raw mode, alt screen, mouse capture
- Windows console API abstraction

### What you must build yourself
- Cell buffer with double-buffering and diff rendering (~1,500 lines)
- ANSI-aware string operations (width, truncate, wrap, pad) (~1,200 lines)
- Style system with render pipeline (~2,800 lines)
- Elm architecture runtime with event loop (~1,000 lines)
- 14 UI components (~3,000 lines)
- Markdown renderer (~1,500 lines)
- Color profile detection (~1,600 lines)
- Animation physics (~600 lines)

---

## 5. Implementation Order

### Phase 1: Foundation
1. ANSI utilities (SGR builder, strip, width, truncate, wrap, pad)
2. Color profile detection
3. Style system (struct, bitflags, render pipeline, borders)
4. Layout functions (join, place)
5. Animation (spring, projectile)
6. Basic runtime (Model trait, Msg enum, Cmd type, simple event loop with clear-and-rewrite)

### Phase 2: Components
7. Key bindings system
8. Simple components (spinner, cursor, paginator, timer, stopwatch)
9. Text input
10. Text area
11. Viewport
12. Progress bar
13. Help
14. List
15. Table
16. File picker

### Phase 3: Polish
17. Cell buffer + diff-based renderer (replaces clear-and-rewrite)
18. Synchronized output (mode 2026)
19. Panic recovery
20. Exec/subprocess with terminal release/restore
21. Suspend/resume (SIGTSTP/SIGCONT)
22. External program control (Send/Kill/Wait)
23. Custom I/O (WithInput/WithOutput for testing and SSH)
24. Message filtering middleware
25. Markdown renderer

### Phase 4: Advanced
26. ANSI parser state machine
27. Tree rendering with enumerators
28. Layer/Compositor/Canvas
29. StyleRunes/StyleRanges for match highlighting
30. Fuzzy matching in List
31. Suggestion system in TextInput
32. Viewport highlights and gutter
33. VT terminal emulator
34. Kitty graphics protocol

---

## License

This guide is MIT licensed. The charm.sh libraries it documents are also MIT licensed.
