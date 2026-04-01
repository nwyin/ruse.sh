/// Layer/Compositor/Canvas system for compositing overlapping text regions.
use std::collections::HashMap;

/// A hierarchical layer: content positioned at `(x, y)` with a z-order.
///
/// Children are positioned relative to their parent.
pub struct Layer {
    pub id: String,
    pub content: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub children: Vec<Layer>,
}

/// A flattened, absolutely-positioned layer ready for compositing.
struct FlatLayer {
    id: String,
    content: String,
    abs_x: i32,
    abs_y: i32,
    z: i32,
    width: u32,
    height: u32,
}

/// Composites a tree of [`Layer`]s into a single string output.
pub struct Compositor {
    layers: Vec<FlatLayer>,
    index: HashMap<String, usize>,
}

impl Layer {
    /// Create a new layer with the given id and content at position `(x, y, z)`.
    pub fn new(id: impl Into<String>, content: impl Into<String>, x: i32, y: i32, z: i32) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            x,
            y,
            z,
            children: Vec::new(),
        }
    }

    /// Attach children and return self.
    pub fn with_children(mut self, children: Vec<Layer>) -> Self {
        self.children = children;
        self
    }
}

impl Compositor {
    /// Flatten a layer tree, computing absolute positions, then sort by z-order.
    pub fn new(root: &Layer) -> Self {
        let mut layers = Vec::new();
        flatten(root, 0, 0, &mut layers);
        layers.sort_by_key(|l| l.z);

        let index: HashMap<String, usize> = layers
            .iter()
            .enumerate()
            .map(|(i, l)| (l.id.clone(), i))
            .collect();

        Self { layers, index }
    }

    /// Render all layers to a single string canvas, painting low-z first.
    pub fn draw(&self) -> String {
        if self.layers.is_empty() {
            return String::new();
        }

        // Determine bounding box.
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for fl in &self.layers {
            min_x = min_x.min(fl.abs_x);
            min_y = min_y.min(fl.abs_y);
            max_x = max_x.max(fl.abs_x + fl.width as i32);
            max_y = max_y.max(fl.abs_y + fl.height as i32);
        }

        let canvas_w = (max_x - min_x) as usize;
        let canvas_h = (max_y - min_y) as usize;

        if canvas_w == 0 || canvas_h == 0 {
            return String::new();
        }

        // Build a 2-D grid of characters, initialized to spaces.
        let mut grid: Vec<Vec<char>> = vec![vec![' '; canvas_w]; canvas_h];

        // Paint layers in z-order (already sorted).
        for fl in &self.layers {
            let lines: Vec<&str> = fl.content.split('\n').collect();
            for (row_off, line) in lines.iter().enumerate() {
                for (col_off, ch) in line.chars().enumerate() {
                    let gy = (fl.abs_y - min_y) as usize + row_off;
                    let gx = (fl.abs_x - min_x) as usize + col_off;
                    if gy < canvas_h && gx < canvas_w {
                        grid[gy][gx] = ch;
                    }
                }
            }
        }

        // Convert grid to string, trimming trailing spaces per line.
        let mut out = String::new();
        for (i, row) in grid.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let line: String = row.iter().collect();
            out.push_str(line.trim_end());
        }

        // Remove trailing empty lines.
        while out.ends_with('\n') {
            out.pop();
        }

        out
    }

    /// Return the id of the highest-z layer whose bounding box contains `(x, y)`.
    ///
    /// Coordinates are in the same absolute system used after flattening (relative
    /// to the root layer's origin).
    pub fn hit_test(&self, x: i32, y: i32) -> Option<&str> {
        // Iterate in *reverse* z-order (highest z first).
        for fl in self.layers.iter().rev() {
            if x >= fl.abs_x
                && x < fl.abs_x + fl.width as i32
                && y >= fl.abs_y
                && y < fl.abs_y + fl.height as i32
            {
                return Some(&fl.id);
            }
        }
        None
    }

    /// Look up a layer by id. Returns `true` if the id exists.
    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(id)
    }
}

/// Recursively flatten a [`Layer`] tree into a vec of [`FlatLayer`]s.
fn flatten(layer: &Layer, parent_x: i32, parent_y: i32, out: &mut Vec<FlatLayer>) {
    let abs_x = parent_x + layer.x;
    let abs_y = parent_y + layer.y;

    let lines: Vec<&str> = layer.content.split('\n').collect();
    let height = lines.len() as u32;
    let width = lines
        .iter()
        .map(|l| l.chars().count() as u32)
        .max()
        .unwrap_or(0);

    out.push(FlatLayer {
        id: layer.id.clone(),
        content: layer.content.clone(),
        abs_x,
        abs_y,
        z: layer.z,
        width,
        height,
    });

    for child in &layer.children {
        flatten(child, abs_x, abs_y, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_single_layer() {
        let root = Layer::new("root", "hello", 0, 0, 0);
        let comp = Compositor::new(&root);
        assert_eq!(comp.layers.len(), 1);
        assert!(comp.contains("root"));
    }

    #[test]
    fn test_flatten_nested() {
        let root = Layer::new("root", "bg", 0, 0, 0).with_children(vec![
            Layer::new("child1", "fg1", 2, 1, 1),
            Layer::new("child2", "fg2", 4, 2, 2),
        ]);
        let comp = Compositor::new(&root);
        assert_eq!(comp.layers.len(), 3);
        assert!(comp.contains("root"));
        assert!(comp.contains("child1"));
        assert!(comp.contains("child2"));

        // Verify absolute positions.
        let c1 = &comp.layers[comp.index["child1"]];
        assert_eq!((c1.abs_x, c1.abs_y), (2, 1));

        let c2 = &comp.layers[comp.index["child2"]];
        assert_eq!((c2.abs_x, c2.abs_y), (4, 2));
    }

    #[test]
    fn test_draw_order() {
        // Two overlapping 1-char layers at the same position.
        // Higher z should win.
        let root =
            Layer::new("bg", "X", 0, 0, 0).with_children(vec![Layer::new("fg", "O", 0, 0, 1)]);
        let comp = Compositor::new(&root);
        let result = comp.draw();
        assert_eq!(result, "O");
    }

    #[test]
    fn test_draw_non_overlapping() {
        let root = Layer::new("root", "", 0, 0, 0).with_children(vec![
            Layer::new("a", "A", 0, 0, 1),
            Layer::new("b", "B", 2, 0, 1),
        ]);
        let comp = Compositor::new(&root);
        let result = comp.draw();
        assert_eq!(result, "A B");
    }

    #[test]
    fn test_draw_multiline() {
        let root = Layer::new("root", "ab\ncd", 0, 0, 0);
        let comp = Compositor::new(&root);
        let result = comp.draw();
        assert_eq!(result, "ab\ncd");
    }

    #[test]
    fn test_hit_test_highest_z() {
        let root =
            Layer::new("bg", "XXX", 0, 0, 0).with_children(vec![Layer::new("fg", "O", 0, 0, 1)]);
        let comp = Compositor::new(&root);
        // (0,0) is covered by both bg (z=0, width=3) and fg (z=1, width=1)
        assert_eq!(comp.hit_test(0, 0), Some("fg"));
        // (2,0) is only covered by bg
        assert_eq!(comp.hit_test(2, 0), Some("bg"));
        // off-canvas
        assert_eq!(comp.hit_test(10, 10), None);
    }

    #[test]
    fn test_hit_test_none() {
        let root = Layer::new("root", "X", 5, 5, 0);
        let comp = Compositor::new(&root);
        // Miss: before the layer
        assert_eq!(comp.hit_test(0, 0), None);
        // Hit
        assert_eq!(comp.hit_test(5, 5), Some("root"));
    }

    #[test]
    fn test_draw_offset_layers() {
        // Child at offset, verifying canvas expands correctly.
        let root =
            Layer::new("root", "R", 0, 0, 0).with_children(vec![Layer::new("child", "C", 3, 1, 1)]);
        let comp = Compositor::new(&root);
        let result = comp.draw();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "R");
        assert_eq!(lines[1], "   C");
    }

    #[test]
    fn test_empty_content() {
        let root = Layer::new("root", "", 0, 0, 0);
        let comp = Compositor::new(&root);
        let result = comp.draw();
        assert_eq!(result, "");
    }
}
