use tauri::{AppHandle, command};
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use crate::latex;
use crate::constants;

#[command]
pub async fn line_clicked(_app: AppHandle, line_number: u32) {
    let payload = format!("{{\"line\": {}}}\n", line_number);
    let addr = format!("127.0.0.1:{}", constants::NVIM_LISTENER_PORT);

    match TcpStream::connect(&addr).await {
        Ok(mut stream) => {
            if let Err(e) = stream.write_all(payload.as_bytes()).await {
                eprintln!("Failed to send to line number: {}", e);
            }
            let _ = stream.shutdown().await;
        }
        Err(e) => {
            eprintln!("Could not connect to listener: {}", e);
        }
    }
}

#[command]
pub async fn render_latex(
    _app: AppHandle,
    id: String,
    tex: String,
    display_mode: bool,
) -> Result<String, String> {
    latex::compile(&id, &tex, display_mode)
}