use crate::nvim::client::NvimSession;
use godot::classes::{EditorPlugin, IEditorPlugin};
use godot::prelude::*;
use tokio::runtime::Runtime;

#[derive(GodotClass)]
#[class(tool, base=EditorPlugin)]
struct GodimPlugin {
    base: Base<EditorPlugin>,
    runtime: tokio::runtime::Runtime,
    session: Option<NvimSession>,
}

#[godot_api]
impl IEditorPlugin for GodimPlugin {
    fn enter_tree(&mut self) {
        // self.session = NvimSession::start().await;
        //  session.input(&args.keys).await;
        // let get_state = session.get_state().await;
        // let state = get_state;
        // for line in &state.lines {
        //     println!("{}", line);
        // }
        // println!("cursor ({}, {})", state.cursor.0, state.cursor.1);
        // println!("mode {}", state.mode);
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
        }
    }

    fn exit_tree(&mut self) {
        // Perform typical plugin operations here.
    }
}
