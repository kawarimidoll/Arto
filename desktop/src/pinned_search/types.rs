use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a pinned search
pub type PinnedSearchId = String;

/// A pinned search query with color and settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedSearch {
    pub id: PinnedSearchId,
    pub pattern: String,
    pub color: HighlightColor,
    pub case_sensitive: bool,
    #[serde(default)]
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
}

impl PinnedSearch {
    /// Create a new pinned search with auto-generated ID
    pub fn new(pattern: impl Into<String>, color: HighlightColor, case_sensitive: bool) -> Self {
        Self {
            id: Self::generate_id(),
            pattern: pattern.into(),
            color,
            case_sensitive,
            disabled: false,
            created_at: Utc::now(),
        }
    }

    /// Generate a unique ID for a pinned search
    fn generate_id() -> PinnedSearchId {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("ps_{}", timestamp)
    }
}

/// Highlight color for pinned searches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlightColor {
    #[default]
    Yellow,
    Green,
    Blue,
    Pink,
    Orange,
}

impl HighlightColor {
    /// All available colors
    pub const ALL: [HighlightColor; 5] = [
        HighlightColor::Yellow,
        HighlightColor::Green,
        HighlightColor::Blue,
        HighlightColor::Pink,
        HighlightColor::Orange,
    ];

    /// Get the CSS class name for this color
    pub fn css_class(&self) -> &'static str {
        match self {
            HighlightColor::Yellow => "highlight-yellow",
            HighlightColor::Green => "highlight-green",
            HighlightColor::Blue => "highlight-blue",
            HighlightColor::Pink => "highlight-pink",
            HighlightColor::Orange => "highlight-orange",
        }
    }

    /// Get the next color to use based on current usage
    /// Returns the least used color to distribute colors evenly
    pub fn next_color(pinned: &[PinnedSearch]) -> Self {
        use std::collections::HashMap;

        let mut usage: HashMap<HighlightColor, usize> = HashMap::new();
        for p in pinned {
            *usage.entry(p.color).or_insert(0) += 1;
        }

        Self::ALL
            .iter()
            .min_by_key(|&color| usage.get(color).unwrap_or(&0))
            .copied()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_search_creation() {
        let search = PinnedSearch::new("TODO", HighlightColor::Orange, false);
        assert_eq!(search.pattern, "TODO");
        assert_eq!(search.color, HighlightColor::Orange);
        assert!(!search.case_sensitive);
        assert!(!search.disabled);
        assert!(search.id.starts_with("ps_"));
    }

    #[test]
    fn test_highlight_color_serialization() {
        let color = HighlightColor::Yellow;
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(json, r#""yellow""#);

        let color = HighlightColor::Orange;
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(json, r#""orange""#);
    }

    #[test]
    fn test_next_color() {
        let pinned = vec![
            PinnedSearch::new("TODO", HighlightColor::Yellow, false),
            PinnedSearch::new("FIXME", HighlightColor::Yellow, false),
            PinnedSearch::new("API", HighlightColor::Blue, false),
        ];

        // Yellow is used 2 times, Blue 1 time, others 0 times
        // Should return Green, Pink, or Orange (all have 0 usage)
        let next = HighlightColor::next_color(&pinned);
        assert!(matches!(
            next,
            HighlightColor::Green | HighlightColor::Pink | HighlightColor::Orange
        ));
    }

    #[test]
    fn test_css_class() {
        assert_eq!(HighlightColor::Yellow.css_class(), "highlight-yellow");
        assert_eq!(HighlightColor::Green.css_class(), "highlight-green");
    }
}
