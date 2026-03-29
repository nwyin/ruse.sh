use crate::style::{StyleCodeBlock, StyleConfig, StylePrimitive, StyleTable, StyleTask};

fn sp() -> StylePrimitive {
    StylePrimitive::default()
}

fn prim(f: impl FnOnce(&mut StylePrimitive)) -> StylePrimitive {
    let mut s = sp();
    f(&mut s);
    s
}

/// Dark background terminal theme, based on the Glamour dark style.
pub fn dark_theme() -> StyleConfig {
    StyleConfig {
        document: prim(|s| {
            s.block_prefix = Some("\n".into());
            s.block_suffix = Some("\n".into());
            s.color = Some("252".into());
            s.margin = Some(2);
        }),
        block_quote: prim(|s| {
            s.indent = Some(1);
            s.indent_token = Some("\u{2502} ".into());
        }),
        paragraph: sp(),
        heading: prim(|s| {
            s.block_suffix = Some("\n".into());
            s.color = Some("39".into());
            s.bold = Some(true);
        }),
        h1: prim(|s| {
            s.prefix = Some(" ".into());
            s.suffix = Some(" ".into());
            s.color = Some("228".into());
            s.background_color = Some("63".into());
            s.bold = Some(true);
        }),
        h2: prim(|s| {
            s.prefix = Some("## ".into());
        }),
        h3: prim(|s| {
            s.prefix = Some("### ".into());
        }),
        h4: prim(|s| {
            s.prefix = Some("#### ".into());
        }),
        h5: prim(|s| {
            s.prefix = Some("##### ".into());
        }),
        h6: prim(|s| {
            s.prefix = Some("###### ".into());
            s.color = Some("35".into());
            s.bold = Some(false);
        }),
        text: sp(),
        strikethrough: prim(|s| {
            s.strikethrough = Some(true);
        }),
        emph: prim(|s| {
            s.italic = Some(true);
        }),
        strong: prim(|s| {
            s.bold = Some(true);
        }),
        horizontal_rule: prim(|s| {
            s.color = Some("240".into());
            s.format = Some("\n--------\n".into());
        }),
        item: prim(|s| {
            s.block_prefix = Some("\u{2022} ".into());
        }),
        enumeration: prim(|s| {
            s.block_prefix = Some(". ".into());
        }),
        task: StyleTask {
            ticked: Some("[\u{2713}] ".into()),
            unticked: Some("[ ] ".into()),
        },
        link: prim(|s| {
            s.color = Some("30".into());
            s.underline = Some(true);
        }),
        link_text: prim(|s| {
            s.color = Some("35".into());
            s.bold = Some(true);
        }),
        image: prim(|s| {
            s.color = Some("212".into());
            s.underline = Some(true);
        }),
        image_text: prim(|s| {
            s.color = Some("243".into());
        }),
        code: prim(|s| {
            s.prefix = Some(" ".into());
            s.suffix = Some(" ".into());
            s.color = Some("203".into());
            s.background_color = Some("236".into());
        }),
        code_block: StyleCodeBlock {
            style: prim(|s| {
                s.color = Some("244".into());
                s.margin = Some(2);
            }),
            theme: Some("base16-ocean.dark".into()),
        },
        table: StyleTable {
            center_separator: Some("+".into()),
            column_separator: Some("|".into()),
            row_separator: Some("-".into()),
        },
        definition_list: sp(),
        definition_term: sp(),
        definition_description: prim(|s| {
            s.block_prefix = Some("\n\u{1F836} ".into());
        }),
        html_block: sp(),
        html_span: sp(),
    }
}

/// Light background terminal theme, based on the Glamour light style.
pub fn light_theme() -> StyleConfig {
    StyleConfig {
        document: prim(|s| {
            s.block_prefix = Some("\n".into());
            s.block_suffix = Some("\n".into());
            s.color = Some("234".into());
            s.margin = Some(2);
        }),
        block_quote: prim(|s| {
            s.indent = Some(1);
            s.indent_token = Some("\u{2502} ".into());
        }),
        paragraph: sp(),
        heading: prim(|s| {
            s.block_suffix = Some("\n".into());
            s.color = Some("27".into());
            s.bold = Some(true);
        }),
        h1: prim(|s| {
            s.prefix = Some(" ".into());
            s.suffix = Some(" ".into());
            s.color = Some("228".into());
            s.background_color = Some("63".into());
            s.bold = Some(true);
        }),
        h2: prim(|s| {
            s.prefix = Some("## ".into());
        }),
        h3: prim(|s| {
            s.prefix = Some("### ".into());
        }),
        h4: prim(|s| {
            s.prefix = Some("#### ".into());
        }),
        h5: prim(|s| {
            s.prefix = Some("##### ".into());
        }),
        h6: prim(|s| {
            s.prefix = Some("###### ".into());
            s.bold = Some(false);
        }),
        text: sp(),
        strikethrough: prim(|s| {
            s.strikethrough = Some(true);
        }),
        emph: prim(|s| {
            s.italic = Some(true);
        }),
        strong: prim(|s| {
            s.bold = Some(true);
        }),
        horizontal_rule: prim(|s| {
            s.color = Some("249".into());
            s.format = Some("\n--------\n".into());
        }),
        item: prim(|s| {
            s.block_prefix = Some("\u{2022} ".into());
        }),
        enumeration: prim(|s| {
            s.block_prefix = Some(". ".into());
        }),
        task: StyleTask {
            ticked: Some("[\u{2713}] ".into()),
            unticked: Some("[ ] ".into()),
        },
        link: prim(|s| {
            s.color = Some("36".into());
            s.underline = Some(true);
        }),
        link_text: prim(|s| {
            s.color = Some("29".into());
            s.bold = Some(true);
        }),
        image: prim(|s| {
            s.color = Some("205".into());
            s.underline = Some(true);
        }),
        image_text: prim(|s| {
            s.color = Some("243".into());
        }),
        code: prim(|s| {
            s.prefix = Some(" ".into());
            s.suffix = Some(" ".into());
            s.color = Some("203".into());
            s.background_color = Some("254".into());
        }),
        code_block: StyleCodeBlock {
            style: prim(|s| {
                s.color = Some("242".into());
                s.margin = Some(2);
            }),
            theme: Some("base16-ocean.light".into()),
        },
        table: StyleTable {
            center_separator: Some("+".into()),
            column_separator: Some("|".into()),
            row_separator: Some("-".into()),
        },
        definition_list: sp(),
        definition_term: sp(),
        definition_description: prim(|s| {
            s.block_prefix = Some("\n\u{1F836} ".into());
        }),
        html_block: sp(),
        html_span: sp(),
    }
}

/// Dracula color scheme theme.
pub fn dracula_theme() -> StyleConfig {
    StyleConfig {
        document: prim(|s| {
            s.block_prefix = Some("\n".into());
            s.block_suffix = Some("\n".into());
            s.color = Some("#f8f8f2".into());
            s.margin = Some(2);
        }),
        block_quote: prim(|s| {
            s.color = Some("#f1fa8c".into());
            s.italic = Some(true);
            s.indent = Some(2);
        }),
        paragraph: sp(),
        heading: prim(|s| {
            s.block_suffix = Some("\n".into());
            s.color = Some("#bd93f9".into());
            s.bold = Some(true);
        }),
        h1: prim(|s| {
            s.prefix = Some("# ".into());
        }),
        h2: prim(|s| {
            s.prefix = Some("## ".into());
        }),
        h3: prim(|s| {
            s.prefix = Some("### ".into());
        }),
        h4: prim(|s| {
            s.prefix = Some("#### ".into());
        }),
        h5: prim(|s| {
            s.prefix = Some("##### ".into());
        }),
        h6: prim(|s| {
            s.prefix = Some("###### ".into());
        }),
        text: sp(),
        strikethrough: prim(|s| {
            s.strikethrough = Some(true);
        }),
        emph: prim(|s| {
            s.color = Some("#f1fa8c".into());
            s.italic = Some(true);
        }),
        strong: prim(|s| {
            s.color = Some("#ffb86c".into());
            s.bold = Some(true);
        }),
        horizontal_rule: prim(|s| {
            s.color = Some("#6272A4".into());
            s.format = Some("\n--------\n".into());
        }),
        item: prim(|s| {
            s.block_prefix = Some("\u{2022} ".into());
        }),
        enumeration: prim(|s| {
            s.block_prefix = Some(". ".into());
            s.color = Some("#8be9fd".into());
        }),
        task: StyleTask {
            ticked: Some("[\u{2713}] ".into()),
            unticked: Some("[ ] ".into()),
        },
        link: prim(|s| {
            s.color = Some("#8be9fd".into());
            s.underline = Some(true);
        }),
        link_text: prim(|s| {
            s.color = Some("#ff79c6".into());
        }),
        image: prim(|s| {
            s.color = Some("#8be9fd".into());
            s.underline = Some(true);
        }),
        image_text: prim(|s| {
            s.color = Some("#ff79c6".into());
        }),
        code: prim(|s| {
            s.color = Some("#50fa7b".into());
        }),
        code_block: StyleCodeBlock {
            style: prim(|s| {
                s.color = Some("#ffb86c".into());
                s.margin = Some(2);
            }),
            theme: Some("base16-ocean.dark".into()),
        },
        table: StyleTable {
            center_separator: Some("+".into()),
            column_separator: Some("|".into()),
            row_separator: Some("-".into()),
        },
        definition_list: sp(),
        definition_term: sp(),
        definition_description: prim(|s| {
            s.block_prefix = Some("\n\u{1F836} ".into());
        }),
        html_block: sp(),
        html_span: sp(),
    }
}

/// Tokyo Night color scheme theme.
pub fn tokyo_night_theme() -> StyleConfig {
    StyleConfig {
        document: prim(|s| {
            s.block_prefix = Some("\n".into());
            s.block_suffix = Some("\n".into());
            s.color = Some("#a9b1d6".into());
            s.margin = Some(2);
        }),
        block_quote: prim(|s| {
            s.indent = Some(1);
            s.indent_token = Some("\u{2502} ".into());
        }),
        paragraph: sp(),
        heading: prim(|s| {
            s.block_suffix = Some("\n".into());
            s.color = Some("#bb9af7".into());
            s.bold = Some(true);
        }),
        h1: prim(|s| {
            s.prefix = Some("# ".into());
            s.bold = Some(true);
        }),
        h2: prim(|s| {
            s.prefix = Some("## ".into());
        }),
        h3: prim(|s| {
            s.prefix = Some("### ".into());
        }),
        h4: prim(|s| {
            s.prefix = Some("#### ".into());
        }),
        h5: prim(|s| {
            s.prefix = Some("##### ".into());
        }),
        h6: prim(|s| {
            s.prefix = Some("###### ".into());
        }),
        text: sp(),
        strikethrough: prim(|s| {
            s.strikethrough = Some(true);
        }),
        emph: prim(|s| {
            s.italic = Some(true);
        }),
        strong: prim(|s| {
            s.bold = Some(true);
        }),
        horizontal_rule: prim(|s| {
            s.color = Some("#565f89".into());
            s.format = Some("\n--------\n".into());
        }),
        item: prim(|s| {
            s.block_prefix = Some("\u{2022} ".into());
        }),
        enumeration: prim(|s| {
            s.block_prefix = Some(". ".into());
            s.color = Some("#7aa2f7".into());
        }),
        task: StyleTask {
            ticked: Some("[\u{2713}] ".into()),
            unticked: Some("[ ] ".into()),
        },
        link: prim(|s| {
            s.color = Some("#7aa2f7".into());
            s.underline = Some(true);
        }),
        link_text: prim(|s| {
            s.color = Some("#2ac3de".into());
        }),
        image: prim(|s| {
            s.color = Some("#7aa2f7".into());
            s.underline = Some(true);
        }),
        image_text: prim(|s| {
            s.color = Some("#2ac3de".into());
        }),
        code: prim(|s| {
            s.color = Some("#9ece6a".into());
        }),
        code_block: StyleCodeBlock {
            style: prim(|s| {
                s.color = Some("#ff9e64".into());
                s.margin = Some(2);
            }),
            theme: Some("base16-ocean.dark".into()),
        },
        table: StyleTable {
            center_separator: Some("+".into()),
            column_separator: Some("|".into()),
            row_separator: Some("-".into()),
        },
        definition_list: sp(),
        definition_term: sp(),
        definition_description: prim(|s| {
            s.block_prefix = Some("\n\u{1F836} ".into());
        }),
        html_block: sp(),
        html_span: sp(),
    }
}

/// Look up a built-in theme by name. Falls back to dark if not found.
pub fn get_theme(name: &str) -> StyleConfig {
    match name {
        "dark" => dark_theme(),
        "light" => light_theme(),
        "dracula" => dracula_theme(),
        "tokyo-night" | "tokyo_night" => tokyo_night_theme(),
        _ => dark_theme(),
    }
}
