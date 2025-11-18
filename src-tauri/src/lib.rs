pub mod commands;
pub mod latex;
pub mod models;
pub mod server;
pub mod constants;

use tauri::{Builder};
use tauri::menu::MenuBuilder;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::line_clicked,
            commands::render_latex
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            let menu = MenuBuilder::new(app)
                .text("open", "Open")
                .text("close", "Close")
                .build()
                .expect("Failed to build menu");

            
            app.set_menu(menu.clone())?;

            tauri::async_runtime::spawn(async move {
                server::start(app_handle).await;
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
