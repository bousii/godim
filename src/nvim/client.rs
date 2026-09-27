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

pub enum NvimCommand {
    Input(String),
    SetBuffer(Vec<String>),
    SetCursor(i32, i32),
    SetPath(String),
}

pub struct NvimSession {
    nvim: Neovim<Compat<ChildStdin>>,
    _io_handle: tokio::task::JoinHandle<Result<(), Box<nvim_rs::error::LoopError>>>,
    _child: tokio::process::Child,
    input_rx: UnboundedReceiver<NvimCommand>,
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
        };
        let (nvim, _io_handle, _child) = create::new_child_cmd(
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
        nvim.ui_attach(1000, 50, &options)
            .await
            .expect("ui_attach failed");

        nvim.get_current_buf()
            .await
            .unwrap()
            .attach(true, vec![])
            .await
            .expect("buf attach failed");

        /* NOTE: Godot treats tabs as 1 character internally for cursor positioning,
         * so we match that in nvim with tabstop=1. shiftwidth=1 ensures >> inserts
         * one real tab character. noexpandtab keeps real tabs instead of spaces... */
        if nvim
            .command("set tabstop=1 shiftwidth=1 noexpandtab")
            .await
            .is_err()
        {
            println!("Error setting tab options");
        }

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
            NvimCommand::Input(input) => {
                println!("got key {}", input);
                self.input(&input).await;
            }
            NvimCommand::SetBuffer(lines) => {
                let Ok(buf) = nvim.get_current_buf().await else {
                    println!("Error getting buffer");
                }
                if buf.set_lines(0, -1, false, lines).await.is_err() {
                    println!("Error setting buffer");
                }
            }
            NvimCommand::SetCursor(row, col) => {
                let win = nvim.get_current_win().await.unwrap();
                if win.set_cursor((row as i64, col as i64)).await.is_err() {
                    println!("Error setting cursor");
                }
            }
            NvimCommand::SetPath(path) => {
                let buf = nvim.get_current_buf().await.unwrap();
                if buf.set_name(&path).await.is_err() {
                    println!("Error setting buffer path");
                }
            }
        }
    }
}

#[derive(Clone)]
struct NvimHandler {
    state: Arc<Mutex<EditorState>>,
}

impl NvimHandler {
    fn handle_redraw(&self, args: &[Value], state: &mut EditorState) {
        for event in args {
            if let Some(array) = event.as_array() {
                if let Some(Value::String(event_name)) = array.first() {
                    match event_name.as_str() {
                        Some("mode_change") => self.handle_mode_change(array, state),
                        Some("cursor_goto") => self.handle_cursor_goto(array, state),
                        _ => {}
                    }
                }
            }
        }
    }

    fn handle_mode_change(&self, array: &[Value], state: &mut EditorState) {
        if let Some(inner) = array.get(1).and_then(|v| v.as_array()) {
            if let Some(mode) = inner.first().and_then(|v| v.as_str()) {
                state.mode = mode.to_string();
            }
        }
    }

    fn handle_cursor_goto(&self, array: &[Value], state: &mut EditorState) {
        if let Some(inner) = array.get(1).and_then(|v| v.as_array()) {
            let row = inner.first().and_then(|v| v.as_i64()).unwrap_or(0) + 1;
            let col = inner.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
            state.cursor = (row, col);
        }
    }

    fn handle_buf_lines_event(&self, args: &[Value], state: &mut EditorState) {
        let first_line = args[2].as_i64().unwrap_or(0) as usize;
        let last_line = args[3].as_i64().unwrap_or(-1);
        let new_lines: Vec<String> = args[4]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        if last_line == -1 {
            state.lines = new_lines;
        } else {
            state
                .lines
                .splice(first_line..last_line as usize, new_lines);
        }
    }
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
        let mut state = self.state.lock().unwrap();
        match _name.as_str() {
            "redraw" => self.handle_redraw(&_args, &mut state),
            "nvim_buf_lines_event" => self.handle_buf_lines_event(&_args, &mut state),
            _ => {}
        }
    }
}
