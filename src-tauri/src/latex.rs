use std::fs;
use std::process::Command;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};


const DEFAULT_PREAMBLE: &str = r#"
    \usepackage{amsmath}
    \usepackage{amssymb}
    \usepackage{amsfonts}
"#;

pub struct LatexSettings {
    preamble: Mutex<String>,
}

impl LatexSettings {
    pub fn new(initial_content: String) -> Self {
        Self {
            preamble: Mutex::new(initial_content),
        }
    }

    pub fn get_preamble(&self) -> String {
        let guard = self.preamble.lock().unwrap();
        guard.clone()
    }

    pub fn set_preamble(&self, new_content: String) {
        let mut guard = self.preamble.lock().unwrap();
        *guard = new_content;
    }
}

pub fn read_preamble(app: &AppHandle) -> String {
    match app.path().app_config_dir() {
        Ok(config_dir) => {
            let preamble_path = config_dir.join("preamble.tex");

            if preamble_path.exists() {
                match fs::read_to_string(preamble_path) {
                    Ok(content) => return content, // Found user preamble
                    Err(_) => println!("Error reading preamble file"),
                }
            }
        }
        Err(e) => println!("Could not resolve app config dir: {}", e),
    }

    // Fallback
    DEFAULT_PREAMBLE.to_string()
}

pub fn compile(
    id: &str,
    tex: &str,
    display_mode: bool,
    preamble_content: &str,
) -> Result<String, String> {
    let tex_content = format!(
        r#"
            \documentclass[dvisvgm, preview, 12pt]{{standalone}}
            \usepackage[utf8]{{inputenc}}
            % --- Preamble below ---
            {}
            % --- Input below ---
            \begin{{document}}
            {}
            \end{{document}}
        "#,
        preamble_content,
        // If not display mode, wrap in $...$ for inline math
        if display_mode {
            tex.to_string()
        } else {
            format!("${}$", tex)
        }
    );

    // Create temp dir
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

        return Err(format!("LaTeX compilation failed. See log:\n\n{}", log));
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
    String::from_utf8(dvisvgm_output.stdout).map_err(|e| e.to_string())
}