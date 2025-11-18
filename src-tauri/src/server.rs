use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter};
use crate::models::UpdatePayload;
use crate::constants;

async fn update_handler(
    State(app_handle): State<AppHandle>,
    Json(payload): Json<UpdatePayload>,
) -> StatusCode {
    if let Err(e) = app_handle.emit("markdown-update", payload) {
        eprintln!("Failed to emit update event: {}", e);
    }
    StatusCode::OK
}

pub async fn start(app_handle: AppHandle) {
    let addr = SocketAddr::from(([127, 0, 0, 1], constants::SERVER_PORT));

    let app = Router::new()
        .route("/update", post(update_handler))
        .with_state(app_handle);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Glimpse server listening on http://{}", addr);
    
    if let Err(e) = axum::serve(listener, app.into_make_service()).await {
        eprintln!("Server error: {}", e);
    }
}