#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::process::Command;
use std::fs;
use tauri::{App, AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

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

#[tauri::command]
async fn render_latex(
    app: AppHandle,
    id: String,
    tex: String,
    display_mode: bool,
) -> Result<String, String> {

    // Should be read from config in the future!
    let preamble = r#"
        \usepackage{amsmath}
        \usepackage{amssymb}
        \usepackage{amsfonts}
    "#;

    let tex_content = format!(
        r#"
            \documentclass[dvisvgm, preview, 12pt]{{standalone}}
            \usepackage[utf8]{{inputenc}}
            \usepackage{{amsmath}}
            \usepackage{{fouriernc}}
            \usepackage{{amssymb}}
            \usepackage{{amsfonts}}
            % --- User's Custom Preamble Below ---
            {}
            % --- End of User's Preamble ---
            \begin{{document}}
            {}
            \end{{document}}
        "#,
        preamble,
        // If not display mode, wrap in $...$ for inline math
        if display_mode {
            tex
        } else {
            format!("${}$", tex)
        }
    );

    // Create a temporary directory
    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let tex_path = temp_dir.path().join(format!("{}.tex", id));
    std::fs::write(&tex_path, tex_content).map_err(|e| e.to_string())?;

    // Run latex
    let latex_output = Command::new("latex")
        .args([
            "-interaction=nonstopmode",
            "-output-directory",
            temp_dir.path().to_str().unwrap(),
            tex_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("`latex` command failed: {}", e))?;

    if !latex_output.status.success() {
        let log = fs::read_to_string(temp_dir.path().join("input.log"))
            .unwrap_or_else(|_| "Could not read LaTeX log.".to_string());

        return Err(format!(
            "LaTeX compilation failed. See log:\n\n{}",
            log)
        );
    }

    // Run dvisvgm
    let dvi_path = temp_dir.path().join(format!("{}.dvi", id));
    let dvisvgm_output = Command::new("dvisvgm")
        .args([
            "--zoom=1.1", // Seems to fix scaling issues
            "--exact-bbox",
            "--stdout",
            dvi_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("`dvisvgm` command failed: {}", e))?;

    if !dvisvgm_output.status.success() {
        return Err(format!(
            "dvisvgm conversion failed: {}",
            String::from_utf8_lossy(&dvisvgm_output.stderr)
        ));
    }

    // Return SVG
    let svg_string = String::from_utf8(dvisvgm_output.stdout)
        .map_err(|e| e.to_string())?;

    Ok(svg_string)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![line_clicked, render_latex])
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