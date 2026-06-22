use serde::{Deserialize, Serialize};

fn default_repeats() -> u32 {
    1
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct SongFlowItem {
    pub title: String,
    #[serde(default)]
    pub occurrence_index: u32,
    #[serde(default = "default_repeats")]
    pub repeats: u32,
}
