use crate::constants;
use crate::latex;
use tauri::{command, AppHandle, State};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

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
    state: State<'_, latex::LatexSettings>, // <--- Inject State here
    id: String,
    tex: String,
    display_mode: bool,
) -> Result<String, String> {
    let preamble = state.get_preamble();

    latex::compile(&id, &tex, display_mode, &preamble)
}

#[tauri::command]
pub fn reload_preamble_from_disk(
    app: AppHandle,
    state: State<'_, latex::LatexSettings>,
) -> Result<(), String> {
    let new_content = latex::read_preamble(&app);
    state.set_preamble(new_content);

    Ok(())
}
