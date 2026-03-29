use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

use rouge_ansi::SgrStyle;

use crate::style::{StyleConfig, StylePrimitive};

/// Markdown-to-ANSI terminal renderer.
pub struct TermRenderer {
    style: StyleConfig,
    word_wrap: usize,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    syntax_theme: String,
}

/// Context for tracking rendering state inside block/inline elements.
#[derive(Clone, Debug)]
enum Context {
    Paragraph,
    Heading(HeadingLevel),
    BlockQuote,
    CodeBlock,
    List,
    Item,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
    Image,
    Table,
    TableHead,
    TableRow,
    TableCell,
    DefinitionList,
    DefinitionListTitle,
    DefinitionListDefinition,
    HtmlBlock,
}

impl TermRenderer {
    pub fn new(style: StyleConfig) -> Self {
        Self {
            style,
            word_wrap: 0,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    pub fn with_word_wrap(mut self, w: usize) -> Self {
        self.word_wrap = w;
        self
    }

    pub fn with_syntax_theme(mut self, theme: &str) -> Self {
        self.syntax_theme = theme.to_string();
        self
    }

    /// Render a markdown string to ANSI-styled terminal output.
    pub fn render(&self, markdown: &str) -> String {
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);

        let mut out = String::new();
        let mut stack: Vec<Context> = Vec::new();
        let mut code_buf = String::new();
        let mut code_lang: Option<String> = None;
        let mut in_code_block = false;
        let mut link_text_buf = String::new();
        let mut in_link = false;
        let mut link_dest = String::new();
        let mut in_image = false;
        let mut image_text_buf = String::new();
        let mut image_dest = String::new();
        let mut list_number: Option<u64> = None;

        // Table state
        let mut in_table = false;
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell = String::new();
        let mut in_table_head = false;

        // Emit document block_prefix
        if let Some(ref bp) = self.style.document.block_prefix {
            out.push_str(bp);
        }

        let margin = self.style.document.margin.unwrap_or(0) as usize;

        for event in parser {
            // If we're accumulating code block text, handle specially
            if in_code_block {
                match event {
                    Event::Text(text) => {
                        code_buf.push_str(&text);
                        continue;
                    }
                    Event::End(TagEnd::CodeBlock) => {
                        in_code_block = false;
                        stack.pop();

                        let highlighted = self.highlight_code(&code_buf, code_lang.as_deref());
                        let cb_margin = self.style.code_block.style.margin.unwrap_or(0) as usize;
                        let total_indent = margin + cb_margin;

                        for line in highlighted.lines() {
                            indent_line(&mut out, total_indent);
                            out.push_str(line);
                            out.push('\n');
                        }
                        // Trailing blank line after code block
                        out.push('\n');

                        code_buf.clear();
                        code_lang = None;
                        continue;
                    }
                    _ => continue,
                }
            }

            // Table cell accumulation
            if in_table {
                match event {
                    Event::Start(Tag::TableHead) => {
                        in_table_head = true;
                        current_row = Vec::new();
                        stack.push(Context::TableHead);
                        continue;
                    }
                    Event::End(TagEnd::TableHead) => {
                        // The header row's cells are directly inside TableHead
                        // (no TableRow wrapper in pulldown-cmark 0.12).
                        table_rows.push(std::mem::take(&mut current_row));
                        in_table_head = false;
                        stack.pop();
                        continue;
                    }
                    Event::Start(Tag::TableRow) => {
                        current_row = Vec::new();
                        stack.push(Context::TableRow);
                        continue;
                    }
                    Event::End(TagEnd::TableRow) => {
                        table_rows.push(std::mem::take(&mut current_row));
                        stack.pop();
                        continue;
                    }
                    Event::Start(Tag::TableCell) => {
                        current_cell = String::new();
                        stack.push(Context::TableCell);
                        continue;
                    }
                    Event::End(TagEnd::TableCell) => {
                        current_row.push(std::mem::take(&mut current_cell));
                        stack.pop();
                        continue;
                    }
                    Event::Text(ref text) => {
                        if stack.iter().any(|c| matches!(c, Context::TableCell)) {
                            current_cell.push_str(text);
                            continue;
                        }
                    }
                    Event::Code(ref code) => {
                        if stack.iter().any(|c| matches!(c, Context::TableCell)) {
                            current_cell.push('`');
                            current_cell.push_str(code);
                            current_cell.push('`');
                            continue;
                        }
                    }
                    Event::End(TagEnd::Table) => {
                        in_table = false;
                        stack.pop();
                        self.render_table(&table_rows, in_table_head, margin, &mut out);
                        table_rows.clear();
                        out.push('\n');
                        continue;
                    }
                    _ => continue,
                }
            }

            match event {
                Event::Start(Tag::Paragraph) => {
                    stack.push(Context::Paragraph);
                }
                Event::End(TagEnd::Paragraph) => {
                    stack.pop();
                    out.push('\n');
                    // Extra blank line between paragraphs
                    if !out.ends_with("\n\n") {
                        out.push('\n');
                    }
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    stack.push(Context::Heading(level));
                    let prim = self.heading_style(level);
                    // Emit any block_prefix from the heading base style
                    if let Some(ref bp) = self.style.heading.block_prefix {
                        indent_line(&mut out, margin);
                        out.push_str(bp);
                    }
                    // Emit prefix
                    indent_line(&mut out, margin);
                    if let Some(ref pfx) = prim.prefix {
                        let sgr = self.build_heading_sgr(level);
                        out.push_str(&sgr.open());
                        out.push_str(pfx);
                    }
                }
                Event::End(TagEnd::Heading(level)) => {
                    stack.pop();
                    let prim = self.heading_style(level);
                    if let Some(ref sfx) = prim.suffix {
                        out.push_str(sfx);
                    }
                    out.push_str(SgrStyle::close());
                    if let Some(ref bs) = self.style.heading.block_suffix {
                        out.push_str(bs);
                    }
                    out.push('\n');
                }
                Event::Start(Tag::BlockQuote(_)) => {
                    stack.push(Context::BlockQuote);
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    stack.pop();
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    let lang = match &kind {
                        CodeBlockKind::Fenced(lang) => {
                            let l = lang.split(',').next().unwrap_or("").trim().to_string();
                            if l.is_empty() { None } else { Some(l) }
                        }
                        CodeBlockKind::Indented => None,
                    };
                    in_code_block = true;
                    code_lang = lang.clone();
                    code_buf.clear();
                    stack.push(Context::CodeBlock);
                }
                Event::Start(Tag::List(start)) => {
                    list_number = start;
                    stack.push(Context::List);
                }
                Event::End(TagEnd::List(_)) => {
                    stack.pop();
                    list_number = None;
                    out.push('\n');
                }
                Event::Start(Tag::Item) => {
                    stack.push(Context::Item);
                    let bq_indent = self.blockquote_indent(&stack);
                    indent_line(&mut out, margin + bq_indent);

                    if let Some(ref num) = list_number {
                        // Ordered list
                        let enum_prim = &self.style.enumeration;
                        let sgr = self.build_sgr(enum_prim);
                        out.push_str(&sgr.styled(&format!("{num}")));
                        if let Some(ref bp) = enum_prim.block_prefix {
                            out.push_str(bp);
                        }
                        list_number = Some(num + 1);
                    } else if let Some(ref bp) = self.style.item.block_prefix {
                        // Unordered list
                        out.push_str(bp);
                    }
                }
                Event::End(TagEnd::Item) => {
                    stack.pop();
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                }
                Event::Start(Tag::Emphasis) => {
                    stack.push(Context::Emphasis);
                }
                Event::End(TagEnd::Emphasis) => {
                    stack.pop();
                }
                Event::Start(Tag::Strong) => {
                    stack.push(Context::Strong);
                }
                Event::End(TagEnd::Strong) => {
                    stack.pop();
                }
                Event::Start(Tag::Strikethrough) => {
                    stack.push(Context::Strikethrough);
                }
                Event::End(TagEnd::Strikethrough) => {
                    stack.pop();
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    in_link = true;
                    link_dest = dest_url.to_string();
                    link_text_buf.clear();
                    stack.push(Context::Link);
                }
                Event::End(TagEnd::Link) => {
                    in_link = false;
                    stack.pop();

                    let link_text_sgr = self.build_sgr(&self.style.link_text);
                    let link_sgr = self.build_sgr(&self.style.link);

                    out.push_str(&link_text_sgr.styled(&link_text_buf));
                    if !link_dest.is_empty() && link_dest != link_text_buf {
                        out.push_str(" (");
                        out.push_str(&link_sgr.styled(&link_dest));
                        out.push(')');
                    }

                    link_text_buf.clear();
                    link_dest.clear();
                }
                Event::Start(Tag::Image { dest_url, .. }) => {
                    in_image = true;
                    image_dest = dest_url.to_string();
                    image_text_buf.clear();
                    stack.push(Context::Image);
                }
                Event::End(TagEnd::Image) => {
                    in_image = false;
                    stack.pop();

                    let text_sgr = self.build_sgr(&self.style.image_text);
                    let img_sgr = self.build_sgr(&self.style.image);

                    let label = if image_text_buf.is_empty() {
                        "Image".to_string()
                    } else {
                        format!("Image: {}", image_text_buf)
                    };
                    out.push_str(&text_sgr.styled(&label));
                    if !image_dest.is_empty() {
                        out.push_str(" (");
                        out.push_str(&img_sgr.styled(&image_dest));
                        out.push(')');
                    }

                    image_text_buf.clear();
                    image_dest.clear();
                }
                Event::Start(Tag::Table(_)) => {
                    in_table = true;
                    in_table_head = false;
                    table_rows.clear();
                    stack.push(Context::Table);
                }
                Event::Start(Tag::DefinitionList) => {
                    stack.push(Context::DefinitionList);
                }
                Event::End(TagEnd::DefinitionList) => {
                    stack.pop();
                    out.push('\n');
                }
                Event::Start(Tag::DefinitionListTitle) => {
                    stack.push(Context::DefinitionListTitle);
                    indent_line(&mut out, margin);
                }
                Event::End(TagEnd::DefinitionListTitle) => {
                    stack.pop();
                    out.push('\n');
                }
                Event::Start(Tag::DefinitionListDefinition) => {
                    stack.push(Context::DefinitionListDefinition);
                    if let Some(ref bp) = self.style.definition_description.block_prefix {
                        indent_line(&mut out, margin);
                        out.push_str(bp);
                    }
                }
                Event::End(TagEnd::DefinitionListDefinition) => {
                    stack.pop();
                    out.push('\n');
                }
                Event::Start(Tag::HtmlBlock) => {
                    stack.push(Context::HtmlBlock);
                }
                Event::End(TagEnd::HtmlBlock) => {
                    stack.pop();
                }
                Event::Start(Tag::FootnoteDefinition(_)) | Event::End(TagEnd::FootnoteDefinition) => {
                    // Footnotes: minimal handling
                }
                Event::Start(Tag::MetadataBlock(_)) | Event::End(TagEnd::MetadataBlock(_)) => {
                    // Skip metadata blocks
                }
                Event::Text(text) => {
                    if in_link {
                        link_text_buf.push_str(&text);
                        continue;
                    }
                    if in_image {
                        image_text_buf.push_str(&text);
                        continue;
                    }
                    let styled = self.style_text(&text, &stack);
                    let bq_indent = self.blockquote_indent(&stack);
                    let is_in_item = stack.iter().any(|c| matches!(c, Context::Item));
                    let is_in_heading = stack.iter().any(|c| matches!(c, Context::Heading(_)));

                    if !is_in_item && !is_in_heading && bq_indent == 0 {
                        // Only indent the start of paragraph text at the document margin
                        let needs_indent = stack.iter().any(|c| matches!(c, Context::Paragraph));
                        if needs_indent && (out.is_empty() || out.ends_with('\n')) {
                            indent_line(&mut out, margin);
                        }
                    }
                    if bq_indent > 0 && (out.is_empty() || out.ends_with('\n')) {
                        indent_line(&mut out, margin);
                        let token = self.style.block_quote.indent_token.as_deref().unwrap_or("  ");
                        let bq_depth = stack.iter().filter(|c| matches!(c, Context::BlockQuote)).count();
                        for _ in 0..bq_depth {
                            out.push_str(token);
                        }
                    }
                    out.push_str(&styled);
                }
                Event::Code(code) => {
                    if in_link {
                        link_text_buf.push_str(&code);
                        continue;
                    }
                    if in_image {
                        image_text_buf.push_str(&code);
                        continue;
                    }
                    let code_prim = &self.style.code;
                    let sgr = self.build_sgr(code_prim);
                    let mut rendered = String::new();
                    if let Some(ref pfx) = code_prim.prefix {
                        rendered.push_str(pfx);
                    }
                    rendered.push_str(&code);
                    if let Some(ref sfx) = code_prim.suffix {
                        rendered.push_str(sfx);
                    }
                    out.push_str(&sgr.styled(&rendered));
                }
                Event::SoftBreak => {
                    if in_link {
                        link_text_buf.push(' ');
                        continue;
                    }
                    out.push(' ');
                }
                Event::HardBreak => {
                    if in_link {
                        link_text_buf.push('\n');
                        continue;
                    }
                    out.push('\n');
                }
                Event::Rule => {
                    indent_line(&mut out, margin);
                    if let Some(ref fmt) = self.style.horizontal_rule.format {
                        let sgr = self.build_sgr(&self.style.horizontal_rule);
                        out.push_str(&sgr.styled(fmt));
                    } else {
                        let sgr = self.build_sgr(&self.style.horizontal_rule);
                        out.push_str(&sgr.styled("--------"));
                    }
                    out.push('\n');
                }
                Event::TaskListMarker(checked) => {
                    if checked {
                        if let Some(ref t) = self.style.task.ticked {
                            out.push_str(t);
                        }
                    } else if let Some(ref t) = self.style.task.unticked {
                        out.push_str(t);
                    }
                }
                Event::FootnoteReference(label) => {
                    out.push('[');
                    out.push_str(&label);
                    out.push(']');
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    // Render raw HTML as plain text
                    out.push_str(&html);
                }
                Event::InlineMath(math) => {
                    out.push('$');
                    out.push_str(&math);
                    out.push('$');
                }
                Event::DisplayMath(math) => {
                    out.push_str("$$");
                    out.push_str(&math);
                    out.push_str("$$");
                    out.push('\n');
                }
                // Catch remaining Start/End pairs
                _ => {}
            }
        }

        // Emit document block_suffix
        if let Some(ref bs) = self.style.document.block_suffix {
            out.push_str(bs);
        }

        // Apply word wrap if configured
        if self.word_wrap > 0 {
            rouge_ansi::wordwrap(&out, self.word_wrap)
        } else {
            out
        }
    }

    /// Build an SgrStyle from a StylePrimitive.
    fn build_sgr(&self, prim: &StylePrimitive) -> SgrStyle {
        let mut sgr = SgrStyle::new();
        if prim.bold == Some(true) {
            sgr = sgr.bold();
        }
        if prim.italic == Some(true) {
            sgr = sgr.italic();
        }
        if prim.underline == Some(true) {
            sgr = sgr.underline();
        }
        if prim.strikethrough == Some(true) {
            sgr = sgr.strikethrough();
        }
        if prim.faint == Some(true) {
            sgr = sgr.faint();
        }
        if prim.inverse == Some(true) {
            sgr = sgr.reverse();
        }
        if let Some(ref color) = prim.color {
            sgr = apply_fg_color(sgr, color);
        }
        if let Some(ref bg) = prim.background_color {
            sgr = apply_bg_color(sgr, bg);
        }
        sgr
    }

    /// Build an SgrStyle for a specific heading level, merging the base heading
    /// style with the level-specific overrides.
    fn build_heading_sgr(&self, level: HeadingLevel) -> SgrStyle {
        let base = &self.style.heading;
        let specific = self.heading_style(level);
        let merged = merge_primitives(base, specific);
        self.build_sgr(&merged)
    }

    /// Get the level-specific heading style.
    fn heading_style(&self, level: HeadingLevel) -> &StylePrimitive {
        match level {
            HeadingLevel::H1 => &self.style.h1,
            HeadingLevel::H2 => &self.style.h2,
            HeadingLevel::H3 => &self.style.h3,
            HeadingLevel::H4 => &self.style.h4,
            HeadingLevel::H5 => &self.style.h5,
            HeadingLevel::H6 => &self.style.h6,
        }
    }

    /// Compute block-quote indentation depth from the context stack.
    fn blockquote_indent(&self, stack: &[Context]) -> usize {
        let depth = stack.iter().filter(|c| matches!(c, Context::BlockQuote)).count();
        let per_level = self.style.block_quote.indent.unwrap_or(1) as usize;
        let token_width = self
            .style
            .block_quote
            .indent_token
            .as_ref()
            .map(|t| t.chars().count())
            .unwrap_or(2);
        depth * per_level * token_width
    }

    /// Style a text segment based on the active context stack.
    fn style_text(&self, text: &str, stack: &[Context]) -> String {
        let mut sgr = SgrStyle::new();

        // Apply document-level text color
        if let Some(ref c) = self.style.document.color {
            sgr = apply_fg_color(sgr, c);
        }

        // Walk the stack and apply styles
        for ctx in stack {
            match ctx {
                Context::Emphasis => {
                    let p = &self.style.emph;
                    if p.italic == Some(true) {
                        sgr = sgr.italic();
                    }
                    if let Some(ref c) = p.color {
                        sgr = apply_fg_color(sgr, c);
                    }
                }
                Context::Strong => {
                    let p = &self.style.strong;
                    if p.bold == Some(true) {
                        sgr = sgr.bold();
                    }
                    if let Some(ref c) = p.color {
                        sgr = apply_fg_color(sgr, c);
                    }
                }
                Context::Strikethrough => {
                    let p = &self.style.strikethrough;
                    if p.strikethrough == Some(true) {
                        sgr = sgr.strikethrough();
                    }
                    if let Some(ref c) = p.color {
                        sgr = apply_fg_color(sgr, c);
                    }
                }
                Context::Heading(level) => {
                    let base = &self.style.heading;
                    let specific = self.heading_style(*level);
                    let merged = merge_primitives(base, specific);
                    sgr = self.build_sgr(&merged);
                }
                Context::BlockQuote => {
                    let p = &self.style.block_quote;
                    if p.italic == Some(true) {
                        sgr = sgr.italic();
                    }
                    if let Some(ref c) = p.color {
                        sgr = apply_fg_color(sgr, c);
                    }
                }
                Context::DefinitionListTitle => {
                    let p = &self.style.definition_term;
                    if p.bold == Some(true) {
                        sgr = sgr.bold();
                    }
                    if let Some(ref c) = p.color {
                        sgr = apply_fg_color(sgr, c);
                    }
                }
                _ => {}
            }
        }

        sgr.styled(text)
    }

    /// Syntax-highlight a code block, falling back to plain styled output.
    fn highlight_code(&self, code: &str, lang: Option<&str>) -> String {
        // Determine which syntect theme to use
        let theme_name = self
            .style
            .code_block
            .theme
            .as_deref()
            .unwrap_or(&self.syntax_theme);

        let theme = match self.theme_set.themes.get(theme_name) {
            Some(t) => t,
            None => {
                // Fall back to first available theme or just return plain text
                if let Some(t) = self.theme_set.themes.values().next() {
                    t
                } else {
                    return self.plain_code_block(code);
                }
            }
        };

        // Find syntax by language
        let syntax = lang
            .and_then(|l| self.syntax_set.find_syntax_by_token(l))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut result = String::new();

        for line in code.lines() {
            let line_with_nl = format!("{line}\n");
            match highlighter.highlight_line(&line_with_nl, &self.syntax_set) {
                Ok(ranges) => {
                    result.push_str(&as_24_bit_terminal_escaped(&ranges, false));
                    result.push_str("\x1b[0m");
                }
                Err(_) => {
                    result.push_str(line);
                    result.push('\n');
                }
            }
        }

        result
    }

    /// Render code without syntax highlighting, using the code_block style colors.
    fn plain_code_block(&self, code: &str) -> String {
        let sgr = self.build_sgr(&self.style.code_block.style);
        let mut out = String::new();
        for line in code.lines() {
            out.push_str(&sgr.styled(line));
            out.push('\n');
        }
        out
    }

    /// Render accumulated table data as a simple grid.
    fn render_table(&self, rows: &[Vec<String>], _has_head: bool, margin: usize, out: &mut String) {
        if rows.is_empty() {
            return;
        }

        let col_sep = self.style.table.column_separator.as_deref().unwrap_or("|");
        let row_sep = self.style.table.row_separator.as_deref().unwrap_or("-");
        let center_sep = self.style.table.center_separator.as_deref().unwrap_or("+");

        // Compute column widths
        let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut col_widths = vec![0usize; num_cols];
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < num_cols {
                    col_widths[i] = col_widths[i].max(cell.len());
                }
            }
        }

        // Make sure each column is at least 3 chars wide
        for w in &mut col_widths {
            *w = (*w).max(3);
        }

        let separator_line = |out: &mut String| {
            indent_line(out, margin);
            out.push_str(center_sep);
            for &w in &col_widths {
                for _ in 0..w + 2 {
                    out.push_str(row_sep);
                }
                out.push_str(center_sep);
            }
            out.push('\n');
        };

        separator_line(out);

        for (row_idx, row) in rows.iter().enumerate() {
            indent_line(out, margin);
            out.push_str(col_sep);
            for (i, cell) in row.iter().enumerate() {
                let width = if i < num_cols { col_widths[i] } else { cell.len() };
                out.push(' ');
                out.push_str(cell);
                // Pad
                for _ in cell.len()..width {
                    out.push(' ');
                }
                out.push(' ');
                out.push_str(col_sep);
            }
            out.push('\n');

            // Separator after head row (first row)
            if row_idx == 0 {
                separator_line(out);
            }
        }

        separator_line(out);
    }
}

/// Parse a color string and apply it as a foreground color on the SgrStyle.
/// Supports: "#rrggbb", a decimal 256-color index, or a named ANSI color.
fn apply_fg_color(sgr: SgrStyle, color: &str) -> SgrStyle {
    if let Some(hex) = color.strip_prefix('#')
        && let Some((r, g, b)) = parse_hex_rgb(hex) {
            return sgr.fg_rgb(r, g, b);
        }
    if let Ok(n) = color.parse::<u8>() {
        return sgr.fg_256(n);
    }
    // Unknown color, skip
    sgr
}

/// Parse a color string and apply it as a background color on the SgrStyle.
fn apply_bg_color(sgr: SgrStyle, color: &str) -> SgrStyle {
    if let Some(hex) = color.strip_prefix('#')
        && let Some((r, g, b)) = parse_hex_rgb(hex) {
            return sgr.bg_rgb(r, g, b);
        }
    if let Ok(n) = color.parse::<u8>() {
        return sgr.bg_256(n);
    }
    sgr
}

/// Parse a 6-digit hex color into (r, g, b).
fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Write `n` spaces to the output for indentation.
fn indent_line(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
    }
}

/// Merge two StylePrimitives: child overrides parent (non-None fields win).
fn merge_primitives(parent: &StylePrimitive, child: &StylePrimitive) -> StylePrimitive {
    StylePrimitive {
        prefix: child.prefix.clone().or_else(|| parent.prefix.clone()),
        suffix: child.suffix.clone().or_else(|| parent.suffix.clone()),
        color: child.color.clone().or_else(|| parent.color.clone()),
        background_color: child.background_color.clone().or_else(|| parent.background_color.clone()),
        bold: child.bold.or(parent.bold),
        italic: child.italic.or(parent.italic),
        underline: child.underline.or(parent.underline),
        strikethrough: child.strikethrough.or(parent.strikethrough),
        faint: child.faint.or(parent.faint),
        inverse: child.inverse.or(parent.inverse),
        block_prefix: child.block_prefix.clone().or_else(|| parent.block_prefix.clone()),
        block_suffix: child.block_suffix.clone().or_else(|| parent.block_suffix.clone()),
        indent: child.indent.or(parent.indent),
        indent_token: child.indent_token.clone().or_else(|| parent.indent_token.clone()),
        margin: child.margin.or(parent.margin),
        format: child.format.clone().or_else(|| parent.format.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::dark_theme;

    #[test]
    fn test_render_nonempty() {
        let style = dark_theme();
        let r = TermRenderer::new(style).with_word_wrap(80);
        let output = r.render("# Hello\n\nWorld\n");
        assert!(!output.is_empty());
        // Should contain "Hello" somewhere in the output
        assert!(rouge_ansi::strip_ansi(&output).contains("Hello"));
        assert!(rouge_ansi::strip_ansi(&output).contains("World"));
    }

    #[test]
    fn test_render_code_block() {
        let style = dark_theme();
        let r = TermRenderer::new(style);
        let md = "```rust\nfn main() {}\n```\n";
        let output = r.render(md);
        assert!(!output.is_empty());
        let plain = rouge_ansi::strip_ansi(&output);
        assert!(plain.contains("fn main()"));
    }

    #[test]
    fn test_render_list() {
        let style = dark_theme();
        let r = TermRenderer::new(style);
        let md = "- one\n- two\n- three\n";
        let output = r.render(md);
        let plain = rouge_ansi::strip_ansi(&output);
        assert!(plain.contains("one"));
        assert!(plain.contains("two"));
        assert!(plain.contains("three"));
    }

    #[test]
    fn test_render_table() {
        let style = dark_theme();
        let r = TermRenderer::new(style);
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let output = r.render(md);
        let plain = rouge_ansi::strip_ansi(&output);
        assert!(plain.contains("A"));
        assert!(plain.contains("B"));
        assert!(plain.contains("1"));
        assert!(plain.contains("2"));
    }

    #[test]
    fn test_render_link() {
        let style = dark_theme();
        let r = TermRenderer::new(style);
        let md = "[click here](https://example.com)\n";
        let output = r.render(md);
        let plain = rouge_ansi::strip_ansi(&output);
        assert!(plain.contains("click here"));
        assert!(plain.contains("https://example.com"));
    }

    #[test]
    fn test_render_emphasis_and_strong() {
        let style = dark_theme();
        let r = TermRenderer::new(style);
        let md = "This is *italic* and **bold** text.\n";
        let output = r.render(md);
        let plain = rouge_ansi::strip_ansi(&output);
        assert!(plain.contains("italic"));
        assert!(plain.contains("bold"));
    }

    #[test]
    fn test_parse_hex_rgb() {
        assert_eq!(parse_hex_rgb("ff0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex_rgb("00ff00"), Some((0, 255, 0)));
        assert_eq!(parse_hex_rgb("0000ff"), Some((0, 0, 255)));
        assert_eq!(parse_hex_rgb("abc"), None);
    }

    #[test]
    fn test_style_deserialization() {
        let json = r##"{"bold": true, "color": "#ff0000"}"##;
        let prim: StylePrimitive = serde_json::from_str(json).unwrap();
        assert_eq!(prim.bold, Some(true));
        assert_eq!(prim.color.as_deref(), Some("#ff0000"));
    }
}
