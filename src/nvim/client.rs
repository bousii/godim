use async_trait::async_trait;
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat, create::tokio as create};
use tokio::process::ChildStdin;

pub struct EditorState {
    pub lines: Vec<String>,
    pub cursor: (i64, i64),
    pub mode: String,
}

pub struct NvimSession {
    nvim: Neovim<Compat<ChildStdin>>,
    _io_handle: tokio::task::JoinHandle<Result<(), Box<nvim_rs::error::LoopError>>>,
    _child: tokio::process::Child,
    input_rx: tokio::sync::mpsc::Receiver<String>,
}

impl NvimSession {
    pub async fn start(
        input_rx: tokio::sync::mpsc::Receiver<String>,
        state_tx: std::sync::mpsc::Sender<EditorState>,
    ) -> Self {
        let handler = NvimHandler { state_tx };
        let (nvim, _io_handle, _child) = create::new_child_cmd(
            /* NOTE: Can maybe add basic config profiles down the line? */
            tokio::process::Command::new("nvim")
                .arg("--embed")
                .arg("--headless")
                .arg("-n")
                .arg("-u")
                .arg("NONE"),
            handler,
        )
        .await
        .expect("failed to start nvim");

        let options = nvim_rs::UiAttachOptions::new();
        /* NOTE: Placeholder width and height values, grab this from TextEdit later */
        nvim.ui_attach(80, 24, &options)
            .await
            .expect("ui_attach failed");
        Self {
            nvim,
            _io_handle,
            _child,
            input_rx,
        }
    }

    pub async fn listen_for_keys(&mut self) {
        println!("task started");
        loop {
            println!("waiting for key");
            let key = self.input_rx.recv().await.unwrap();
            println!("got key {}", key);
            self.input(&key).await;
        }
    }

    pub async fn input(&self, keys: &str) {
        self.nvim.input(keys).await.expect("input failed");
    }
}

#[derive(Clone)]
struct NvimHandler {
    state_tx: std::sync::mpsc::Sender<EditorState>,
}

#[async_trait]
impl Handler for NvimHandler {
    type Writer = Compat<ChildStdin>;
    async fn handle_notify(
        &self,
        _name: String,
        _args: Vec<Value>,
        _neovim: Neovim<<Self as Handler>::Writer>,
    ) {
        println!("THERE HAS BEEN A NOTIFICATION {}", _name);
        if _name == "redraw" && check_for_flush(&_args) {
            let state = get_state(_neovim).await;
            println!("lines: {:?}", state.lines);
            println!("cursor: ({}, {})", state.cursor.0, state.cursor.1);
            println!("mode: {}", state.mode);
            self.state_tx.send(state).unwrap();
        }
    }
}

fn check_for_flush(args: &Vec<Value>) -> bool {
    for arg in args {
        if let Some(array) = arg.as_array() {
            if let Some(Value::String(event_name)) = array.first() {
                if event_name.as_str() == Some("flush") {
                    return true;
                }
            }
        }
    }
    false
}
pub async fn get_state(nvim: Neovim<Compat<ChildStdin>>) -> EditorState {
    let buf = nvim
        .get_current_buf()
        .await
        .expect("get_current_buf failed");
    let lines = buf.get_lines(0, -1, false).await.expect("get_lines failed");

    let cursor = nvim
        .get_current_win()
        .await
        .expect("get_current_win failed")
        .get_cursor()
        .await
        .expect("get_cursor failed");

    let mode = nvim.get_mode().await.expect("get_mode failed");
    let mode_str = mode
        .iter()
        .find(|(k, _)| k.as_str() == Some("mode"))
        .and_then(|(_, v)| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    EditorState {
        lines,
        cursor,
        mode: mode_str,
    }
}
