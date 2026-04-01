/// Tree/list rendering with configurable enumerators.
/// A tree node: a value string with optional children.
pub struct Tree {
    pub value: String,
    pub children: Vec<Tree>,
}

/// Controls the bullet/prefix style used when rendering a [`Tree`].
pub enum Enumerator {
    /// Box-drawing: `├──` / `└──`
    Default,
    /// Rounded box-drawing: `├──` / `╰──`
    Rounded,
    /// Bullet: `•`
    Bullet,
    /// Dash: `-`
    Dash,
    /// Arabic numerals: `1.`, `2.`, `3.`
    Arabic,
    /// Roman numerals: `I.`, `II.`, `III.`
    Roman,
    /// Lowercase alphabet: `a.`, `b.`, ... `z.`, `aa.`, `ab.`
    Alphabet,
    /// Asterisk: `*`
    Asterisk,
    /// Custom function `(index, is_last) -> prefix_string`.
    Custom(Box<dyn Fn(usize, bool) -> String + Send + Sync>),
}

impl Tree {
    /// Create a leaf node with the given value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            children: Vec::new(),
        }
    }

    /// Attach children to this node, consuming and returning self.
    pub fn with_children(mut self, children: Vec<Tree>) -> Self {
        self.children = children;
        self
    }

    /// Render the tree to a string using the given enumerator style.
    pub fn render(&self, enumerator: &Enumerator) -> String {
        let mut out = String::new();
        out.push_str(&self.value);
        render_children(&mut out, &self.children, enumerator, "");
        out
    }
}

/// Recursively render `children` into `out`, accumulating `prefix` for indentation.
fn render_children(out: &mut String, children: &[Tree], enumerator: &Enumerator, prefix: &str) {
    let len = children.len();
    for (i, child) in children.iter().enumerate() {
        let is_last = i == len - 1;
        out.push('\n');

        let (bullet, child_prefix) = make_prefixes(enumerator, i, is_last);

        out.push_str(prefix);
        out.push_str(&bullet);
        out.push_str(&child.value);

        let next_prefix = format!("{prefix}{child_prefix}");
        render_children(out, &child.children, enumerator, &next_prefix);
    }
}

/// Return `(bullet_prefix, continuation_prefix)` for a child at `index`.
fn make_prefixes(enumerator: &Enumerator, index: usize, is_last: bool) -> (String, String) {
    match enumerator {
        Enumerator::Default => {
            if is_last {
                ("└── ".to_string(), "    ".to_string())
            } else {
                ("├── ".to_string(), "│   ".to_string())
            }
        }
        Enumerator::Rounded => {
            if is_last {
                ("╰── ".to_string(), "    ".to_string())
            } else {
                ("├── ".to_string(), "│   ".to_string())
            }
        }
        Enumerator::Bullet => ("• ".to_string(), "  ".to_string()),
        Enumerator::Dash => ("- ".to_string(), "  ".to_string()),
        Enumerator::Asterisk => ("* ".to_string(), "  ".to_string()),
        Enumerator::Arabic => {
            let label = format!("{}. ", index + 1);
            let pad = " ".repeat(label.len());
            (label, pad)
        }
        Enumerator::Roman => {
            let label = format!("{}. ", to_roman(index + 1));
            let pad = " ".repeat(label.len());
            (label, pad)
        }
        Enumerator::Alphabet => {
            let label = format!("{}. ", to_alphabet(index));
            let pad = " ".repeat(label.len());
            (label, pad)
        }
        Enumerator::Custom(f) => {
            let label = f(index, is_last);
            let pad = " ".repeat(label.len());
            (label, pad)
        }
    }
}

/// Convert a 1-based number to upper-case Roman numerals using the
/// subtractive algorithm.
fn to_roman(mut n: usize) -> String {
    const TABLE: &[(usize, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for &(value, sym) in TABLE {
        while n >= value {
            out.push_str(sym);
            n -= value;
        }
    }
    out
}

/// Convert a 0-based index to a lowercase alphabetic label:
/// 0 → `a`, 25 → `z`, 26 → `aa`, 27 → `ab`, …
fn to_alphabet(mut n: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'a' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_enumerator() {
        let tree = Tree::new("root").with_children(vec![
            Tree::new("a").with_children(vec![Tree::new("a1"), Tree::new("a2")]),
            Tree::new("b"),
        ]);
        let result = tree.render(&Enumerator::Default);
        let expected = "\
root
├── a
│   ├── a1
│   └── a2
└── b";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rounded_enumerator() {
        let tree = Tree::new("root").with_children(vec![Tree::new("a"), Tree::new("b")]);
        let result = tree.render(&Enumerator::Rounded);
        let expected = "\
root
├── a
╰── b";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_bullet_enumerator() {
        let tree =
            Tree::new("list").with_children(vec![Tree::new("x"), Tree::new("y"), Tree::new("z")]);
        let result = tree.render(&Enumerator::Bullet);
        let expected = "\
list
• x
• y
• z";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_arabic_enumerator() {
        let tree = Tree::new("items").with_children(vec![
            Tree::new("first"),
            Tree::new("second"),
            Tree::new("third"),
        ]);
        let result = tree.render(&Enumerator::Arabic);
        let expected = "\
items
1. first
2. second
3. third";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_roman_enumerator() {
        let tree = Tree::new("steps").with_children(vec![
            Tree::new("one"),
            Tree::new("two"),
            Tree::new("three"),
            Tree::new("four"),
        ]);
        let result = tree.render(&Enumerator::Roman);
        let expected = "\
steps
I. one
II. two
III. three
IV. four";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_alphabet_enumerator() {
        // Build 28 children to test wrap-around: a..z, aa, ab
        let children: Vec<Tree> = (0..28).map(|i| Tree::new(format!("item{i}"))).collect();
        let tree = Tree::new("alpha").with_children(children);
        let result = tree.render(&Enumerator::Alphabet);
        let lines: Vec<&str> = result.lines().collect();
        // First child: "a. item0"
        assert_eq!(lines[1], "a. item0");
        // 26th child (index 25): "z. item25"
        assert_eq!(lines[26], "z. item25");
        // 27th child (index 26): "aa. item26"
        assert_eq!(lines[27], "aa. item26");
        // 28th child (index 27): "ab. item27"
        assert_eq!(lines[28], "ab. item27");
    }

    #[test]
    fn test_dash_enumerator() {
        let tree = Tree::new("root").with_children(vec![Tree::new("a"), Tree::new("b")]);
        let result = tree.render(&Enumerator::Dash);
        let expected = "\
root
- a
- b";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asterisk_enumerator() {
        let tree = Tree::new("root").with_children(vec![Tree::new("a"), Tree::new("b")]);
        let result = tree.render(&Enumerator::Asterisk);
        let expected = "\
root
* a
* b";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_custom_enumerator() {
        let tree = Tree::new("root").with_children(vec![Tree::new("a"), Tree::new("b")]);
        let result = tree.render(&Enumerator::Custom(Box::new(|i, _last| {
            format!("[{}] ", i)
        })));
        let expected = "\
root
[0] a
[1] b";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_leaf_renders_value_only() {
        let tree = Tree::new("leaf");
        assert_eq!(tree.render(&Enumerator::Default), "leaf");
    }

    #[test]
    fn test_nested_arabic() {
        let tree = Tree::new("root").with_children(vec![
            Tree::new("a").with_children(vec![Tree::new("a1"), Tree::new("a2")]),
        ]);
        let result = tree.render(&Enumerator::Arabic);
        let expected = "\
root
1. a
   1. a1
   2. a2";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_to_roman_basic() {
        assert_eq!(to_roman(1), "I");
        assert_eq!(to_roman(4), "IV");
        assert_eq!(to_roman(9), "IX");
        assert_eq!(to_roman(14), "XIV");
        assert_eq!(to_roman(42), "XLII");
        assert_eq!(to_roman(1994), "MCMXCIV");
    }

    #[test]
    fn test_to_alphabet_basic() {
        assert_eq!(to_alphabet(0), "a");
        assert_eq!(to_alphabet(25), "z");
        assert_eq!(to_alphabet(26), "aa");
        assert_eq!(to_alphabet(27), "ab");
        assert_eq!(to_alphabet(51), "az");
        assert_eq!(to_alphabet(52), "ba");
    }
}
