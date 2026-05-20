use nvim_rs::{create::tokio as create, rpc::handler::Dummy, Neovim, compat::tokio::Compat};
use tokio::process::{ChildStdin};

pub struct EditorState {
    pub lines: Vec<String>,
    pub cursor: (i64, i64),
    pub mode: String,
}

pub struct NvimSession {
    nvim: Neovim<Compat<ChildStdin>>,
    _io_handle: tokio::task::JoinHandle<Result<(), Box<nvim_rs::error::LoopError>>>,
    _child: tokio::process::Child,
}

impl NvimSession {
    pub async fn start() -> Self {
        let handler = Dummy::new();
        let (nvim, _io_handle, _child) = create::new_child_cmd(
            /* NOTE: Can maybe add basic config profiles down the line? */
            tokio::process::Command::new("nvim")
                .arg("--embed")
                .arg("--headless")
                .arg("-u")
                .arg("NONE"),
            handler,
        )
        .await
        .expect("failed to start nvim");

        Self { nvim, _io_handle, _child }
    }

    pub async fn input(&self, keys: &str) {
        self.nvim.input(keys).await.expect("input failed");
    }

    pub async fn get_state(&self) -> EditorState {
        let buf = self.nvim.get_current_buf().await.expect("get_current_buf failed");
        let lines = buf.get_lines(0, -1, false).await.expect("get_lines failed");

        let cursor = self.nvim.get_current_win().await.expect("get_current_win failed")
            .get_cursor().await.expect("get_cursor failed");

        let mode = self.nvim.get_mode().await.expect("get_mode failed");
        let mode_str = mode.iter()
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
}
