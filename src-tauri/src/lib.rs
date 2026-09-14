pub mod commands;
pub mod constants;
pub mod latex;
pub mod models;
pub mod server;

use tauri::{Builder, Manager};

pub fn initialize_local_data(base_path: &std::path::PathBuf) {
    if std::fs::create_dir_all(base_path.join("tex")).is_err() ||
       std::fs::create_dir_all(base_path.join("svg")).is_err() {

        todo!("handle create dir failure");
    }
}

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
            
            let base_path = match app_handle.path().app_local_data_dir() {
                Ok(path) => path.clone(),
                Err(_) => todo!("Handle didn't find local data dir")
            };

            initialize_local_data(&base_path);

            tauri::async_runtime::spawn(async move {
                server::start(app_handle).await;
            });

            let initial_preamble = latex::read_preamble(&app.handle());
            app.manage(latex::LatexMathCompilerImpl::new(base_path, initial_preamble));


            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
