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

#[derive(GodotClass)]
#[class(tool, base=EditorPlugin)]
struct GodimPlugin {
    base: Base<EditorPlugin>,
    _runtime: tokio::runtime::Runtime,
    input_tx: tokio::sync::mpsc::UnboundedSender<NvimCommand>,
    state: Arc<Mutex<EditorState>>,
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

        self.base_mut().set_process(true);
    }

    fn init(base: Base<EditorPlugin>) -> Self {
        godot_print!("Initializing godim plugin");
        let runtime = Runtime::new().expect("failed to create tokio runtime");
        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel::<NvimCommand>();
        let (mut session, state) = runtime.block_on(NvimSession::start(input_rx));

        runtime.spawn(async move { session.recv().await });

        Self {
            base,
            _runtime: runtime,
            input_tx,
            state,
            attached_editor: None,
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

            let nvim_cmd = NvimCommand::Input(nvim_str);
            self.input_tx.send(nvim_cmd).unwrap();

            /*
             * NOTE: This lets us "consume" the input instead of us just
             * reading the presses and passing it to Godot.
             */
            self.base().get_viewport().unwrap().set_input_as_handled();
        }
    }
}

impl GodimPlugin {
    fn render_current_state(&mut self) {
        let Ok(mut state) = self.state.try_lock() else {
            return;
        };

        if !state.render_me {
            return;
        }

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
        println!("setting caret to row={} col={}", row, col);
        editor.set_caret_column(col as i32);
        editor.set_caret_line((row - 1) as i32);
        editor.set_caret_type(caret_type);
        state.render_me = false;
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
        let _ = self.input_tx.send(nvim_cmd);
    }
    fn set_nvim_path(&mut self, path: String) {
        let nvim_cmd: NvimCommand = NvimCommand::SetPath(path);
        let _ = self.input_tx.send(nvim_cmd);
    }
}
