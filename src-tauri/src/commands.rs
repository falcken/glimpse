use crate::constants;
use crate::latex;
use crate::latex::LatexMathCompiler;
use crate::latex::SvgResult;
use tauri::{command, AppHandle, State};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use serde::Serialize;

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


impl Serialize for SvgResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Define the mirror enum locally inside the function
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase", tag = "enum")]
        enum Mirror<'a> {
            Perfect { svg: &'a str },
            Alright { svg: &'a str, errors: &'a Vec<String> },
            Bad { errors: &'a Vec<String> },
        }

        // Map the original type to the local mirror type
        let mirror = match self {
            SvgResult::Perfect { svg } => Mirror::Perfect { svg },
            SvgResult::Alright { svg, errors } => Mirror::Alright { svg, errors },
            SvgResult::Bad { errors } => Mirror::Bad { errors },
        };

        // Serialize the mirror instance
        mirror.serialize(serializer)
    }
}

#[command]
pub async fn render_latex(
    state: State<'_, latex::LatexMathCompilerImpl>,
    tex: String,
    display_mode: bool,
) -> Result<SvgResult, ()> {
   Ok(state.math_to_svg(&tex.to_string(), display_mode).await.into())
}

#[command]
pub fn reload_preamble_from_disk(
    app: AppHandle,
    state: State<'_, latex::LatexMathCompilerImpl>,
) -> Result<(), String> {
    let new_content = latex::read_preamble(&app);
    state.set_preamble(new_content)
}
