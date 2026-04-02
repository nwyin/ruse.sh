use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ruse_runtime::{Cmd, Msg};
use ruse_style::Style;

use crate::key::Binding;

/// Key bindings for the FilePicker component.
pub struct FilePickerKeyMap {
    pub up: Binding,
    pub down: Binding,
    pub open: Binding,
    pub back: Binding,
    pub toggle_hidden: Binding,
}

impl Default for FilePickerKeyMap {
    fn default() -> Self {
        Self {
            up: Binding::new(&["up", "k"], "↑/k", "up"),
            down: Binding::new(&["down", "j"], "↓/j", "down"),
            open: Binding::new(&["right", "enter", "l"], "→/enter", "open"),
            back: Binding::new(&["left", "backspace", "h"], "←/bksp", "back"),
            toggle_hidden: Binding::new(&["."], ".", "toggle hidden"),
        }
    }
}

static FILEPICKER_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    FILEPICKER_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub permissions: Option<u32>,
}

struct ReadDirMsg {
    id: usize,
    entries: Vec<DirEntry>,
}

pub struct FilePickerStyles {
    pub cursor: Style,
    pub directory: Style,
    pub file: Style,
    pub selected: Style,
    pub symlink: Style,
}

impl Default for FilePickerStyles {
    fn default() -> Self {
        Self {
            cursor: Style::new().reverse(true),
            directory: Style::new().bold(true),
            file: Style::new(),
            selected: Style::new().foreground(ruse_style::Color::Rgb {
                r: 100,
                g: 200,
                b: 100,
            }),
            symlink: Style::new().italic(true),
        }
    }
}

pub struct FilePicker {
    pub key_map: FilePickerKeyMap,
    current_dir: PathBuf,
    entries: Vec<DirEntry>,
    cursor: usize,
    selected: Option<PathBuf>,
    height: usize,
    y_offset: usize,
    show_hidden: bool,
    show_permissions: bool,
    show_size: bool,
    allowed_types: Vec<String>,
    pub file_allowed: bool,
    pub dir_allowed: bool,
    styles: FilePickerStyles,
    id: usize,
}

impl FilePicker {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            key_map: FilePickerKeyMap::default(),
            current_dir,
            entries: Vec::new(),
            cursor: 0,
            selected: None,
            height: 15,
            y_offset: 0,
            show_hidden: false,
            show_permissions: false,
            show_size: false,
            allowed_types: Vec::new(),
            file_allowed: true,
            dir_allowed: false,
            styles: FilePickerStyles::default(),
            id: next_id(),
        }
    }

    pub fn with_height(mut self, h: usize) -> Self {
        self.height = h;
        self
    }

    pub fn with_allowed_types(mut self, types: Vec<String>) -> Self {
        self.allowed_types = types;
        self
    }

    pub fn with_show_hidden(mut self, v: bool) -> Self {
        self.show_hidden = v;
        self
    }

    pub fn with_show_permissions(mut self, v: bool) -> Self {
        self.show_permissions = v;
        self
    }

    pub fn with_show_size(mut self, v: bool) -> Self {
        self.show_size = v;
        self
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_dir
    }

    pub fn selected_file(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    /// Read the initial directory. Returns a command that reads the directory
    /// in the background and sends a ReadDirMsg.
    pub fn init(&self) -> Cmd {
        self.read_dir_cmd()
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        // Check for ReadDirMsg
        if let Some(read_dir) = msg.downcast_ref::<ReadDirMsg>() {
            if read_dir.id != self.id {
                return None;
            }
            self.entries = read_dir.entries.clone();
            self.cursor = 0;
            self.y_offset = 0;
            return None;
        }

        if let Msg::KeyPress(key) = msg {
            if self.key_map.up.matches(key) {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.ensure_cursor_visible();
                }
            } else if self.key_map.down.matches(key) {
                if self.cursor < self.entries.len().saturating_sub(1) {
                    self.cursor += 1;
                    self.ensure_cursor_visible();
                }
            } else if self.key_map.open.matches(key) {
                if let Some(entry) = self.entries.get(self.cursor) {
                    if entry.is_dir {
                        self.current_dir = entry.path.clone();
                        return self.read_dir_cmd();
                    } else if self.file_allowed && self.is_allowed(&entry.name) {
                        self.selected = Some(entry.path.clone());
                    }
                }
            } else if self.key_map.back.matches(key) {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    return self.read_dir_cmd();
                }
            } else if self.key_map.toggle_hidden.matches(key) {
                self.show_hidden = !self.show_hidden;
                return self.read_dir_cmd();
            }
        }
        None
    }

    pub fn view(&self) -> String {
        let mut out = String::new();

        // Header: current directory
        let header = format!(" {}", self.current_dir.display());
        out.push_str(&Style::new().bold(true).render(&[&header]));

        let visible_height = self.height.saturating_sub(1); // minus header
        let end = (self.y_offset + visible_height).min(self.entries.len());

        for i in self.y_offset..end {
            out.push('\n');
            let entry = &self.entries[i];

            let icon = if entry.is_dir { "📁 " } else { "  " };

            let perms_str = if self.show_permissions {
                if let Some(mode) = entry.permissions {
                    format!("{} ", format_permissions(mode))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let name = if entry.is_dir {
                format!("{}{}{}/", perms_str, icon, entry.name)
            } else {
                let size_str = if self.show_size {
                    format!(" ({})", human_size(entry.size))
                } else {
                    String::new()
                };
                format!("{}{}{}{}", perms_str, icon, entry.name, size_str)
            };

            let style = if i == self.cursor {
                &self.styles.cursor
            } else if entry.is_dir {
                &self.styles.directory
            } else {
                &self.styles.file
            };

            out.push_str(&style.render(&[&name]));
        }

        // Pad remaining height
        for _ in (end.saturating_sub(self.y_offset))..visible_height {
            out.push('\n');
        }

        out
    }

    fn read_dir_cmd(&self) -> Cmd {
        let dir = self.current_dir.clone();
        let id = self.id;
        let show_hidden = self.show_hidden;
        ruse_runtime::cmd(move || {
            let mut entries = Vec::new();

            if let Ok(read_dir) = std::fs::read_dir(&dir) {
                for entry in read_dir.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files unless show_hidden
                    if !show_hidden && name.starts_with('.') {
                        continue;
                    }

                    let metadata = entry.metadata().ok();
                    let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                    #[cfg(unix)]
                    let permissions = metadata.as_ref().map(|m| {
                        use std::os::unix::fs::PermissionsExt;
                        m.permissions().mode()
                    });
                    #[cfg(not(unix))]
                    let permissions = None;

                    entries.push(DirEntry {
                        name,
                        path: entry.path(),
                        is_dir,
                        size,
                        permissions,
                    });
                }
            }

            // Sort: directories first, then alphabetically
            entries.sort_by(|a, b| {
                if a.is_dir == b.is_dir {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                } else if a.is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            });

            Msg::custom(ReadDirMsg { id, entries })
        })
    }

    fn is_allowed(&self, name: &str) -> bool {
        if self.allowed_types.is_empty() {
            return true;
        }
        let name_lower = name.to_lowercase();
        self.allowed_types
            .iter()
            .any(|ext| name_lower.ends_with(ext))
    }

    fn ensure_cursor_visible(&mut self) {
        let visible = self.height.saturating_sub(1);
        if visible == 0 {
            return;
        }
        if self.cursor < self.y_offset {
            self.y_offset = self.cursor;
        } else if self.cursor >= self.y_offset + visible {
            self.y_offset = self.cursor - visible + 1;
        }
    }
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}

fn format_permissions(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    let flags = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    for &(bit, ch) in &flags {
        if mode & bit != 0 {
            s.push(ch);
        } else {
            s.push('-');
        }
    }
    s
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    for unit in UNITS {
        if size < 1024.0 {
            return format!("{:.1}{}", size, unit);
        }
        size /= 1024.0;
    }
    format!("{:.1}PB", size)
}
