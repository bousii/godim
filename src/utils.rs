use godot::classes::Node;
use godot::prelude::*;

/* DFS to find first descendant matching Node Type T within max_depth max_depth */
pub fn find_descendant<T>(node: &Gd<Node>, max_depth: u32) -> Option<Gd<T>>
where
    T: GodotClass + Inherits<Node>,
{
    if let Ok(found) = node.clone().try_cast::<T>() {
        return Some(found);
    }
    if max_depth == 0 {
        return None;
    }
    for child in node.get_children().iter_shared() {
        if let Some(found) = find_descendant(&child, max_depth - 1) {
            return Some(found);
        }
    }
    None
}
