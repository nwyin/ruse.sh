# Ruse.sh — Status & Roadmap

Rust TUI framework at full parity with charm.sh's 7 core libraries.
UI layer for [tau](~/projects/harnesses/tau) and future dev tools.

Last updated: 2026-03-31

---

## Parity Summary

| Crate | Charm.sh | Parity | Notes |
|-------|----------|--------|-------|
| ruse-runtime | bubbletea | ~90% | Missing terminal capability messages |
| ruse-style | lipgloss | ~95% | Missing WrapWriter |
| ruse-components | bubbles | ~95% | All 14 components, undo/redo, paste |
| ruse-glamour | glamour | ~90% | 7 themes, emoji, HTML sanitization |
| ruse-ansi | x/ansi | ~95% | Kitty + Sixel + iTerm2 + Mosaic |
| ruse-colorprofile | colorprofile | ~90% | Env-var + TERM_PROGRAM detection |
| ruse-harmonica | harmonica | 100% | Pre-computed coefficients (ahead of Go) |

---

## Remaining Gaps

### Runtime (~90%)
- [ ] Terminal version detection — parse XTVERSION response → `Msg::TerminalVersion`
- [ ] Keyboard enhancements — Kitty keyboard protocol negotiation
- [ ] Mode reporting — DECRPM query/response
- [ ] Color profile as message — emit `Msg::ColorProfile` after startup detection
- [ ] Foreground/background color query — parse OSC 10/11 responses
- [ ] Cursor position query — parse DSR response

### Styling (~95%)
- [ ] WrapWriter — standalone `impl Write` that auto-wraps at column width

### Components (~95%)
*No critical gaps. Polish items only.*

### Glamour (~90%)
- [ ] More themes — port additional charm.sh community themes beyond the 7 built-in
- [ ] Per-element style granularity — LevelIndent on more elements, full cascading

### ANSI (~95%)
*No critical gaps. Image protocols all implemented.*

### Color Profile (~90%)
- [ ] Terminfo database lookup — read terminfo for capability detection (optional feature)

---

## What's Implemented

### Runtime (ruse-runtime)
Elm architecture (Model/Update/View), async event loop (tokio), keyboard/mouse/paste/focus/resize events, cell-buffer diff renderer, synchronized output, Cmd system (sync/async/batch/sequence/tick/every), subprocess exec, suspend/resume, clipboard read/write, cursor control, panic recovery, alt screen, progress bar reporting (OSC 9;4).

### Styling (ruse-style)
42+ style properties via bitflags, 12-step render pipeline, bold/italic/underline/strikethrough/reverse/blink/faint, underline variants (single/double/curly/dotted/dashed), fg/bg/underline colors, padding/margin (CSS shorthand), width/height/max constraints, alignment, 11 border presets, `join_horizontal`/`join_vertical`/`place`, color blending/gradients, style inheritance, fuzzy match highlighting, table/tree rendering, Layer/Compositor, **Transform hook**, **Hyperlink (OSC 8)**.

### Components (ruse-components)
All 14: TextInput (echo modes, suggestions, validation, **clipboard paste**), TextArea (multi-line, soft wrap, **undo/redo**), Viewport (scroll, mouse, gutter, highlights), List (fuzzy filter, status messages, **spinner**, **paginator**), Table, FilePicker, Spinner (12 presets), Progress (spring animation), Help (column layout), Timer/Stopwatch, Cursor, Paginator, Key binding system.

### Glamour (ruse-glamour)
Full markdown parsing (pulldown-cmark), syntax highlighting (syntect, 100+ languages), headings/code blocks/block quotes/lists/tables/links/images, **emoji shortcodes** (`:rocket:` → 🚀), **HTML sanitization** (ammonia), **7 built-in themes** (dark, light, dracula, tokyo-night, ascii, notty, pink), **template format strings**, **nested list indentation**, **base URL resolution**, task lists, definition lists, math, footnotes.

### ANSI (ruse-ansi)
Full parser state machine (15 states), cell buffer with diff rendering, VT102 emulator, SGR builder, strip/truncate/wrap/pad, grapheme-aware string ops, OSC sequences, **Kitty graphics**, **Sixel graphics** (palette quantization), **iTerm2 inline images** (OSC 1337), **Mosaic rendering** (Unicode half-blocks), **image protocol detection**.

### Color Profile (ruse-colorprofile)
5 profile levels, NO_COLOR/CLICOLOR/COLORTERM detection, tmux capability detection, **TERM_PROGRAM detection** (iTerm, WezTerm, mintty, Hyper), **ConEmuANSI** (Windows), Google Cloud Shell, WT_SESSION.

### Animation (ruse-harmonica)
Spring (damped harmonic oscillator, all 3 regimes), pre-computed motion coefficients, projectile physics (3D), fps() helper. **Full parity.**

---

## Priority Order (remaining work)

1. **Runtime** — terminal capability messages (requires response parsing)
2. **Glamour** — more themes, element granularity
3. **Color Profile** — terminfo (optional feature flag)
4. **Styling** — WrapWriter
5. **Examples & Docs** — ongoing

## Out of Scope

- x/ utilities — use Rust ecosystem crates at app layer
- crates.io publishing — not until framework is stable
