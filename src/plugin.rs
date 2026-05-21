use crate::discovery::{find_code_edit, find_script_editor};
use crate::input::keycode_to_nvim;
use crate::nvim::client::NvimSession;
use godot::classes::{
    CodeEdit, Control, EditorPlugin, IEditorPlugin, InputEvent, InputEventKey, Script,
};
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
        let Some(mut current_code_edit) = find_code_edit() else {
            godot_print!("Unable to find current CodeEdit, might not be open");
            return;
        };

        if let Some(ref editor) = self.attached_editor {
            if editor.is_instance_valid() && editor.instance_id() == current_code_edit.instance_id()
            {
                return;
            }
        }

        let gui_input: Callable = self.base().callable("on_gui_input");
        current_code_edit
            .upcast_mut::<Control>()
            .connect("gui_input", &gui_input);

        godot_print!("Found the CodeEdit!");
        self.attached_editor = Some(current_code_edit);
    }

    #[func]
    fn on_gui_input(&mut self, input_event: Gd<InputEvent>) {
        if let Ok(key_event) = input_event.try_cast::<InputEventKey>()
            && key_event.is_pressed()
        {
            let keycode = key_event.get_keycode();
            let key_str = keycode.as_str();
            godot_print!("{}", key_str);
            let Some(nvim_str) = keycode_to_nvim(
                keycode,
                key_event.is_shift_pressed(),
                key_event.is_ctrl_pressed(),
                key_event.is_alt_pressed(),
            ) else {
                godot_print!("Unrecognized key code {}", key_str);
                return;
            };

            self.runtime
                .block_on(self.session.as_ref().unwrap().input(&nvim_str));

            /*
             * NOTE: This lets us "consume" the input instead of us just
             * reading the presses and passing it to Godot.
             */
            self.base().get_viewport().unwrap().set_input_as_handled();

            let state = self
                .runtime
                .block_on(self.session.as_ref().unwrap().get_state());
            godot_print!("lines: {:?}", state.lines);
            godot_print!("cursor: ({}, {})", state.cursor.0, state.cursor.1);
            godot_print!("mode: {}", state.mode);
        }
    }
}
