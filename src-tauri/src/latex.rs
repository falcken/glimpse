use std::fs;
use std::process::Command;
use std::sync::Mutex;
use tauri::path::{BaseDirectory, PathResolver};
use tauri::{AppHandle, Manager, path};  

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


static ID_COUNTER: Mutex<i32> = Mutex::new(0);

pub fn compile(
    math_blocks: Vec<String>,
    display_mode: bool,
    preamble_content: &str,
    base_path: &std::path::PathBuf,
) -> Result<String, String> {
    let mut idInt = 0;

    {
        let mut guard = ID_COUNTER.lock().unwrap();
        idInt = guard.clone();
        *guard = idInt + 1;
    };

    let id = idInt.to_string();

    let tex_content = generate_latex_content(math_blocks, display_mode, preamble_content);

    // Create temp dir
    let tex_path = base_path.join(format!("{}.tex", id));
    std::fs::write(&tex_path, tex_content).map_err(|e| e.to_string())?;

    // Run latex
    let latex_output = Command::new("latex")
        .args([
            "-interaction=nonstopmode",
            "-output-directory",
            base_path.to_str().unwrap(),
            tex_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("`latex` command failed: {}", e))?;

    if !latex_output.status.success() {
        let log = fs::read_to_string(base_path.join(format!("{}.log", id)))
            .unwrap_or_else(|_| "Could not read LaTeX log.".to_string());

        return Err(format!("LaTeX compilation failed. See log:\n\n{}", log));
    }

    // Run dvisvgm
    let dvi_path = base_path.join(format!("{}.dvi", id));
    let dvisvgm_output = Command::new("dvisvgm")
        .args([
            "--zoom=1.1", // Seems to fix scaling issues
            "--exact-bbox",
            "--no-fonts",
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

// pub fn compile_multi(
//     ids: [&str],
//     tex: [&str],
//     display_mode: bool,
//     preamble_content: &str,
//     base_path: &std::path::PathBuf,
// ) -> Result<String, String> {
//     let tex_content = generate_latex_content(tex, display_mode, preamble_content);
//
//     // Create temp dir
//     let tex_path = base_path.join(format!("{}.tex", id));
//     std::fs::write(&tex_path, tex_content).map_err(|e| e.to_string())?;
//
//     // Run latex
//     let latex_output = Command::new("latex")
//         .args([
//             "-interaction=nonstopmode",
//             "-output-directory",
//             base_path.to_str().unwrap(),
//             tex_path.to_str().unwrap(),
//         ])
//         .output()
//         .map_err(|e| format!("`latex` command failed: {}", e))?;
//
//     if !latex_output.status.success() {
//         let log = fs::read_to_string(base_path.join("input.log"))
//             .unwrap_or_else(|_| "Could not read LaTeX log.".to_string());
//
//         return Err(format!("LaTeX compilation failed. See log:\n\n{}", log));
//     }
//
//     // Run dvisvgm
//     let dvi_path = base_path.join(format!("{}.dvi", id));
//     let dvisvgm_output = Command::new("dvisvgm")
//         .args([
//             "--zoom=1.1", // Seems to fix scaling issues
//             "--exact-bbox",
//             "--no-fonts",
//             "--stdout",
//             dvi_path.to_str().unwrap(),
//         ])
//         .output()
//         .map_err(|e| format!("`dvisvgm` command failed: {}", e))?;
//
//     if !dvisvgm_output.status.success() {
//         return Err(format!(
//             "dvisvgm conversion failed: {}",
//             String::from_utf8_lossy(&dvisvgm_output.stderr)
//         ));
//     }
//
//     // Return SVG
//     String::from_utf8(dvisvgm_output.stdout).map_err(|e| e.to_string())
// }
//
fn create_equation_page(math: &str) -> String {
    return format!("\\begin{{page}}\\begin{{equation*}}\\textcolor[RGB]{{244, 244, 244}}{{\\rule[0pt]{{1pt}}{{1pt}}}}\n{}\n\\end{{equation*}}\n\\end{{page}}", math);
}

fn generate_latex_content(math_blocks: Vec<String>, display_mode: bool, preamble_content: &str) -> String {
    let pages: Vec<String> = math_blocks.iter().map(|math| create_equation_page(math.as_str())).collect();
    let body = pages.join("\n");

    format!(
        r#"
            \documentclass[dvisvgm, preview, 12pt, multi=page]{{standalone}}
            \usepackage[utf8]{{inputenc}}
            \usepackage{{xcolor}}
            % --- Preamble below ---
            {}
            % --- Input below ---
            \begin{{document}}
            {}
            \end{{document}}
        "#,
        preamble_content,
        // TODO: make it do text-size when not in display mode
        format!("\\begin{{equation*}}\\textcolor[RGB]{{244, 244, 244}}{{\\rule[0pt]{{1pt}}{{1pt}}}}\n{}\n\\end{{equation*}}", body.as_str())
    )
}
