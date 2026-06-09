use crate::discovery::{find_code_edit, find_script_editor};
use crate::input::keycode_to_nvim;
use crate::nvim::client::{EditorState, NvimCommand, NvimSession};
use godot::classes::{
    CodeEdit, Control, EditorPlugin, IEditorPlugin, InputEvent, InputEventKey, ProjectSettings,
    Script, TextEdit, text_edit::CaretType,
};
use godot::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::UnboundedSender;

#[derive(GodotClass)]
#[class(tool, base=EditorPlugin)]
struct GodimPlugin {
    base: Base<EditorPlugin>,
    _runtime: Runtime,
    input_tx: UnboundedSender<NvimCommand>,
    state: Arc<Mutex<EditorState>>,
    attached_editor: Option<Gd<CodeEdit>>,
    render: bool,
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

        self.base_mut().set_process(true);
    }

    fn init(base: Base<EditorPlugin>) -> Self {
        godot_print!("Initializing godim plugin");
        let runtime = Runtime::new().expect("failed to create tokio runtime");
        let (mut session, state, input_tx) = runtime.block_on(NvimSession::start());

        runtime.spawn(async move { session.recv().await });

        Self {
            base,
            _runtime: runtime,
            input_tx,
            state,
            attached_editor: None,
            render: true,
        }
    }

    fn exit_tree(&mut self) {
        // gracefully close neovim and runtime
    }

    fn process(&mut self, _delta: f64) {
        self.render_current_state();
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

        if let Some(ref editor) = self.attached_editor
            && editor.is_instance_valid()
            && editor.instance_id() == current_code_edit.instance_id()
        {
            return;
        }

        let gui_input: Callable = self.base().callable("on_gui_input");
        current_code_edit
            .upcast_mut::<Control>()
            .connect("gui_input", &gui_input);

        current_code_edit
            .upcast_mut::<TextEdit>()
            .set_caret_blink_enabled(false);

        godot_print!("Found the CodeEdit!");
        self.attached_editor = Some(current_code_edit);

        self.sync_buffer_to_nvim();
        self.set_nvim_path(
            ProjectSettings::singleton()
                .globalize_path(&_script.get_path())
                .to_string(),
        );
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
                key_event.get_unicode(),
                key_event.is_shift_pressed(),
                key_event.is_ctrl_pressed(),
                key_event.is_alt_pressed(),
            ) else {
                godot_print!("Unrecognized key code {}", key_str);
                return;
            };

            let mode = {
                let Ok(state) = self.state.try_lock() else {
                    return;
                };
                state.mode.clone()
            };
            let is_insert = mode == "insert";
            let is_escaping = nvim_str == "<Esc>" || nvim_str == "<C-c>";
            println!("mode {} str {}", mode, nvim_str);
            if is_insert && !is_escaping {
                self.render = false;
                return;
            }

            /*
             * NOTE: This lets us "consume" the input instead of us just
             * reading the presses and passing it to Godot.
             */
            self.base().get_viewport().unwrap().set_input_as_handled();

            let nvim_cmd = NvimCommand::Input(nvim_str);
            let res = self.input_tx.send(nvim_cmd);
            if res.is_err() {
                println!("Error in sending on_gui_input");
            }
            if is_insert && is_escaping {
                self.sync_buffer_to_nvim();
                // if let Some(ref mut editor) = self.attached_editor {
                //     editor.upcast_mut::<Control>().release_focus();
                // }
                self.render = true;
            }
        }
    }
}

impl GodimPlugin {
    fn render_current_state(&mut self) {
        if !self.render {
            return;
        }
        let Ok(state) = self.state.try_lock() else {
            return;
        };
        let Some(ref mut editor) = self.attached_editor else {
            return;
        };
        let text = state.lines.join("\n");
        if editor.upcast_mut::<TextEdit>().get_text().to_string() != text {
            editor.upcast_mut::<TextEdit>().set_text(&text);
        }

        let caret_type = match state.mode.as_str() {
            "i" | "ci" | "insert" => CaretType::LINE,

            _ => CaretType::BLOCK,
        };

        let (row, col) = state.cursor;
        // println!("setting caret to row={} col={}", row, col);
        editor.set_caret_column(col as i32);
        editor.set_caret_line((row - 1) as i32);
        editor.set_caret_type(caret_type);
    }

    fn sync_buffer_to_nvim(&mut self) {
        let Some(ref mut editor) = self.attached_editor else {
            return;
        };
        let lines: Vec<String> = editor
            .upcast_mut::<TextEdit>()
            .get_text()
            .to_string()
            .split("\n")
            .map(String::from)
            .collect();
        let nvim_cmd: NvimCommand = NvimCommand::SetBuffer(lines);
        let res = self.input_tx.send(nvim_cmd);
        if res.is_err() {
            println!("Error in sending sync_buffer_to_nvim");
        }
    }
    fn set_nvim_path(&mut self, path: String) {
        let nvim_cmd: NvimCommand = NvimCommand::SetPath(path);
        let res = self.input_tx.send(nvim_cmd);
        if res.is_err() {
            println!("Error in sending set_nvim_path");
        }
    }
}
