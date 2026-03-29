use ruse_runtime::{Cmd, Msg};

use crate::key::Binding;

/// Key bindings for the Paginator component.
pub struct PaginatorKeyMap {
    pub prev_page: Binding,
    pub next_page: Binding,
}

impl Default for PaginatorKeyMap {
    fn default() -> Self {
        Self {
            prev_page: Binding::new(&["left", "h", "pgup"], "←/h", "prev page"),
            next_page: Binding::new(&["right", "l", "pgdn"], "→/l", "next page"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PaginatorType {
    Arabic,
    Dots,
}

pub struct Paginator {
    pub key_map: PaginatorKeyMap,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub display_type: PaginatorType,
    pub active_dot: String,
    pub inactive_dot: String,
}

impl Default for Paginator {
    fn default() -> Self {
        Self::new()
    }
}

impl Paginator {
    pub fn new() -> Self {
        Self {
            key_map: PaginatorKeyMap::default(),
            page: 0,
            per_page: 1,
            total_pages: 1,
            display_type: PaginatorType::Arabic,
            active_dot: "●".to_string(),
            inactive_dot: "○".to_string(),
        }
    }

    /// Compute total pages from total item count, using per_page.
    pub fn set_total_pages(&mut self, total_items: usize) {
        if self.per_page < 1 {
            self.total_pages = 1;
            return;
        }
        self.total_pages = total_items.div_ceil(self.per_page);
        if self.total_pages < 1 {
            self.total_pages = 1;
        }
        // Clamp current page
        if self.page >= self.total_pages {
            self.page = self.total_pages - 1;
        }
    }

    /// Number of items visible on the current page.
    pub fn items_on_page(&self, total_items: usize) -> usize {
        if self.on_last_page() {
            let remainder = total_items % self.per_page;
            if remainder == 0 {
                self.per_page
            } else {
                remainder
            }
        } else {
            self.per_page
        }
    }

    /// Return (start, end) slice bounds for the current page.
    pub fn slice_bounds(&self, length: usize) -> (usize, usize) {
        let start = self.page * self.per_page;
        let end = (start + self.per_page).min(length);
        (start, end)
    }

    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    pub fn next_page(&mut self) {
        if self.page < self.total_pages.saturating_sub(1) {
            self.page += 1;
        }
    }

    pub fn on_first_page(&self) -> bool {
        self.page == 0
    }

    pub fn on_last_page(&self) -> bool {
        self.page >= self.total_pages.saturating_sub(1)
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if let Msg::KeyPress(key) = msg {
            if self.key_map.prev_page.matches(key) {
                self.prev_page();
            } else if self.key_map.next_page.matches(key) {
                self.next_page();
            }
        }
        None
    }

    pub fn view(&self) -> String {
        match self.display_type {
            PaginatorType::Arabic => {
                format!("{}/{}", self.page + 1, self.total_pages)
            }
            PaginatorType::Dots => {
                let mut out = String::new();
                for i in 0..self.total_pages {
                    if i > 0 {
                        out.push(' ');
                    }
                    if i == self.page {
                        out.push_str(&self.active_dot);
                    } else {
                        out.push_str(&self.inactive_dot);
                    }
                }
                out
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_total_pages() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(25);
        assert_eq!(p.total_pages, 3);
    }

    #[test]
    fn test_set_total_pages_exact() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(20);
        assert_eq!(p.total_pages, 2);
    }

    #[test]
    fn test_set_total_pages_zero() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(0);
        assert_eq!(p.total_pages, 1);
    }

    #[test]
    fn test_items_on_page_full() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(25);
        p.page = 0;
        assert_eq!(p.items_on_page(25), 10);
    }

    #[test]
    fn test_items_on_page_last() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(25);
        p.page = 2;
        assert_eq!(p.items_on_page(25), 5);
    }

    #[test]
    fn test_slice_bounds() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(25);
        p.page = 1;
        let (start, end) = p.slice_bounds(25);
        assert_eq!(start, 10);
        assert_eq!(end, 20);
    }

    #[test]
    fn test_slice_bounds_last_page() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(25);
        p.page = 2;
        let (start, end) = p.slice_bounds(25);
        assert_eq!(start, 20);
        assert_eq!(end, 25);
    }

    #[test]
    fn test_navigation() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(30);
        assert_eq!(p.page, 0);
        assert!(p.on_first_page());

        p.next_page();
        assert_eq!(p.page, 1);
        assert!(!p.on_first_page());
        assert!(!p.on_last_page());

        p.next_page();
        assert_eq!(p.page, 2);
        assert!(p.on_last_page());

        p.next_page();
        assert_eq!(p.page, 2); // no change

        p.prev_page();
        assert_eq!(p.page, 1);

        p.prev_page();
        p.prev_page();
        assert_eq!(p.page, 0); // clamped
    }

    #[test]
    fn test_view_arabic() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(25);
        p.page = 1;
        assert_eq!(p.view(), "2/3");
    }

    #[test]
    fn test_view_dots() {
        let mut p = Paginator::new();
        p.display_type = PaginatorType::Dots;
        p.per_page = 10;
        p.set_total_pages(30);
        p.page = 1;
        assert_eq!(p.view(), "○ ● ○");
    }

    #[test]
    fn test_clamp_page_on_total_change() {
        let mut p = Paginator::new();
        p.per_page = 10;
        p.set_total_pages(50);
        p.page = 4;
        p.set_total_pages(20);
        assert_eq!(p.page, 1); // clamped from 4 to 1
    }
}
