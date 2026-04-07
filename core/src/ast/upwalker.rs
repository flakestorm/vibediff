use tree_sitter::{Node, Tree};

pub fn find_enclosing_anchor<'a>(
    tree: &'a Tree,
    line_zero_based: usize,
    anchor_kinds: &[&str],
) -> Option<Node<'a>> {
    let root = tree.root_node();
    let point = tree_sitter::Point {
        row: line_zero_based,
        column: 0,
    };
    let mut node = root.descendant_for_point_range(point, point)?;
    loop {
        if anchor_kinds.iter().any(|k| *k == node.kind()) {
            return Some(node);
        }
        match node.parent() {
            Some(parent) => node = parent,
            None => return None,
        }
    }
}
