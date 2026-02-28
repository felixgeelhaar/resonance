//! Taste bias — scores proposals based on user preferences.

use super::profile::TasteProfile;

/// A score indicating how well a proposal matches user taste.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BiasScore(pub f64);

impl BiasScore {
    /// Neutral score — no bias applied.
    pub fn neutral() -> Self {
        Self(0.0)
    }

    /// Whether this score indicates a positive bias.
    pub fn is_positive(&self) -> bool {
        self.0 > 0.0
    }

    /// Whether this score indicates a negative bias.
    pub fn is_negative(&self) -> bool {
        self.0 < 0.0
    }
}

/// Scores proposals based on the taste profile.
#[derive(Debug, Clone)]
pub struct TasteBias {
    acceptance_weight: f64,
    rejection_weight: f64,
}

impl TasteBias {
    /// Create a new bias scorer with default weights.
    pub fn new() -> Self {
        Self {
            acceptance_weight: 1.0,
            rejection_weight: -1.5,
        }
    }

    /// Score a change description against the taste profile.
    ///
    /// Positive score = similar to previously accepted changes.
    /// Negative score = similar to previously rejected changes.
    /// Zero = no data.
    pub fn score(&self, description: &str, profile: &TasteProfile) -> BiasScore {
        let lower = description.to_lowercase();
        let mut score = 0.0;

        // Check similarity to accepted patterns
        for accepted in &profile.accepted_patterns {
            if patterns_similar(&lower, &accepted.to_lowercase()) {
                score += self.acceptance_weight;
            }
        }

        // Check similarity to rejected patterns
        for rejected in &profile.rejected_patterns {
            if patterns_similar(&lower, &rejected.to_lowercase()) {
                score += self.rejection_weight;
            }
        }

        BiasScore(score)
    }

    /// Score a macro value against known preferences.
    pub fn score_macro_value(&self, name: &str, value: f64, profile: &TasteProfile) -> BiasScore {
        if let Some(pref) = profile.macro_preferences.get(name) {
            // Closer to preferred value → higher score
            let distance = (value - pref.preferred_value).abs();
            let range = (pref.max_observed - pref.min_observed).max(0.01);
            let normalized = 1.0 - (distance / range).min(1.0);
            BiasScore(normalized)
        } else {
            BiasScore::neutral()
        }
    }
}

impl Default for TasteBias {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple keyword-based similarity check between two pattern descriptions.
fn patterns_similar(a: &str, b: &str) -> bool {
    let a_words: Vec<&str> = a.split_whitespace().collect();
    let b_words: Vec<&str> = b.split_whitespace().collect();

    // At least 2 words in common (beyond stopwords)
    let stopwords = ["the", "a", "an", "to", "from", "in", "of", "and", "or"];
    let common = a_words
        .iter()
        .filter(|w| !stopwords.contains(w) && b_words.contains(w))
        .count();

    common >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taste::profile::MacroPreference;

    #[test]
    fn neutral_score() {
        let score = BiasScore::neutral();
        assert!(!score.is_positive());
        assert!(!score.is_negative());
    }

    #[test]
    fn score_with_no_history() {
        let bias = TasteBias::new();
        let profile = TasteProfile::new();
        let score = bias.score("Added track bass", &profile);
        assert_eq!(score, BiasScore::neutral());
    }

    #[test]
    fn score_matches_accepted_pattern() {
        let bias = TasteBias::new();
        let mut profile = TasteProfile::new();
        profile
            .accepted_patterns
            .push("Added track bass".to_string());

        let score = bias.score("Added track synth", &profile);
        assert!(score.is_positive(), "score should be positive: {:?}", score);
    }

    #[test]
    fn score_matches_rejected_pattern() {
        let bias = TasteBias::new();
        let mut profile = TasteProfile::new();
        profile
            .rejected_patterns
            .push("Removed track drums".to_string());

        let score = bias.score("Removed track bass", &profile);
        assert!(score.is_negative(), "score should be negative: {:?}", score);
    }

    #[test]
    fn score_macro_value_close_to_preferred() {
        let bias = TasteBias::new();
        let mut profile = TasteProfile::new();
        profile.macro_preferences.insert(
            "filter".to_string(),
            MacroPreference {
                preferred_value: 0.7,
                min_observed: 0.2,
                max_observed: 0.9,
                adjustment_count: 10,
            },
        );

        let score = bias.score_macro_value("filter", 0.7, &profile);
        assert!(score.0 > 0.9); // Very close to preferred

        let far_score = bias.score_macro_value("filter", 0.2, &profile);
        assert!(far_score.0 < score.0); // Farther from preferred
    }

    #[test]
    fn score_macro_value_no_history() {
        let bias = TasteBias::new();
        let profile = TasteProfile::new();
        let score = bias.score_macro_value("filter", 0.5, &profile);
        assert_eq!(score, BiasScore::neutral());
    }

    #[test]
    fn patterns_similar_check() {
        assert!(patterns_similar("added track bass", "added track synth"));
        assert!(!patterns_similar("added track", "removed section"));
        assert!(patterns_similar(
            "changed tempo from 120",
            "changed tempo to 140"
        ));
    }
}
