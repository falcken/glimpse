use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdatePayload {
    content: String,
    #[serde(rename = "cursorLine")]
    cursor_line: u32,
    #[serde(rename = "fileName")]
    file_name: String,
}