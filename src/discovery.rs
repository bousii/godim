use godot::classes::{CodeEdit, EditorInterface, Node, ScriptEditor};
use godot::prelude::*;
/*
 * This value is stolen directly from godotvim, apparently
 * they measured this to be the maximum.
 * I believe them.
 */
pub const MAX_DISCOVERY_DEPTH: u32 = 20;

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

pub fn find_code_edit() -> Option<Gd<CodeEdit>> {
    let interface = EditorInterface::singleton();
    let script_editor = interface.get_script_editor()?;
    let current_editor = script_editor.get_current_editor()?;
    find_descendant::<CodeEdit>(&current_editor.upcast(), MAX_DISCOVERY_DEPTH)
}

pub fn find_script_editor() -> Option<Gd<ScriptEditor>> {
    let interface = EditorInterface::singleton();
    interface.get_script_editor()
}
