use crate::utils::find_descendant;
use godot::classes::{CodeEdit, EditorInterface, ScriptEditor};
use godot::prelude::*;
/*
 * This value is stolen directly from godotvim, apparently
 * they measured this to be the maximum.
 * I believe them.
 */
pub const MAX_DISCOVERY_DEPTH: u32 = 20;

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
