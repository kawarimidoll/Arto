use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInfo {
    pub index: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    pub context: String,
    pub pattern: String,
    pub color: String,
}
