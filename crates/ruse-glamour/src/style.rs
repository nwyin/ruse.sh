use serde::{Deserialize, Serialize};

/// Primitive style properties that can be applied to any markdown element.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StylePrimitive {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub color: Option<String>,
    pub background_color: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    #[serde(alias = "crossed_out")]
    pub strikethrough: Option<bool>,
    pub faint: Option<bool>,
    pub inverse: Option<bool>,
    pub block_prefix: Option<String>,
    pub block_suffix: Option<String>,
    pub indent: Option<u32>,
    pub indent_token: Option<String>,
    pub margin: Option<u32>,
    pub format: Option<String>,
}

/// List styling with level indentation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleList {
    pub level_indent: Option<u32>,
}

/// Style configuration for the entire markdown document.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleConfig {
    pub document: StylePrimitive,
    pub block_quote: StylePrimitive,
    pub paragraph: StylePrimitive,
    pub list: StyleList,
    pub heading: StylePrimitive,
    pub h1: StylePrimitive,
    pub h2: StylePrimitive,
    pub h3: StylePrimitive,
    pub h4: StylePrimitive,
    pub h5: StylePrimitive,
    pub h6: StylePrimitive,
    pub text: StylePrimitive,
    pub strikethrough: StylePrimitive,
    pub emph: StylePrimitive,
    pub strong: StylePrimitive,
    #[serde(alias = "hr")]
    pub horizontal_rule: StylePrimitive,
    pub item: StylePrimitive,
    pub enumeration: StylePrimitive,
    pub task: StyleTask,
    pub link: StylePrimitive,
    pub link_text: StylePrimitive,
    pub image: StylePrimitive,
    pub image_text: StylePrimitive,
    pub code: StylePrimitive,
    pub code_block: StyleCodeBlock,
    pub table: StyleTable,
    pub definition_list: StylePrimitive,
    pub definition_term: StylePrimitive,
    pub definition_description: StylePrimitive,
    pub html_block: StylePrimitive,
    pub html_span: StylePrimitive,
}

/// Task list item styling.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleTask {
    pub ticked: Option<String>,
    pub unticked: Option<String>,
}

/// Code block styling with optional syntax theme.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleCodeBlock {
    #[serde(flatten)]
    pub style: StylePrimitive,
    pub theme: Option<String>,
}

/// Table styling with separator characters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleTable {
    pub center_separator: Option<String>,
    pub column_separator: Option<String>,
    pub row_separator: Option<String>,
}
