#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct UpdatePayload {
    content: String,
    #[serde(rename = "cursorLine")]
    cursor_line: u32,
    #[serde(rename = "fileName")]
    file_name: String,
}

const NVIM_LISTENER_PORT: u16 = 42070;
const SERVER_PORT: u16 = 42069;

#[tauri::command]
async fn line_clicked(line_number: u32) {
    let payload = format!("{{\"line\": {}}}\n", line_number);
    let addr = format!("127.0.0.1:{}", NVIM_LISTENER_PORT);

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

async fn update_handler(
    State(app_handle): State<AppHandle>,
    Json(payload): Json<UpdatePayload>,
) -> StatusCode {
    app_handle.emit("markdown-update", payload).unwrap();
    StatusCode::OK
}

async fn start_server(app_handle: AppHandle) {
    let addr = SocketAddr::from(([127, 0, 0, 1], SERVER_PORT));

    let app = Router::new()
        .route("/update", post(update_handler))
        .with_state(app_handle);

    // Run server
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Glimpse server listening on http://{}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![line_clicked])
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                start_server(app_handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
