pub mod commands;
pub mod constants;
pub mod latex;
pub mod models;
pub mod server;

use tauri::{Builder, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::line_clicked,
            commands::render_latex,
            commands::reload_preamble_from_disk
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // let menu = MenuBuilder::new(app)
            //     .text("open", "Open")
            //     .text("close", "Close")
            //     .build()
            //     .expect("Failed to build menu");

            // app.set_menu(menu.clone())?;

            tauri::async_runtime::spawn(async move {
                server::start(app_handle).await;
            });

            let initial_content = latex::read_preamble(&app.handle());
            app.manage(latex::LatexSettings::new(initial_content));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
