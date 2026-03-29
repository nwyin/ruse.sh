# Ruse.sh Gap Analysis — charm.sh vs ruse.sh

Full inventory of every feature in the Go charm.sh ecosystem that is missing or incomplete in the Rust ruse.sh port, with implementation-level detail.

**Go**: 118,755 lines across 7 libraries
**Rust**: 12,447 lines across 8 crates (~10.5% of Go)

**Skip list**: Pony layout framework (x/pony) — declarative XML-like markup DSL, ~8K lines. Skipped intentionally.

---

## Table of Contents

1. [x/ansi vs ruse-ansi](#1-xansi-vs-ruse-ansi)
2. [bubbletea vs ruse-runtime](#2-bubbletea-vs-ruse-runtime)
3. [lipgloss vs ruse-style](#3-lipgloss-vs-ruse-style)
4. [bubbles vs ruse-components](#4-bubbles-vs-ruse-components)
5. [glamour vs ruse-glamour](#5-glamour-vs-ruse-glamour)
6. [colorprofile vs ruse-colorprofile](#6-colorprofile-vs-ruse-colorprofile)
7. [harmonica vs ruse-harmonica](#7-harmonica-vs-ruse-harmonica)
8. [Implementation Plan](#8-implementation-plan)

---

## 1. x/ansi vs ruse-ansi

**Go**: 68,458 lines (25 packages) | **Rust**: 1,210 lines (6 modules) | **Coverage**: ~2%

Most of Go's x/ is replaced by `crossterm` in Rust, but significant gaps remain.

### 1.1 Cell Buffer System (`x/cellbuf/`) — NOT IMPLEMENTED

**Priority**: P0 | **Effort**: ~2,500 lines | **Depends on**: ruse-ansi SGR, ruse-colorprofile

The foundation for diff-based rendering. Go has ~5,089 lines across 13 files.

#### 1.1.1 Cell Type

```rust
struct Cell {
    rune: char,              // Primary character
    comb: SmallVec<[char; 2]>, // Combining marks (zero-width diacritics)
    width: u8,               // Display width (1-4 cells, 0 = wide-cell placeholder)
    style: CellStyle,        // SGR attributes + colors
    link: Link,              // OSC 8 hyperlink
}
```

- **Width=0** cells are placeholders for wide-cell continuations (CJK occupies indices N and N+1)
- `BlankCell` = `{rune: ' ', width: 1}` vs `EmptyCell` = `{rune: '\0', width: 0}`
- Methods: `new(char)`, `new_grapheme(str)`, `equal()`, `empty()`, `clear()`, `blank()`, `clone()`

#### 1.1.2 CellStyle Type

```rust
struct CellStyle {
    fg: Option<Color>,
    bg: Option<Color>,
    ul: Option<Color>,       // Underline color
    attrs: AttrMask,         // Bitflags: bold, faint, italic, blink, reverse, conceal, strikethrough
    ul_style: UnderlineStyle, // None, Single, Double, Curly, Dotted, Dashed
}
```

- **Critical method**: `diff_sequence(old_style) -> String` — generates minimal SGR to transform old→new (only writes changes)
- Used by renderer to minimize ANSI output

#### 1.1.3 Link Type

```rust
struct Link { url: String, params: String }
```

OSC 8: `\x1b]8;<params>;<url>\x1b\\`

#### 1.1.4 Buffer Type

```rust
type Line = Vec<Option<Cell>>;  // None = blank

struct Buffer {
    lines: Vec<Line>,
    width: usize,
    height: usize,
}
```

- Methods: `new(w, h)`, `cell(x, y)`, `set_cell(x, y, cell)`, `resize(w, h)`, `fill(cell)`, `fill_rect(cell, rect)`, `clear()`, `insert_line(y, n)`, `delete_line(y, n)`, `insert_cell(x, y, n)`, `delete_cell(x, y, n)`
- **Wide cell layout**: Setting a width-2 cell at x=5 sets `line[5] = cell, line[6] = EmptyCell`. Overwriting a wide cell blanks all its positions first.

#### 1.1.5 Screen (Double-Buffered Renderer)

```rust
struct Screen {
    w: Box<dyn Write>,
    buf: Vec<u8>,            // Output buffer for ANSI sequences
    curbuf: Buffer,          // What's currently displayed
    newbuf: Buffer,          // What we want to display
    cur: Cursor,             // Current pen state (position, style, link)
    saved: Cursor,           // Saved cursor (for alt-screen)
    touch: HashMap<usize, LineData>, // Changed lines since last render
    oldhash: Vec<u64>,       // Line hashes for diff
    newhash: Vec<u64>,
    oldnum: Vec<isize>,      // Old line index mapping (-1 = new/changed)
    opts: ScreenOptions,
    at_phantom: bool,        // Cursor at wrap-around position
}
```

#### 1.1.6 Hash-Based Line Diff Algorithm

The core rendering optimization. Flow:

1. **Hash each line**: `hash(line) = accumulate(h <<= 5; h += rune_value)`
2. **Build hash table**: Match old/new lines with unique hash pairs
3. **Grow hunks**: Expand matched regions forward/backward using cost function
4. **Scroll optimize**: Two passes (top→bottom for upward scrolls, bottom→top for downward), emit IL/DL/SU/SD commands
5. **Transform changed lines**: For each non-scrolled line, compute minimal cell diff via `transform_line()`
6. **Emit cells**: Use REP (repeat char) and ECH (erase char) optimizations

#### 1.1.7 Cursor Movement Optimization

`move_cursor(from, to)` tries 4 methods and picks shortest:

1. **Absolute CUP** — if distance > 8 cells
2. **Relative moves** — CUU/CUD/CUF/CUB with optional tab/backspace
3. **CR + relative** — carriage return then move
4. **Home + relative** — home then move

#### 1.1.8 Tab Stop Management

```rust
struct TabStops {
    stops: Vec<u32>,   // Bitset array
    interval: usize,   // Default 8
    width: usize,
}
```

Methods: `is_stop(col)`, `next(col)`, `prev(col)`, `set(col)`, `reset(col)`, `clear()`

### 1.2 ANSI Parser State Machine — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~800 lines | **Depends on**: nothing

Full CSI/OSC/DCS/APC/SOS sequence parser (~2,100 lines in Go).

```rust
struct Parser {
    state: ParserState,      // 15 states
    params: Vec<i32>,        // CSI parameter accumulator (cap 32)
    data: Vec<u8>,           // OSC/DCS data buffer (cap 64KB)
    cmd: i32,                // Packed command (final byte + prefix + intermediate)
}

enum ParserState {
    Ground, CsiEntry, CsiParam, CsiIntermediate,
    DcsEntry, DcsParam, DcsIntermediate, DcsString,
    Escape, EscapeIntermediate,
    OscString, SosString, PmString, ApcString,
    Utf8,
}
```

- 9 actions: Clear, Collect, Prefix, Dispatch, Execute, Start, Put, Param, Print
- Transition table generated from DEC VT spec
- Subparameter support via HasMore flag (bit 31)
- Key method: `advance(byte)` — state transition + action execution

### 1.3 Virtual Terminal Emulator (`x/vt/`) — NOT IMPLEMENTED

**Priority**: P3 | **Effort**: ~2,000 lines | **Depends on**: 1.1 (cellbuf), 1.2 (parser)

Full VT102/xterm terminal emulation with scrollback. Needed for terminal recording/testing.

```rust
struct Terminal {
    cursor: (u16, u16),
    saved_cursor: (u16, u16),
    cells: Buffer,           // From cellbuf
    scrollback: VecDeque<Line>,
    width: u16,
    height: u16,
    auto_wrap: bool,
    origin_mode: bool,
    insert_mode: bool,
    alt_screen: bool,
    charsets: [CharSet; 4],  // G0-G3
    margin_top: u16,
    margin_bottom: u16,
}
```

~50 command handlers: CUU, CUD, CUF, CUB, CHA, CUP, ED, EL, ECH, IL, DL, ICH, DCH, SU, SD, SGR, SM, RM, DECSC, DECRC, etc.

**Subcomponents**:
- VT state machine (cursor, modes, scroll regions): ~600 lines
- Command dispatch table: ~400 lines
- SGR attribute handling: ~200 lines
- Character set support: ~200 lines
- Scrollback management: ~300 lines
- Input/output integration: ~300 lines

### 1.4 Kitty Graphics Protocol — NOT IMPLEMENTED

**Priority**: P3 | **Effort**: ~400 lines | **Depends on**: nothing (standalone)

Inline terminal image encoding/decoding.

```rust
struct KittyOptions {
    action: KittyAction,     // t, T, q, p, d, f, a, c
    id: u32,
    format: ImageFormat,     // RGBA (32), RGB (24), PNG (100)
    compression: bool,       // zlib
    width: u32, height: u32,
    columns: u32, rows: u32,
}

struct ImageEncoder {
    compress: bool,
    format: ImageFormat,
}
```

- Encoding: RGBA pixels → base64 → 4KB chunks with `m=1`/`m=0` flags
- Wrapped in APC sequence: `\x1b_G<options>;<base64>\x1b\\`

### 1.5 OSC Sequence Handlers — PARTIAL

**Priority**: P2 | **Effort**: ~300 lines

Missing handlers:
- **OSC 0/1/2**: Window/icon title — `set_window_title(s)`, `set_icon_name(s)`
- **OSC 8**: Hyperlinks — `set_hyperlink(url, params)`, `reset_hyperlink()`
- **OSC 52**: Clipboard — `set_clipboard(data, selection)`, `request_clipboard(selection)`, base64 encode/decode
- **OSC 777**: Desktop notifications
- **OSC 1337**: iTerm2 file transfer protocol

### 1.6 Advanced SGR — PARTIAL

**Priority**: P2 | **Effort**: ~100 lines

Missing attributes:
- Overline: SGR 53 on, SGR 55 off
- Superscript: SGR 73
- Subscript: SGR 74
- Font selection: SGR 10-19 (10=default, 11-19 alternate)
- Normal intensity: SGR 22 (resets bold/faint)

### 1.7 Grapheme Cluster Handling — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~200 lines | **Depends on**: `unicode-segmentation` crate

Current code uses character-level iteration. Need grapheme-cluster-aware operations for emoji sequences (ZWJ), combining marks, etc.

```rust
// Current: uses unicode_width::UnicodeWidthStr::width(char)
// Needed: iterate grapheme clusters, measure each cluster's width
use unicode_segmentation::UnicodeSegmentation;

fn string_width_grapheme(s: &str) -> usize {
    strip_ansi(s).graphemes(true)
        .map(|cluster| UnicodeWidthStr::width(cluster))
        .sum()
}
```

Affects: `string_width()`, `truncate()`, `truncate_left()`, `wrap()`, `wordwrap()`, `pad()`

### 1.8 Terminal Queries — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~100 lines

Raw sequence generation for terminal interrogation:
- Device status report: `\x1b[6n` (cursor position)
- Device attributes: `\x1b[c`, `\x1b[>c`
- Mode reports: DECRPM
- Terminal parameters: DECREQTPARM

### 1.9 Color Palette Management — NOT IMPLEMENTED

**Priority**: P3 | **Effort**: ~150 lines

```rust
struct Palette {
    colors: [Color; 256],  // 16 basic + 216 cube + 24 grayscale
}
```

XTerm 256 default palette, custom palette loading, OSC 4 set/query.

---

## 2. bubbletea vs ruse-runtime

**Go**: 13,839 lines | **Rust**: 940 lines | **Coverage**: ~7%

### 2.1 Cell-Level Diffing Renderer — NOT IMPLEMENTED

**Priority**: P0 | **Effort**: ~800 lines | **Depends on**: 1.1 (cellbuf)

Current Rust renderer clears entire screen and rewrites on every frame. Go uses cellbuf for diff-based rendering.

#### Integration with cellbuf

```rust
struct CursedRenderer {
    screen: Screen,          // From cellbuf (double-buffered)
    last_view: Option<View>, // Previous frame
    syncd_updates: bool,     // Mode 2026
    width: u16,
    height: u16,
    profile: ColorProfile,
}
```

**Render cycle** (decoupled from update):
1. `model.view()` → View struct (string content + metadata)
2. Compare with `last_view` — skip if identical
3. Parse view content into cellbuf's `newbuf`
4. `screen.render()` → hash-based diff → minimal ANSI output
5. Flush on ticker (60 FPS default)

#### Synchronized Output (Mode 2026)

```rust
// Wrap all updates in synchronized output block
if self.syncd_updates {
    write!(stdout, "\x1b[?2026h")?;  // Begin sync
}
// ... all ANSI updates ...
if self.syncd_updates {
    write!(stdout, "\x1b[?2026l")?;  // End sync
}
```

Terminal buffers all updates and displays atomically — eliminates flicker.

### 2.2 Exec/ExecProcess — NOT IMPLEMENTED

**Priority**: P0 | **Effort**: ~200 lines | **Depends on**: terminal release/restore

Launch interactive subprocesses (vim, shell) with full terminal handoff.

```rust
pub fn exec<F>(cmd: &str, args: &[&str], on_finish: F) -> Cmd
where F: FnOnce(io::Result<ExitStatus>) -> Msg + Send + 'static
```

**Algorithm**:
1. `release_terminal()` — cancel input reader, stop renderer, disable raw mode, leave alt screen
2. Spawn subprocess with inherited stdin/stdout/stderr
3. Wait for exit
4. `restore_terminal()` — re-enable raw mode, enter alt screen, restart renderer, re-check window size
5. Return callback result as Msg

### 2.3 Suspend/Resume (SIGTSTP/SIGCONT) — NOT IMPLEMENTED

**Priority**: P0 | **Effort**: ~100 lines

```rust
#[cfg(unix)]
fn suspend_process() {
    // 1. Release terminal (disable raw, leave alt screen)
    // 2. Send SIGTSTP to process group: kill(0, SIGTSTP)
    // 3. Block until SIGCONT received
    // 4. Restore terminal
    // 5. Send Msg::Resume
}
```

Requires `nix` or `libc` crate for `kill(0, SIGTSTP)`.

### 2.4 Send() — External Message Injection — PARTIAL

**Priority**: P0 | **Effort**: ~50 lines

Current: internal `mpsc::unbounded_channel` only. Need public API.

```rust
pub struct ProgramHandle {
    msg_tx: mpsc::UnboundedSender<Msg>,
    cancel: CancellationToken,
}

impl ProgramHandle {
    pub fn send(&self, msg: Msg) -> Result<(), SendError> {
        if self.cancel.is_cancelled() { return Err(SendError::Closed); }
        self.msg_tx.send(msg).map_err(|_| SendError::Failed)
    }
    pub fn quit(&self) { let _ = self.send(Msg::Quit); }
    pub fn kill(&self) { self.cancel.cancel(); }
}
```

`Program::run()` should return `(JoinHandle<Result<M>>, ProgramHandle)` via a `run_detached()` variant.

### 2.5 Kill/Wait/Quit — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~50 lines | **Depends on**: 2.4

- `quit()` — send `Msg::Quit` (graceful, final render happens)
- `kill()` — cancel token (force, skip final render)
- `wait()` — `Arc<Notify>` signaled on program exit

### 2.6 WithInput/WithOutput/WithEnvironment — NOT IMPLEMENTED

**Priority**: P0 | **Effort**: ~200 lines

Essential for SSH servers and testing.

```rust
pub trait EventSource: Send + 'static {
    fn poll_event(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<Event>>;
}

impl Program<M> {
    pub fn with_input<S: EventSource>(mut self, source: S) -> Self { ... }
    pub fn with_output<W: Write + Send + 'static>(mut self, writer: W) -> Self { ... }
    pub fn with_environment(mut self, env: Vec<(String, String)>) -> Self { ... }
}
```

- Input=None disables keyboard entirely
- Environment passed to color profile detection, terminal type detection
- Output can be any `Write` (file, network socket for SSH)

### 2.7 WithFilter — Message Middleware — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~30 lines

```rust
pub type MessageFilter<M> = Box<dyn Fn(&M, Msg) -> Option<Msg> + Send>;

// In event loop, before model.update():
let msg = match &self.filter {
    Some(f) => match f(&self.model, msg) {
        Some(m) => m,
        None => continue, // Drop message
    },
    None => msg,
};
```

### 2.8 WithContext — External Cancellation — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~30 lines

Accept external `CancellationToken`, create child token for internal use.

### 2.9 Panic Recovery — NOT IMPLEMENTED

**Priority**: P0 | **Effort**: ~100 lines

```rust
// Wrap run_inner() in catch_unwind
let result = std::panic::catch_unwind(AssertUnwindSafe(|| { ... }));

// On panic:
// 1. Restore terminal (disable raw, leave alt screen, show cursor)
// 2. Format panic with \r\n for raw mode readability
// 3. Print to stderr
// 4. If TEA_DEBUG=1: write ruse-panic-{unix_ts}.log
// 5. Return Err(ProgramError::Panic(msg))
```

Also wrap each `tokio::spawn` in catch_unwind for task panics.

### 2.10 Every() — Clock-Synchronized Tick — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~30 lines

```rust
pub fn every<F>(duration: Duration, f: F) -> Cmd
where F: FnOnce(SystemTime) -> Msg + Send + 'static
{
    cmd_async(async move {
        let now = SystemTime::now();
        let epoch_ms = now.duration_since(UNIX_EPOCH).unwrap().as_millis();
        let dur_ms = duration.as_millis();
        let next_boundary_ms = ((epoch_ms / dur_ms) + 1) * dur_ms;
        let wait = Duration::from_millis((next_boundary_ms - epoch_ms) as u64);
        tokio::time::sleep(wait).await;
        f(SystemTime::now())
    })
}
```

### 2.11 Deferred Initialization Messages — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~50 lines

At startup, send before first render:
1. `Msg::ColorProfile(profile)` — detected from env
2. `Msg::WindowSize { width, height }` — terminal size
3. `Msg::Environment(env)` — environment variables (async, after render starts)

### 2.12 Terminal Fallback (/dev/tty) — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~40 lines

```rust
pub fn open_tty() -> io::Result<(File, File)> {
    let f = File::open("/dev/tty")?;
    Ok((f.try_clone()?, f))
}

pub fn is_tty(fd: RawFd) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}
```

If stdout isn't TTY: disable raw mode, renderer becomes no-op.

### 2.13 Println/Printf — Insert Above TUI — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~50 lines

Output text above the managed TUI area that persists across redraws. In alt screen mode, must leave/re-enter alt screen.

### 2.14 Clipboard (OSC 52) — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~80 lines

```rust
pub fn set_clipboard(content: &str) -> Cmd {
    // Send: ESC ] 52 ; c ; {base64} BEL
}
pub fn read_clipboard() -> Cmd {
    // Send: ESC ] 52 ; c ; ? BEL
    // Response comes as Msg::Clipboard { content, selection }
}
```

### 2.15 Raw() Command — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~20 lines

```rust
pub fn raw(sequence: impl Into<String>) -> Cmd {
    let seq = sequence.into();
    cmd(move || Msg::custom(RawSequence(seq)))
}
```

### 2.16 Logging (TEA_DEBUG/TEA_TRACE) — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~50 lines

- `TEA_TRACE=<path>` — trace logging to file
- `TEA_DEBUG=1` — panic log files
- `log_to_file(path, prefix)` — public API

### 2.17 RequestWindowSize / RequestCursorPosition — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~40 lines

Terminal query commands that send escape sequences and receive responses through input stream.

---

## 3. lipgloss vs ruse-style

**Go**: 15,442 lines | **Rust**: 2,822 lines | **Coverage**: ~18%

### 3.1 Layer/Compositor/Canvas System — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~800 lines | **Depends on**: 1.1 (cellbuf)

Entire subsystem for overlapping positioned content.

#### 3.1.1 Layer Type

```rust
struct Layer {
    id: String,
    content: String,         // Rendered content
    x: i32, y: i32, z: i32, // Position + z-index
    width: u32, height: u32,
    children: Vec<Layer>,
}
```

- `add_layers()` — add children, recalculate bounds
- `bounds_with_offset(px, py)` — recursive absolute bounds calculation (union of self + all children)

#### 3.1.2 Compositor

```rust
struct Compositor {
    flattened: Vec<CompositeLayer>,  // Sorted by z
    index: HashMap<String, usize>,  // ID → layer index
}

struct CompositeLayer {
    id: String,
    content: String,
    abs_x: i32, abs_y: i32, z: i32,
    bounds: Rect,
}
```

- `flatten()` — recursively traverse, calculate absolute positions, sort by z
- `draw()` — render all layers onto canvas (low z first)
- `hit_test(x, y)` — iterate from highest z, return first containing layer

#### 3.1.3 Canvas

```rust
struct Canvas {
    cells: Vec<Vec<Cell>>,   // Row-major cell buffer
    width: u32,
    height: u32,
}
```

- `resize()`, `clear()`, `cell_at(x, y)`, `set_cell(x, y, cell)`, `compose(drawable)`, `render() -> String`

### 3.2 Tree/List Rendering — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~600 lines

#### 3.2.1 Tree Type

```rust
struct Tree {
    value: String,
    children: Vec<Tree>,
    renderer: TreeRenderer,  // Controls enumerator + indenter
}

struct TreeRenderer {
    enumerator: Box<dyn Fn(Vec<&Tree>, usize) -> (String, String)>,
    indenter: Box<dyn Fn(Vec<&Tree>, usize) -> String>,
    item_style: Box<dyn Fn(&Tree, bool) -> Style>,
}
```

**Rendering algorithm**: Recursive prefix accumulation — each level adds indentation prefix, children rendered with accumulated prefix.

#### 3.2.2 Enumerators

```rust
enum Enumerator {
    Default,    // ├──/└──
    Rounded,    // ├──/╰──
    Bullet,     // •
    Dash,       // -
    Arabic,     // 1., 2., 3.
    Roman,      // I., II., III. (subtraction algorithm: 1000→M, 900→CM)
    Alphabet,   // a., b., ..., z., aa., ab. (rollover at boundaries)
    Asterisk,   // *
}
```

#### 3.2.3 List Type

Wrapper around Tree with list-specific defaults (single-space indent, bullet enumerator).

### 3.3 Underline Styles — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~80 lines

```rust
enum UnderlineStyle {
    None,
    Single,     // SGR 4 / \x1b[4m
    Double,     // SGR 21 / \x1b[21m
    Curly,      // SGR 4:3 / \x1b[4:3m
    Dotted,     // SGR 4:4 / \x1b[4:4m
    Dashed,     // SGR 4:5 / \x1b[4:5m
}
```

Also: `underline_color(Color)` — separate color via SGR 58;...m

### 3.4 StyleRunes / StyleRanges — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~150 lines

#### StyleRunes

Apply different styles to specific character indices (for fuzzy match highlighting).

```rust
pub fn style_runes(text: &str, indices: &[usize], matched: &Style, unmatched: &Style) -> String {
    // 1. Build HashSet from indices
    // 2. Iterate runes, accumulate groups by style
    // 3. Render each group with appropriate style
}
```

#### StyleRanges

Apply styles to text ranges.

```rust
struct Range { start: usize, end: usize, style: Style }

pub fn style_ranges(text: &str, ranges: &[Range]) -> String {
    // 1. Strip ANSI for accurate indexing
    // 2. Sort ranges by start
    // 3. Render: unstyled gaps + styled ranges
}
```

### 3.5 Whitespace System — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~80 lines

```rust
struct Whitespace {
    chars: Vec<char>,        // Characters to cycle through
    style: Style,            // Style for whitespace fill
}

impl Whitespace {
    fn render(&self, width: usize) -> String {
        // Cycle through chars, measuring width of each
        // Fill remainder with spaces if wide chars create gaps
    }
}
```

### 3.6 Hyperlink Support (OSC 8) — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~50 lines

Add to Style: `link: Option<String>`, `link_params: Option<String>`

Rendering: wrap styled text with `\x1b]8;<params>;<url>\x1b\\` ... `\x1b]8;;\x1b\\`

### 3.7 LightDark / Complete / CompleteFunc — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~60 lines

```rust
// Select color based on terminal background brightness
pub fn light_dark(is_dark: bool, light: Color, dark: Color) -> Color {
    if is_dark { dark } else { light }
}

// Select color variant based on color profile
pub fn complete(profile: ColorProfile, ansi: Color, ansi256: Color, truecolor: Color) -> Color {
    match profile {
        ColorProfile::Ansi => ansi,
        ColorProfile::Ansi256 => ansi256,
        ColorProfile::TrueColor => truecolor,
        _ => Color::NoColor,
    }
}
```

### 3.8 Blend2D — 2D Gradient — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~100 lines | **Depends on**: palette crate

```rust
pub fn blend_2d(width: usize, height: usize, colors: &[Color], angle_deg: f64) -> Vec<Color> {
    // 1. Create 1D diagonal gradient
    // 2. For each cell: calculate rotated position from center
    //    rotX = dx*cos(θ) - dy*sin(θ)
    // 3. Map to gradient index, sample color
    // 4. Return row-major array
}
```

### 3.9 Border Foreground Blend — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~120 lines

Gradient along border perimeter with offset rotation.

```rust
pub fn border_blend(width: usize, height: usize, colors: &[Color], offset: i32)
    -> (Vec<Color>, Vec<Color>, Vec<Color>, Vec<Color>)  // top, right, bottom, left
{
    // 1. Perimeter length = (width+2)*2 + height*2
    // 2. Create 1D gradient of perimeter length
    // 3. Rotate by offset (wrapping, negative = counter-clockwise)
    // 4. Distribute: top takes width+2, right takes height, etc.
    // 5. Reverse bottom and left (drawn in opposite direction)
}
```

### 3.10 Table Advanced Features — PARTIAL

**Priority**: P1 | **Effort**: ~400 lines

#### 3.10.1 Data Interface

```rust
pub trait TableData {
    fn rows(&self) -> usize;
    fn columns(&self) -> usize;
    fn at(&self, row: usize, col: usize) -> &str;
}

pub struct StringData { data: Vec<Vec<String>> }
pub struct FilteredData<'a, D: TableData> { source: &'a D, predicate: Box<dyn Fn(usize) -> bool> }
```

#### 3.10.2 Column Resizing Algorithm (~250 lines in Go)

1. Calculate median non-whitespace length per column
2. If table too narrow: expand columns evenly
3. If too wide: shrink by median difference (preserves narrow columns)
4. Account for padding/border sizes

#### 3.10.3 Cell Wrapping

Wrap content within column widths, increase row height for multi-line cells.

### 3.11 Getter/Unset Methods — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~300 lines

Go has 39+ `Get*()` and `Unset*()` methods. Need all of:
- `get_bold()`, `get_italic()`, `get_foreground()`, `get_padding_top()`, etc.
- `unset_bold()`, `unset_foreground()`, `unset_padding_top()`, etc.
- Cumulative: `frame_size() -> (width, height)` = margins + padding + borders

### 3.12 Transform — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~20 lines

```rust
// Store: transform: Option<Box<dyn Fn(&str) -> String>>
// Applied BEFORE any styling (first step in render pipeline)
impl Style {
    pub fn transform<F: Fn(&str) -> String + 'static>(mut self, f: F) -> Self { ... }
}
```

### 3.13 SetString / Value / String() — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~30 lines

Store a value in the Style, render it via `Display` trait.

```rust
impl Style {
    pub fn set_string(mut self, s: &str) -> Self { self.value = Some(s.to_string()); self }
    pub fn value(&self) -> Option<&str> { self.value.as_deref() }
}

impl Display for Style {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(ref v) = self.value {
            write!(f, "{}", self.render(&[v]))
        } else {
            Ok(())
        }
    }
}
```

### 3.14 Additional Color Functions — PARTIAL

**Priority**: P2 | **Effort**: ~40 lines

Missing: `complementary(color)` (180-degree hue shift), `alpha(color, alpha)`.
Existing: `darken`, `lighten`, `blend_1d`, `is_dark`.

### 3.15 UnderlineSpaces / StrikethroughSpaces — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~20 lines

Extend underline/strikethrough through whitespace padding.

### 3.16 PaddingChar / MarginChar — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~30 lines

Custom fill characters (default NBSP for copy-paste preservation).

---

## 4. bubbles vs ruse-components

**Go**: 13,792 lines | **Rust**: 3,037 lines | **Coverage**: ~22%

### 4.1 KeyMap System — Cross-Cutting, NOT IMPLEMENTED

**Priority**: P0 | **Effort**: ~300 lines (system + all components)

Every interactive Go component has a customizable `KeyMap` struct with `DefaultKeyMap()`.

#### Binding Enhancements

```rust
pub struct Binding {
    keys: Vec<String>,       // Key aliases: "ctrl+c", "alt+right", "enter"
    help: BindingHelp,       // Help text pair
    enabled: bool,
    // NEW:
}

pub struct BindingHelp {
    pub key: String,         // Display: "↑/k"
    pub desc: String,        // Description: "move up"
}

impl Binding {
    pub fn unbind(&mut self) { self.keys.clear(); self.help = Default::default(); }
    pub fn enabled(&self) -> bool { self.enabled && !self.keys.is_empty() }
}
```

#### Per-Component KeyMaps

Each component needs a `KeyMap` struct + `DefaultKeyMap()`. Example for Table:

```rust
pub struct TableKeyMap {
    pub line_up: Binding,      // ↑/k
    pub line_down: Binding,    // ↓/j
    pub page_up: Binding,      // b/pgup
    pub page_down: Binding,    // f/pgdn/space
    pub half_page_up: Binding, // u/ctrl+u
    pub half_page_down: Binding, // d/ctrl+d
    pub goto_top: Binding,     // home/g
    pub goto_bottom: Binding,  // end/G
}
```

Components needing KeyMaps: TextInput (14 bindings), TextArea (21), Viewport (10), Table (8), List (navigation + filter + help), FilePicker (9), Paginator (2).

### 4.2 Focused/Blurred Style States — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~200 lines

Each component needs separate styles for focused vs blurred state.

```rust
pub struct StyleState {
    pub focused: ComponentStyles,
    pub blurred: ComponentStyles,
}
```

### 4.3 TextInput: Suggestion System — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~200 lines

```rust
pub struct TextInput {
    // NEW fields:
    suggestions: Vec<String>,
    matched_suggestions: Vec<String>,
    current_suggestion_idx: usize,
    show_suggestions: bool,
}

impl TextInput {
    pub fn set_suggestions(&mut self, suggestions: Vec<String>) { ... }
    pub fn current_suggestion(&self) -> Option<&str> { ... }

    fn update_suggestions(&mut self) {
        // Filter: suggestion.to_lowercase().starts_with(value.to_lowercase())
        // Reset index if matches changed
    }

    fn accept_suggestion(&mut self) {
        // Replace value with current suggestion, move cursor to end
    }

    fn completion_view(&self) -> String {
        // Display suffix of suggestion beyond typed text
        // "hel" + suggestion "hello" → render "lo" in dim style
    }
}
```

### 4.4 TextInput: Word Navigation — PARTIAL

**Priority**: P1 | **Effort**: ~60 lines

```rust
fn word_backward(&mut self) {
    // 1. Skip whitespace backward
    // 2. Skip non-whitespace backward
}

fn word_forward(&mut self) {
    // 1. Skip whitespace forward
    // 2. Skip non-whitespace forward
}

fn delete_word_forward(&mut self) {
    // alt+d: delete from cursor to end of word
}
```

Bindings: alt+left/ctrl+left/alt+b (backward), alt+right/ctrl+right/alt+f (forward)

### 4.5 TextInput: Horizontal Scrolling — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~100 lines

```rust
pub struct TextInput {
    offset: usize,          // Left scroll position
    offset_right: usize,    // Right boundary
}

fn handle_overflow(&mut self) {
    // If text fits in width: no scrolling
    // If cursor left of viewport: shift left, recalc right
    // If cursor right of viewport: shift right, recalc left
    // Use grapheme-aware width for CJK/emoji
}
```

### 4.6 TextInput: ValidateFunc — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~20 lines

```rust
pub struct TextInput {
    validate: Option<Box<dyn Fn(&str) -> Result<(), String>>>,
    err: Option<String>,
}
```

### 4.7 TextArea: Case Transformations — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~80 lines

```rust
fn uppercase_right(&mut self) {
    self.do_word_right(|_idx, ch| ch.to_uppercase().next().unwrap_or(ch));
}

fn lowercase_right(&mut self) {
    self.do_word_right(|_idx, ch| ch.to_lowercase().next().unwrap_or(ch));
}

fn capitalize_right(&mut self) {
    self.do_word_right(|idx, ch| {
        if idx == 0 { ch.to_uppercase().next().unwrap_or(ch) } else { ch }
    });
}

fn transpose_left(&mut self) {
    // Swap char at cursor-1 with char at cursor, advance cursor
}
```

### 4.8 TextArea: Soft Wrapping + Memoization — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~300 lines

#### Wrap Algorithm

```rust
fn wrap(runes: &[char], width: usize) -> Vec<Vec<char>> {
    // Word-boundary wrapping with grapheme-aware width
    // 1. Accumulate words and spaces
    // 2. When word+spaces exceed width: start new line
    // 3. Handle double-width characters at line boundary
}
```

#### LRU Memoization Cache

```rust
struct MemoCache<K: Hash + Eq, V: Clone> {
    capacity: usize,
    cache: HashMap<String, V>,    // SHA256 hash → value
    eviction: VecDeque<String>,    // LRU order
}
```

Cache key: SHA256 of `format!("{}:{}", text, width)`. Used to avoid rewrapping unchanged lines.

### 4.9 TextArea: LineInfo/PromptInfo/CursorStyle — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~80 lines

```rust
pub struct LineInfo {
    pub width: usize,        // Columns in physical line
    pub char_width: usize,   // Character count
    pub height: usize,       // Visual rows from soft wrap
    pub start_column: usize,
    pub column_offset: usize,
    pub row_offset: usize,
    pub char_offset: usize,
}

pub struct PromptInfo {
    pub line_number: usize,
    pub focused: bool,
}
```

### 4.10 TextArea: Limits — PARTIAL

**Priority**: P2 | **Effort**: ~40 lines

Enforce `char_limit`, `max_height`, `max_width`, `max_lines` (Go defaults to 10,000).

### 4.11 Viewport: Left Gutter Function — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~80 lines

```rust
pub type GutterFunc = Box<dyn Fn(GutterContext) -> String>;

pub struct GutterContext {
    pub index: usize,        // Line index
    pub total_lines: usize,
    pub soft: bool,          // Soft-wrapped continuation line
}

// Example: line numbers
let gutter = |ctx: GutterContext| -> String {
    if ctx.soft { return "     | ".into(); }
    if ctx.index >= ctx.total_lines { return "   ~ | ".into(); }
    format!("{:4} | ", ctx.index + 1)
};
```

### 4.12 Viewport: Highlight System — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~200 lines

```rust
pub struct Viewport {
    highlights: Vec<HighlightInfo>,
    highlight_idx: usize,
    highlight_style: Style,
    selected_highlight_style: Style,
}

struct HighlightInfo {
    line_start: usize,
    line_end: usize,
    lines: HashMap<usize, (usize, usize)>, // line → (col_start, col_end) in grapheme positions
}

impl Viewport {
    pub fn set_highlights(&mut self, matches: &[(usize, usize)]) {
        // Convert byte positions to grapheme positions
        // Build line→column range map
    }
    pub fn highlight_next(&mut self) { ... }
    pub fn highlight_prev(&mut self) { ... }
}
```

### 4.13 Viewport: StyleLineFunc — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~20 lines

```rust
pub struct Viewport {
    style_line_func: Option<Box<dyn Fn(usize) -> Style>>,
}
```

### 4.14 Viewport: Horizontal Scrolling — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~80 lines

```rust
pub struct Viewport {
    x_offset: usize,
    horizontal_step: usize,  // Default 6
}

fn line_left(&mut self) { ... }
fn line_right(&mut self) { ... }
fn page_left(&mut self) { ... }
fn page_right(&mut self) { ... }
```

### 4.15 Viewport: HalfPageUp/Down — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~10 lines

```rust
fn half_page_up(&mut self) { self.line_up(self.height / 2); }
fn half_page_down(&mut self) { self.line_down(self.height / 2); }
```

### 4.16 List: ItemDelegate — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~150 lines

```rust
pub trait ItemDelegate: Send {
    fn render(&self, model: &List, index: usize, item: &dyn ListItem) -> String;
    fn height(&self) -> usize;
    fn spacing(&self) -> usize;
    fn update(&mut self, msg: &Msg, model: &mut List) -> Cmd;
}

pub struct DefaultDelegate {
    pub show_description: bool,
    pub styles: DefaultItemStyles,
}
```

DefaultDelegate renders title + description with matched-character highlighting via `style_runes()`.

### 4.17 List: Fuzzy Matching — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~150 lines | **Depends on**: fuzzy matching crate or custom impl

```rust
pub type FilterFunc = Box<dyn Fn(&str, &[String]) -> Vec<Rank> + Send>;

pub struct Rank {
    pub index: usize,
    pub matched_indices: Vec<usize>,
}

// Default: use `fuzzy-matcher` crate or implement simple fuzzy
pub fn default_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    // Score each target, return sorted by score descending
    // Include matched character indices for highlighting
}
```

### 4.18 List: FilterState Enum — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~20 lines

```rust
pub enum FilterState {
    Unfiltered,    // No filter active
    Filtering,     // User editing filter input
    FilterApplied, // Filter set, not editing
}
```

### 4.19 List: Status Message System — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~40 lines

```rust
pub struct List {
    status_message: Option<String>,
    status_message_lifetime: Duration,  // Default 1s
}

pub fn new_status_message(&mut self, msg: &str) -> Cmd {
    self.status_message = Some(msg.to_string());
    // Return tick command that clears after lifetime
}
```

### 4.20 Table: HalfPageUp/Down + Help — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~30 lines

Same pattern as Viewport.

### 4.21 FilePicker: Full Styles — PARTIAL

**Priority**: P2 | **Effort**: ~80 lines

Go has 11 style variants. Add missing:
- `DisabledCursor`, `Symlink`, `DisabledFile`, `Permission`, `DisabledSelected`, `FileSize`, `EmptyDirectory`

### 4.22 FilePicker: Permission/Size/Symlink Display — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~100 lines

- Permissions in ls format: `-rw-r--r--`
- File sizes with humanize: `1.2 KB`, `3.4 MB`
- Symlink detection via `fs::symlink_metadata()` + `is_symlink()`

### 4.23 FilePicker: AllowedTypes + History — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~60 lines

```rust
pub struct FilePicker {
    allowed_types: Vec<String>,  // [".txt", ".md"]
    selected_stack: Vec<usize>,  // History stack for back navigation
    min_stack: Vec<usize>,
    max_stack: Vec<usize>,
}
```

### 4.24 Progress: ColorFunc + Gradient — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~100 lines

```rust
pub type ColorFunc = Box<dyn Fn(f64, f64) -> Color>;  // (total_percent, current_position)

pub struct Progress {
    color_func: Option<ColorFunc>,
    blend: Vec<Color>,           // Multi-color gradient stops
    scale_blend: bool,           // Scale to filled portion only
}
```

Half-block character (`▌`) enables 2x color resolution with separate FG/BG colors.

### 4.25 Help: Column Layout — PARTIAL

**Priority**: P2 | **Effort**: ~80 lines

Full help should render groups as side-by-side columns (horizontal join), not vertical newlines.

```rust
pub fn full_help_view(&self, groups: &[Vec<&Binding>]) -> String {
    // 1. Render each group as column of "key desc" lines
    // 2. Pad all columns to same height
    // 3. Join side-by-side with separator
}
```

### 4.26 Help: Light/Dark Themes — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~30 lines

```rust
pub fn default_styles(is_dark: bool) -> HelpStyles {
    if is_dark { /* darker colors */ } else { /* lighter colors */ }
}
```

### 4.27 Missing Spinner Presets — NOT IMPLEMENTED

**Priority**: P3 | **Effort**: ~20 lines

Add: Jump, Points, Monkey, Meter, Hamburger.

---

## 5. glamour vs ruse-glamour

**Go**: 4,799 lines | **Rust**: 1,487 lines | **Coverage**: ~31%

### 5.1 Element-Based Rendering Architecture — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~400 lines

Go uses element/renderer/finisher pattern with block stack. Rust uses flat event walking.

```rust
trait ElementRenderer {
    fn render(&self, w: &mut dyn Write, ctx: &RenderContext) -> Result<()>;
}

trait ElementFinisher {
    fn finish(&self, w: &mut dyn Write, ctx: &RenderContext) -> Result<()>;
}

struct Element {
    entering: String,
    exiting: String,
    renderer: Option<Box<dyn ElementRenderer>>,
    finisher: Option<Box<dyn ElementFinisher>>,
}
```

Benefits: separation of collect/output phases, block stack management, style cascading, lazy output.

### 5.2 Block Stack with Nested Context — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~150 lines

```rust
struct BlockStack(Vec<BlockElement>);

struct BlockElement {
    buffer: String,          // Accumulated content
    style: StyleBlock,
    margin: bool,
    newline: bool,
}

impl BlockStack {
    fn indent(&self) -> usize { /* cumulative indent */ }
    fn margin(&self) -> usize { /* cumulative margin */ }
    fn width(&self, ctx: &RenderContext) -> usize {
        ctx.word_wrap.saturating_sub(self.indent() + self.margin() * 2)
    }
    fn current(&self) -> &BlockElement { self.0.last().unwrap() }
    fn parent(&self) -> &BlockElement { &self.0[self.0.len() - 2] }
}
```

### 5.3 Style Cascading/Inheritance — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~80 lines

```rust
fn cascade_style(parent: &StyleBlock, child: &StyleBlock) -> StyleBlock {
    let mut result = child.clone();
    // Apply parent defaults where child is unset
    if result.color.is_none() { result.color = parent.color.clone(); }
    if result.background_color.is_none() { result.background_color = parent.background_color.clone(); }
    if result.bold.is_none() { result.bold = parent.bold; }
    // ... etc for all style properties
    result
}
```

### 5.4 Table Rendering — PARTIAL

**Priority**: P1 | **Effort**: ~200 lines

pulldown-cmark parses tables but Rust renderer doesn't handle `Event::Start(Tag::Table)`.

Need: column alignment, cell padding, border rendering, link footnoting within tables.

### 5.5 Link Footnoting — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~100 lines

Collect links during rendering, append as numbered footnotes at end.

```rust
struct LinkCollector {
    links: Vec<(String, String)>,  // (display_text, url)
}

impl LinkCollector {
    fn add(&mut self, display: &str, url: &str) -> String {
        let idx = self.links.len() + 1;
        self.links.push((display.into(), url.into()));
        format!("[{}]", idx)
    }

    fn render_footnotes(&self) -> String {
        self.links.iter().enumerate()
            .map(|(i, (_, url))| format!("[{}] {}", i + 1, url))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

### 5.6 List Nesting — PARTIAL

**Priority**: P1 | **Effort**: ~60 lines

Proper per-level indentation. `StyleList.level_indent` controls indent per nesting level.

### 5.7 Task Checkboxes — PARTIAL

**Priority**: P2 | **Effort**: ~30 lines

Add `StyleTask { ticked: String, unticked: String }` to style config. Render `[x]` / `[ ]` with style.

### 5.8 Strikethrough — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~10 lines

pulldown-cmark parses it, just need to apply `strikethrough(true)` style.

### 5.9 Images — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~40 lines

Render alt text + URL. Optional OSC 8 hyperlink wrapping.

### 5.10 GutterWriter / Indent Management — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~80 lines

Writer that manages indentation levels, resets/restores ANSI pen state across line breaks.

### 5.11 Theme JSON Loading — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~30 lines

```rust
pub fn load_theme(path: &Path) -> Result<StyleConfig> {
    let json = fs::read_to_string(path)?;
    serde_json::from_str(&json).map_err(|e| e.into())
}
```

### 5.12 GLAMOUR_STYLE Env Var — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~10 lines

```rust
pub fn render_with_env(markdown: &str) -> String {
    let theme = std::env::var("GLAMOUR_STYLE").unwrap_or_else(|_| "dark".into());
    render_with_theme(markdown, &theme)
}
```

### 5.13 Definition Lists + Footnotes + Emoji — NOT IMPLEMENTED

**Priority**: P3 | **Effort**: ~100 lines

Need element handlers for: `DefinitionList`, `DefinitionTerm`, `DefinitionDescription`, `Footnote`, `FootnoteList`, `Emoji`.

---

## 6. colorprofile vs ruse-colorprofile

**Go**: 1,441 lines | **Rust**: 1,631 lines | **Coverage**: ~113%

Closest to complete. 4 small gaps remaining.

### 6.1 Terminfo Database Lookup — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~40 lines | **Depends on**: `terminfo` crate

```rust
pub fn from_terminfo(term: &str) -> Option<Profile> {
    // Load terminfo for $TERM
    // Check extended booleans "Tc" and "RGB"
    // If either present: TrueColor
}
```

### 6.2 Tmux Capability Checking — NOT IMPLEMENTED

**Priority**: P2 | **Effort**: ~30 lines

```rust
pub fn from_tmux() -> Option<Profile> {
    if std::env::var("TMUX").ok().filter(|v| !v.is_empty()).is_none() {
        return None;
    }
    let output = Command::new("tmux").arg("info").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if (line.contains("Tc") || line.contains("RGB")) && line.contains("true") {
            return Some(Profile::TrueColor);
        }
    }
    Some(Profile::Ansi256)
}
```

### 6.3 Windows Version Detection — NOT IMPLEMENTED

**Priority**: P3 | **Effort**: ~40 lines

Check Windows build numbers via `RtlGetNtVersionNumbers`:
- < 10586: NoTTY (unless ANSICON)
- < 14931: Ansi256
- >= 14931: TrueColor

### 6.4 Conversion Cache — NOT IMPLEMENTED

**Priority**: P1 | **Effort**: ~40 lines

Thread-safe cache for color conversions using `RwLock<HashMap<Color, Color>>`.

---

## 7. harmonica vs ruse-harmonica

**Go**: 984 lines | **Rust**: 620 lines | **Coverage**: ~63%

**AT PARITY**. Both have identical Spring (3 damping modes) and Projectile implementations with matching accessors. No additional easing functions exist in Go beyond spring/projectile.

No implementation work needed.

---

## 8. Implementation Plan

### Phase 1: Core Infrastructure (P0)

Foundation work that unblocks everything else.

| Task | Library | Effort | Depends On |
|---|---|---|---|
| 1.1 Cell buffer (Cell, CellStyle, Link, Buffer) | ruse-ansi | ~600 lines | — |
| 1.1.5 Screen (double-buffer, diff, cursor opt) | ruse-ansi | ~900 lines | 1.1 |
| 2.1 Integrate cellbuf renderer into Program | ruse-runtime | ~800 lines | 1.1.5 |
| 2.9 Panic recovery | ruse-runtime | ~100 lines | — |
| 2.2 Exec/ExecProcess | ruse-runtime | ~200 lines | — |
| 2.3 Suspend/Resume (SIGTSTP/SIGCONT) | ruse-runtime | ~100 lines | — |
| 2.4 Send() / ProgramHandle | ruse-runtime | ~50 lines | — |
| 2.6 WithInput/WithOutput/WithEnvironment | ruse-runtime | ~200 lines | — |
| 4.1 KeyMap system (all components) | ruse-components | ~300 lines | — |

**Estimated**: ~3,250 lines | **Verification**: `cargo test --workspace`, all 5 examples run flicker-free

### Phase 2: Essential Features (P1)

Features needed for production use.

| Task | Library | Effort | Depends On |
|---|---|---|---|
| 1.7 Grapheme cluster handling | ruse-ansi | ~200 lines | — |
| 2.5 Kill/Wait/Quit | ruse-runtime | ~50 lines | Phase 1 |
| 2.7 WithFilter | ruse-runtime | ~30 lines | Phase 1 |
| 2.8 WithContext | ruse-runtime | ~30 lines | Phase 1 |
| 3.3 Underline styles (curly, dotted, dashed) | ruse-style | ~80 lines | — |
| 3.4 StyleRunes / StyleRanges | ruse-style | ~150 lines | — |
| 3.10 Table advanced (Data, column resize, wrapping) | ruse-style | ~400 lines | — |
| 3.11 Getter/Unset methods (39+) | ruse-style | ~300 lines | — |
| 4.2 Focused/Blurred style states | ruse-components | ~200 lines | — |
| 4.3 TextInput suggestions | ruse-components | ~200 lines | — |
| 4.4-4.5 TextInput word nav + scrolling | ruse-components | ~160 lines | — |
| 4.8 TextArea soft wrapping + memo | ruse-components | ~300 lines | — |
| 4.11 Viewport gutter | ruse-components | ~80 lines | — |
| 4.12 Viewport highlights | ruse-components | ~200 lines | 3.4 |
| 4.16-4.17 List ItemDelegate + fuzzy | ruse-components | ~300 lines | 3.4 |
| 5.1-5.3 Glamour element arch + block stack + cascade | ruse-glamour | ~630 lines | — |
| 5.4 Glamour table rendering | ruse-glamour | ~200 lines | 5.1 |
| 5.6 Glamour list nesting | ruse-glamour | ~60 lines | 5.2 |
| 6.4 Color conversion cache | ruse-colorprofile | ~40 lines | — |

**Estimated**: ~3,610 lines | **Verification**: `cargo test --workspace`, fuzzy filtering works, glamour renders tables

### Phase 3: Polish & Advanced (P2)

Quality-of-life improvements and advanced features.

| Task | Library | Effort | Depends On |
|---|---|---|---|
| 1.2 ANSI parser state machine | ruse-ansi | ~800 lines | — |
| 1.5 OSC sequence handlers | ruse-ansi | ~300 lines | — |
| 1.6 Advanced SGR | ruse-ansi | ~100 lines | — |
| 1.8 Terminal queries | ruse-ansi | ~100 lines | — |
| 2.10-2.17 Runtime extras (Every, deferred init, println, clipboard, raw, logging, queries, tty fallback) | ruse-runtime | ~360 lines | Phase 1 |
| 3.1 Layer/Compositor/Canvas | ruse-style | ~800 lines | Phase 1 (cellbuf) |
| 3.2 Tree/List rendering | ruse-style | ~600 lines | — |
| 3.5-3.16 Style extras (whitespace, hyperlinks, LightDark, Blend2D, border blend, transform, SetString, colors, padding/margin chars) | ruse-style | ~450 lines | — |
| 4.6-4.10 TextInput validate + TextArea extras | ruse-components | ~220 lines | — |
| 4.13-4.15 Viewport extras (StyleLineFunc, horizontal scroll, half-page) | ruse-components | ~110 lines | — |
| 4.18-4.26 Component extras (FilterState, status msgs, table/help/filepicker/progress enhancements) | ruse-components | ~420 lines | — |
| 5.5-5.12 Glamour extras (footnotes, checkboxes, strikethrough, images, gutter, themes, env) | ruse-glamour | ~300 lines | Phase 2 |
| 6.1-6.2 Colorprofile terminfo + tmux | ruse-colorprofile | ~70 lines | — |

**Estimated**: ~4,630 lines | **Verification**: `cargo test --workspace`, all examples run, glamour renders all markdown elements

### Phase 4: Specialized (P3)

Rarely-needed features for completeness.

| Task | Library | Effort | Depends On |
|---|---|---|---|
| 1.3 Virtual terminal emulator | ruse-ansi | ~2,000 lines | Phase 1 (cellbuf), Phase 3 (parser) |
| 1.4 Kitty graphics protocol | ruse-ansi | ~400 lines | — |
| 1.9 Color palette management | ruse-ansi | ~150 lines | — |
| 4.27 Missing spinner presets | ruse-components | ~20 lines | — |
| 5.13 Glamour definition lists + footnotes + emoji | ruse-glamour | ~100 lines | Phase 2 (element arch) |
| 6.3 Windows version detection | ruse-colorprofile | ~40 lines | — |

**Estimated**: ~2,710 lines

### Total Estimated New Code

| Phase | Lines | Cumulative |
|---|---|---|
| Phase 1 (P0) | ~3,250 | ~15,700 |
| Phase 2 (P1) | ~3,610 | ~19,310 |
| Phase 3 (P2) | ~4,630 | ~23,940 |
| Phase 4 (P3) | ~2,710 | ~26,650 |

Final Rust codebase: ~26,650 lines (~22% of Go's 118,755). The difference is explained by:
- crossterm replacing ~35,000 lines of Go terminal infrastructure
- Rust's type system replacing Go's runtime checks
- No Pony layout framework (~8,000 lines skipped)
- Denser expression (pattern matching, iterators, no error boilerplate)
