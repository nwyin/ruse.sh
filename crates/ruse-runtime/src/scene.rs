use ruse_ansi::Rect;

use crate::cmd::{Cmd, CmdInner, batch};
use crate::msg::Msg;
use crate::pane::{Pane, PaneAny, PaneMsg};
use crate::view::View;

/// Layout and visibility for a pane within a Scene.
#[derive(Debug, Clone)]
pub struct PaneLayout {
    pub rect: Rect,
    pub z: i32,
    pub visible: bool,
}

impl PaneLayout {
    pub fn new(rect: Rect, z: i32) -> Self {
        Self {
            rect,
            z,
            visible: true,
        }
    }
}

struct PaneEntry {
    id: String,
    pane: Box<dyn PaneAny>,
    layout: PaneLayout,
}

/// Manages multiple independent [`Pane`]s with automatic message routing,
/// command tagging, focus management, and region-based view composition.
///
/// # Message routing
///
/// - **Input** (keyboard, paste) goes to the focused pane only.
/// - **Mouse** events are hit-tested against pane rects (highest z wins).
/// - **Background** events (timers, async results, resize) are broadcast
///   to all visible panes so they keep updating even when unfocused.
/// - **Command results** are tagged with the originating pane's ID and
///   routed back to that pane only.
///
/// # Example
///
/// ```ignore
/// struct App { scene: Scene }
///
/// impl Model for App {
///     fn init(&mut self) -> Cmd { self.scene.init_all() }
///     fn update(&mut self, msg: Msg) -> Cmd { self.scene.update(&msg) }
///     fn view(&self) -> View { self.scene.view() }
/// }
/// ```
pub struct Scene {
    panes: Vec<PaneEntry>,
    focus: Option<String>,
    click_focuses: bool,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            panes: Vec::new(),
            focus: None,
            click_focuses: true,
        }
    }

    /// Whether mouse clicks automatically shift focus to the clicked pane.
    /// Default: true.
    pub fn with_click_focus(mut self, enabled: bool) -> Self {
        self.click_focuses = enabled;
        self
    }

    /// Add a pane with the given id and layout. Calls `pane.init()`.
    pub fn add(&mut self, id: impl Into<String>, pane: impl Pane, layout: PaneLayout) -> Cmd {
        let id = id.into();
        let mut entry = PaneEntry {
            id: id.clone(),
            pane: Box::new(pane),
            layout,
        };
        let cmd = entry.pane.init();
        self.panes.push(entry);
        wrap_cmd(id, cmd)
    }

    /// Remove a pane by id.
    pub fn remove(&mut self, id: &str) -> bool {
        let len = self.panes.len();
        self.panes.retain(|e| e.id != id);
        if self.focus.as_deref() == Some(id) {
            self.focus = None;
        }
        self.panes.len() < len
    }

    /// Initialize all panes. Call from `Model::init()`.
    pub fn init_all(&mut self) -> Cmd {
        let results: Vec<(String, Cmd)> = self
            .panes
            .iter_mut()
            .map(|e| (e.id.clone(), e.pane.init()))
            .collect();
        let cmds = results
            .into_iter()
            .map(|(id, cmd)| wrap_cmd(id, cmd))
            .collect();
        batch(cmds)
    }

    /// Set which pane has input focus. Calls blur on the old pane and
    /// focus on the new one. Returns batched commands from both calls.
    pub fn set_focus(&mut self, id: impl Into<String>) -> Cmd {
        let id = id.into();
        let mut cmds = Vec::new();

        // Blur the currently focused pane
        if let Some(ref old_id) = self.focus
            && *old_id != id
        {
            let old_id_clone = old_id.clone();
            if let Some(entry) = self.panes.iter_mut().find(|e| e.id == old_id_clone) {
                entry.pane.blur();
            }
        }

        // Focus the new pane
        self.focus = Some(id.clone());
        if let Some(entry) = self.panes.iter_mut().find(|e| e.id == id) {
            let cmd = entry.pane.focus();
            cmds.push(wrap_cmd(id, cmd));
        }

        batch(cmds)
    }

    /// Clear focus so no pane receives input.
    pub fn clear_focus(&mut self) -> Cmd {
        if let Some(ref old_id) = self.focus.take()
            && let Some(entry) = self.panes.iter_mut().find(|e| e.id == *old_id)
        {
            entry.pane.blur();
        }
        None
    }

    /// The currently focused pane's id.
    pub fn focused(&self) -> Option<&str> {
        self.focus.as_deref()
    }

    /// Update the layout for a specific pane.
    pub fn set_layout(&mut self, id: &str, layout: PaneLayout) {
        if let Some(entry) = self.panes.iter_mut().find(|e| e.id == id) {
            entry.layout = layout;
        }
    }

    /// Get a reference to a pane, downcast to a concrete type.
    pub fn pane_as<T: 'static>(&self, id: &str) -> Option<&T> {
        self.panes
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.pane.as_any().downcast_ref::<T>())
    }

    /// Get a mutable reference to a pane, downcast to a concrete type.
    pub fn pane_as_mut<T: 'static>(&mut self, id: &str) -> Option<&mut T> {
        self.panes
            .iter_mut()
            .find(|e| e.id == id)
            .and_then(|e| e.pane.as_any_mut().downcast_mut::<T>())
    }

    /// Whether a pane with the given id exists.
    pub fn contains(&self, id: &str) -> bool {
        self.panes.iter().any(|e| e.id == id)
    }

    /// Iterate over pane ids.
    pub fn pane_ids(&self) -> impl Iterator<Item = &str> {
        self.panes.iter().map(|e| e.id.as_str())
    }

    /// Route a message to the appropriate pane(s) and return any resulting commands.
    pub fn update(&mut self, msg: &Msg) -> Cmd {
        // Tagged pane result — route to originating pane only
        if let Some(pane_msg) = msg.downcast_ref::<PaneMsg>() {
            return self.route_to_pane(&pane_msg.pane_id, &pane_msg.inner);
        }

        match msg {
            // Input → focused pane only
            Msg::KeyPress(_) | Msg::KeyRelease(_) | Msg::Paste(_) => self.route_to_focused(msg),

            // Terminal focus → focused pane only
            Msg::Focus | Msg::Blur => self.route_to_focused(msg),

            // Mouse click/release → hit-test, optionally shift focus
            Msg::MouseClick(mouse) | Msg::MouseRelease(mouse) => {
                let target = self.hit_test(mouse.x, mouse.y);
                if self.click_focuses
                    && matches!(msg, Msg::MouseClick(_))
                    && let Some(ref id) = target
                    && self.focus.as_deref() != Some(id)
                {
                    let focus_cmd = self.set_focus(id.clone());
                    let route_cmd = self.route_to_pane(id, msg);
                    return batch(vec![focus_cmd, route_cmd]);
                }
                match target {
                    Some(id) => self.route_to_pane(&id, msg),
                    None => None,
                }
            }

            // Mouse wheel/motion → hit-test
            Msg::MouseWheel(mouse) | Msg::MouseMotion(mouse) => {
                match self.hit_test(mouse.x, mouse.y) {
                    Some(id) => self.route_to_pane(&id, msg),
                    None => None,
                }
            }

            // Broadcast to all visible panes
            Msg::WindowSize { .. } | Msg::Resume | Msg::Custom(_) => self.broadcast(msg),

            // Not forwarded — parent Model handles these
            _ => None,
        }
    }

    /// Compose all visible panes into a View with regions sorted by z-order.
    pub fn view(&self) -> View {
        let mut entries: Vec<&PaneEntry> = self.panes.iter().filter(|e| e.layout.visible).collect();
        entries.sort_by_key(|e| e.layout.z);

        let regions: Vec<(Rect, String)> = entries
            .iter()
            .map(|e| (e.layout.rect, e.pane.view()))
            .collect();

        View::default().with_regions(regions).with_alt_screen()
    }

    // --- internal helpers ---

    fn route_to_focused(&mut self, msg: &Msg) -> Cmd {
        if let Some(ref focus_id) = self.focus.clone() {
            self.route_to_pane(focus_id, msg)
        } else {
            None
        }
    }

    fn route_to_pane(&mut self, id: &str, msg: &Msg) -> Cmd {
        if let Some(entry) = self.panes.iter_mut().find(|e| e.id == id) {
            let pane_id = entry.id.clone();
            let cmd = entry.pane.update(msg);
            wrap_cmd(pane_id, cmd)
        } else {
            None
        }
    }

    fn broadcast(&mut self, msg: &Msg) -> Cmd {
        let results: Vec<(String, Cmd)> = self
            .panes
            .iter_mut()
            .filter(|e| e.layout.visible)
            .map(|e| (e.id.clone(), e.pane.update(msg)))
            .collect();
        let cmds = results
            .into_iter()
            .map(|(id, cmd)| wrap_cmd(id, cmd))
            .collect();
        batch(cmds)
    }

    fn hit_test(&self, x: u16, y: u16) -> Option<String> {
        // Highest z first
        let mut candidates: Vec<&PaneEntry> =
            self.panes.iter().filter(|e| e.layout.visible).collect();
        candidates.sort_by(|a, b| b.layout.z.cmp(&a.layout.z));
        candidates
            .iter()
            .find(|e| e.layout.rect.contains(x, y))
            .map(|e| e.id.clone())
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrap a pane's Cmd so the resulting Msg is tagged with the pane's id.
fn wrap_cmd(pane_id: String, cmd: Cmd) -> Cmd {
    let cmd_inner = cmd?;
    match cmd_inner {
        CmdInner::Sync(f) => Some(CmdInner::Sync(Box::new(move || {
            Msg::custom(PaneMsg {
                pane_id,
                inner: f(),
            })
        }))),
        CmdInner::Async(fut) => Some(CmdInner::Async(Box::pin(async move {
            Msg::custom(PaneMsg {
                pane_id,
                inner: fut.await,
            })
        }))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{KeyCode, KeyEvent, Modifiers};
    use crate::mouse::{MouseButton, MouseEvent};

    /// A test pane that records which messages it received.
    struct TestPane {
        name: String,
        messages: Vec<String>,
        is_focused: bool,
    }

    impl TestPane {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                messages: Vec::new(),
                is_focused: false,
            }
        }
    }

    impl Pane for TestPane {
        fn update(&mut self, msg: &Msg) -> Cmd {
            let label = match msg {
                Msg::KeyPress(k) => format!("key:{:?}", k.code),
                Msg::WindowSize { width, height } => format!("resize:{}x{}", width, height),
                Msg::MouseClick(m) => format!("click:{},{}", m.x, m.y),
                Msg::Custom(_) => "custom".to_string(),
                _ => format!("{:?}", msg),
            };
            self.messages.push(label);
            None
        }

        fn view(&self) -> String {
            format!("[{}]", self.name)
        }

        fn focus(&mut self) -> Cmd {
            self.is_focused = true;
            None
        }

        fn blur(&mut self) {
            self.is_focused = false;
        }

        fn focused(&self) -> bool {
            self.is_focused
        }
    }

    fn key_msg(code: KeyCode) -> Msg {
        Msg::KeyPress(KeyEvent {
            code,
            modifiers: Modifiers::empty(),
            is_repeat: false,
        })
    }

    fn click_msg(x: u16, y: u16) -> Msg {
        Msg::MouseClick(MouseEvent {
            x,
            y,
            button: MouseButton::Left,
            modifiers: Modifiers::empty(),
        })
    }

    #[test]
    fn test_add_remove() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "b",
            TestPane::new("B"),
            PaneLayout::new(Rect::new(40, 0, 40, 24), 0),
        );
        assert!(scene.contains("a"));
        assert!(scene.contains("b"));
        assert!(!scene.contains("c"));

        scene.remove("a");
        assert!(!scene.contains("a"));
        assert!(scene.contains("b"));
    }

    #[test]
    fn test_focus_lifecycle() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "b",
            TestPane::new("B"),
            PaneLayout::new(Rect::new(40, 0, 40, 24), 0),
        );

        scene.set_focus("a");
        assert_eq!(scene.focused(), Some("a"));
        assert!(scene.pane_as::<TestPane>("a").unwrap().is_focused);
        assert!(!scene.pane_as::<TestPane>("b").unwrap().is_focused);

        scene.set_focus("b");
        assert_eq!(scene.focused(), Some("b"));
        assert!(!scene.pane_as::<TestPane>("a").unwrap().is_focused);
        assert!(scene.pane_as::<TestPane>("b").unwrap().is_focused);

        scene.clear_focus();
        assert_eq!(scene.focused(), None);
        assert!(!scene.pane_as::<TestPane>("b").unwrap().is_focused);
    }

    #[test]
    fn test_input_routes_to_focused_only() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "b",
            TestPane::new("B"),
            PaneLayout::new(Rect::new(40, 0, 40, 24), 0),
        );
        scene.set_focus("a");

        let msg = key_msg(KeyCode::Char('x'));
        scene.update(&msg);

        assert_eq!(scene.pane_as::<TestPane>("a").unwrap().messages.len(), 1);
        assert_eq!(scene.pane_as::<TestPane>("b").unwrap().messages.len(), 0);
    }

    #[test]
    fn test_broadcast_reaches_all() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "b",
            TestPane::new("B"),
            PaneLayout::new(Rect::new(40, 0, 40, 24), 0),
        );
        scene.set_focus("a");

        let msg = Msg::WindowSize {
            width: 80,
            height: 24,
        };
        scene.update(&msg);

        assert_eq!(scene.pane_as::<TestPane>("a").unwrap().messages.len(), 1);
        assert_eq!(scene.pane_as::<TestPane>("b").unwrap().messages.len(), 1);
    }

    #[test]
    fn test_custom_broadcasts() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "b",
            TestPane::new("B"),
            PaneLayout::new(Rect::new(40, 0, 40, 24), 0),
        );

        let msg = Msg::custom(42u32);
        scene.update(&msg);

        assert_eq!(scene.pane_as::<TestPane>("a").unwrap().messages.len(), 1);
        assert_eq!(scene.pane_as::<TestPane>("b").unwrap().messages.len(), 1);
    }

    #[test]
    fn test_mouse_hit_test() {
        let mut scene = Scene::new().with_click_focus(false);
        scene.add(
            "left",
            TestPane::new("L"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "right",
            TestPane::new("R"),
            PaneLayout::new(Rect::new(40, 0, 40, 24), 0),
        );

        // Click in left pane
        scene.update(&click_msg(10, 5));
        assert_eq!(scene.pane_as::<TestPane>("left").unwrap().messages.len(), 1);
        assert_eq!(
            scene.pane_as::<TestPane>("right").unwrap().messages.len(),
            0
        );

        // Click in right pane
        scene.update(&click_msg(50, 5));
        assert_eq!(scene.pane_as::<TestPane>("left").unwrap().messages.len(), 1);
        assert_eq!(
            scene.pane_as::<TestPane>("right").unwrap().messages.len(),
            1
        );
    }

    #[test]
    fn test_mouse_z_order() {
        let mut scene = Scene::new().with_click_focus(false);
        // Background covers full area
        scene.add(
            "bg",
            TestPane::new("BG"),
            PaneLayout::new(Rect::new(0, 0, 80, 24), 0),
        );
        // Modal overlaps in the center
        scene.add(
            "modal",
            TestPane::new("Modal"),
            PaneLayout::new(Rect::new(20, 5, 40, 14), 1),
        );

        // Click inside the modal area → modal gets it (higher z)
        scene.update(&click_msg(30, 10));
        assert_eq!(scene.pane_as::<TestPane>("bg").unwrap().messages.len(), 0);
        assert_eq!(
            scene.pane_as::<TestPane>("modal").unwrap().messages.len(),
            1
        );

        // Click outside the modal → bg gets it
        scene.update(&click_msg(5, 5));
        assert_eq!(scene.pane_as::<TestPane>("bg").unwrap().messages.len(), 1);
    }

    #[test]
    fn test_view_regions_sorted_by_z() {
        let mut scene = Scene::new();
        scene.add(
            "top",
            TestPane::new("T"),
            PaneLayout::new(Rect::new(0, 0, 80, 24), 5),
        );
        scene.add(
            "bottom",
            TestPane::new("B"),
            PaneLayout::new(Rect::new(0, 0, 80, 24), 0),
        );

        let view = scene.view();
        assert_eq!(view.regions.len(), 2);
        // Lower z first
        assert_eq!(view.regions[0].1, "[B]");
        assert_eq!(view.regions[1].1, "[T]");
    }

    #[test]
    fn test_invisible_pane_excluded() {
        let mut scene = Scene::new();
        scene.add(
            "vis",
            TestPane::new("V"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "hidden",
            TestPane::new("H"),
            PaneLayout {
                rect: Rect::new(40, 0, 40, 24),
                z: 0,
                visible: false,
            },
        );

        let view = scene.view();
        assert_eq!(view.regions.len(), 1);

        // Broadcast skips hidden panes
        let msg = Msg::WindowSize {
            width: 80,
            height: 24,
        };
        scene.update(&msg);
        assert_eq!(scene.pane_as::<TestPane>("vis").unwrap().messages.len(), 1);
        assert_eq!(
            scene.pane_as::<TestPane>("hidden").unwrap().messages.len(),
            0
        );
    }

    #[test]
    fn test_pane_as_downcast() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );

        let pane = scene.pane_as::<TestPane>("a").unwrap();
        assert_eq!(pane.name, "A");

        let pane_mut = scene.pane_as_mut::<TestPane>("a").unwrap();
        pane_mut.name = "Modified".to_string();
        assert_eq!(scene.pane_as::<TestPane>("a").unwrap().name, "Modified");
    }

    #[test]
    fn test_click_focuses() {
        let mut scene = Scene::new().with_click_focus(true);
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.add(
            "b",
            TestPane::new("B"),
            PaneLayout::new(Rect::new(40, 0, 40, 24), 0),
        );
        scene.set_focus("a");

        // Click in pane b should shift focus
        scene.update(&click_msg(50, 5));
        assert_eq!(scene.focused(), Some("b"));
        assert!(!scene.pane_as::<TestPane>("a").unwrap().is_focused);
        assert!(scene.pane_as::<TestPane>("b").unwrap().is_focused);
    }

    #[test]
    fn test_remove_focused_clears_focus() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );
        scene.set_focus("a");
        assert_eq!(scene.focused(), Some("a"));

        scene.remove("a");
        assert_eq!(scene.focused(), None);
    }

    #[test]
    fn test_set_layout() {
        let mut scene = Scene::new();
        scene.add(
            "a",
            TestPane::new("A"),
            PaneLayout::new(Rect::new(0, 0, 40, 24), 0),
        );

        scene.set_layout("a", PaneLayout::new(Rect::new(10, 10, 20, 10), 5));
        let view = scene.view();
        assert_eq!(view.regions[0].0, Rect::new(10, 10, 20, 10));
    }
}
