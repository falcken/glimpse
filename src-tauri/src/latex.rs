use std::process::Command;
use std::fs;

pub fn compile(id: &String, tex: &String, display_mode: bool) -> Result<String, String> {
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
            tex.to_string()
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
    String::from_utf8(dvisvgm_output.stdout).map_err(|e| e.to_string())
}