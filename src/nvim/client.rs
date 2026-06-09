use async_trait::async_trait;
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat, create::tokio as create};
use std::sync::{Arc, Mutex};
use tokio::process::ChildStdin;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub struct EditorState {
    pub lines: Vec<String>,
    pub cursor: (i64, i64),
    pub mode: String,
}

pub struct NvimSession {
    nvim: Neovim<Compat<ChildStdin>>,
    _io_handle: tokio::task::JoinHandle<Result<(), Box<nvim_rs::error::LoopError>>>,
    _child: tokio::process::Child,
    input_rx: UnboundedReceiver<NvimCommand>,
}

pub enum NvimCommand {
    Input(String),
    SetBuffer(Vec<String>),
    SetPath(String),
}

impl NvimSession {
    pub async fn start() -> (Self, Arc<Mutex<EditorState>>, UnboundedSender<NvimCommand>) {
        let (input_tx, input_rx) = unbounded_channel::<NvimCommand>();
        let state = Arc::new(Mutex::new(EditorState {
            lines: vec![],
            cursor: (0, 0),
            mode: String::from("n"),
        }));
        let handler = NvimHandler {
            state: state.clone(),
            // input_tx: input_tx.clone(),
        };
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
        nvim.ui_attach(1000, 50, &options)
            .await
            .expect("ui_attach failed");
        nvim.get_current_buf()
            .await
            .unwrap()
            .attach(true, vec![])
            .await
            .expect("buf attach failed");
        (
            Self {
                nvim,
                _io_handle,
                _child,
                input_rx,
            },
            state,
            input_tx,
        )
    }

    pub async fn recv(&mut self) {
        println!("task started");
        loop {
            let cmd = self.input_rx.recv().await.unwrap();
            self.handle_nvim_cmd(cmd).await;
        }
    }

    pub async fn input(&self, keys: &str) {
        self.nvim.input(keys).await.expect("input failed");
    }

    pub async fn handle_nvim_cmd(&self, cmd: NvimCommand) {
        let nvim = &self.nvim;
        match cmd {
            NvimCommand::SetBuffer(lines) => {
                let buf = nvim.get_current_buf().await.unwrap();
                buf.set_lines(0, -1, false, lines).await.unwrap();
            }
            NvimCommand::SetPath(path) => {
                let buf = nvim.get_current_buf().await.unwrap();
                buf.set_name(&path).await.unwrap();
            }
            NvimCommand::Input(input) => {
                println!("got key {}", input);
                self.input(&input).await;
            }
        }
    }
}

#[derive(Clone)]
struct NvimHandler {
    // input_tx: tokio::sync::mpsc::UnboundedSender<NvimCommand>,
    state: Arc<Mutex<EditorState>>,
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
        let mut state = self.state.lock().unwrap();
        if _name == "redraw" {
            for event in &_args {
                if let Some(array) = event.as_array() {
                    if let Some(Value::String(event_name)) = array.first() {
                        match event_name.as_str() {
                            Some("mode_change") => {
                                if let Some(inner) = array.get(1).and_then(|v| v.as_array()) {
                                    let mode = inner.first().and_then(|v| v.as_str()).unwrap();
                                    state.mode = String::from(mode);
                                }
                            }
                            Some("cursor_goto") => {
                                if let Some(inner) = array.get(1).and_then(|v| v.as_array()) {
                                    let row = inner.first().and_then(|v| v.as_i64()).unwrap() + 1;
                                    let col = inner.get(1).and_then(|v| v.as_i64()).unwrap();
                                    state.cursor = (row, col);
                                }
                            }
                            Some("flush") => {}
                            _ => {}
                        }
                    }
                }
            }
        }
        if _name == "nvim_buf_lines_event" {
            let first_line = _args[2].as_i64().unwrap_or(0) as usize;
            let last_line = _args[3].as_i64().unwrap_or(-1);
            let new_lines: Vec<String> = _args[4]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            if last_line == -1 {
                // full buffer replace
                state.lines = new_lines;
            } else {
                // replace lines from first_line to last_line with new_lines
                state
                    .lines
                    .splice(first_line..last_line as usize, new_lines);
            }
        }
    }
}
