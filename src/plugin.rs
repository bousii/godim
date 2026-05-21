use crate::discovery::{find_code_edit, find_script_editor};
use crate::nvim::client::NvimSession;
use godot::classes::{CodeEdit, EditorPlugin, IEditorPlugin, Script};
use godot::prelude::*;
use tokio::runtime::Runtime;

#[derive(GodotClass)]
#[class(tool, base=EditorPlugin)]
struct GodimPlugin {
    base: Base<EditorPlugin>,
    runtime: tokio::runtime::Runtime,
    session: Option<NvimSession>,
    attached_editor: Option<Gd<CodeEdit>>,
}

#[godot_api]
impl IEditorPlugin for GodimPlugin {
    fn enter_tree(&mut self) {
        let Some(mut script_editor) = find_script_editor() else {
            godot_print!("Unable to find ScriptEditor");
            return;
        };
        godot_print!("entering tree");
        let editor_script_changed: Callable = self.base().callable("on_script_changed");
        script_editor.connect("editor_script_changed", &editor_script_changed);

        // Perform typical plugin operations here.
    }

    fn init(base: Base<EditorPlugin>) -> Self {
        godot_print!("Initializing godim plugin");
        let runtime = Runtime::new().expect("failed to create tokio runtime");
        let session = runtime.block_on(NvimSession::start());
        Self {
            base,
            runtime,
            session: Some(session),
            attached_editor: None,
        }
    }

    fn exit_tree(&mut self) {
        // Perform typical plugin operations here.
    }
}

#[godot_api]
impl GodimPlugin {
    #[func]
    fn on_script_changed(&mut self, _script: Gd<Script>) {
        let Some(current_code_edit) = find_code_edit() else {
            godot_print!("Unable to find current CodeEdit, might not be open");
            return;
        };

        if let Some(ref editor) = self.attached_editor {
            if editor.is_instance_valid() && editor.instance_id() == current_code_edit.instance_id()
            {
                return;
            }
        }

        godot_print!("Found the CodeEdit!");
        self.attached_editor = Some(current_code_edit);
    }
}
