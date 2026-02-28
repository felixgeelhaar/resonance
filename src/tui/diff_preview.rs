//! Diff Preview — modal overlay showing proposed structural diffs.
//!
//! When visible, renders a centered overlay showing accept/reject options.

/// A single line in the diff preview.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    pub text: String,
    pub kind: DiffLineKind,
}

/// The kind of a diff line (determines rendering color).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Header,
    Addition,
    Removal,
    Modification,
    Context,
}

/// The diff preview modal overlay state.
#[derive(Debug, Clone)]
pub struct DiffPreview {
    pub visible: bool,
    pub summaries: Vec<DiffLine>,
    pub scroll_offset: usize,
}

impl DiffPreview {
    /// Create a new hidden diff preview.
    pub fn new() -> Self {
        Self {
            visible: false,
            summaries: Vec::new(),
            scroll_offset: 0,
        }
    }

    /// Show the preview with the given diff summaries.
    pub fn show(&mut self, summaries: Vec<DiffLine>) {
        self.summaries = summaries;
        self.scroll_offset = 0;
        self.visible = true;
    }

    /// Hide the preview.
    pub fn hide(&mut self) {
        self.visible = false;
        self.summaries.clear();
        self.scroll_offset = 0;
    }

    /// Scroll up by one line.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down by one line, clamped to content length.
    pub fn scroll_down(&mut self, visible_lines: usize) {
        if self.summaries.len() > visible_lines {
            let max = self.summaries.len() - visible_lines;
            if self.scroll_offset < max {
                self.scroll_offset += 1;
            }
        }
    }

    /// Get the visible lines for a given viewport height.
    pub fn visible_lines(&self, max_lines: usize) -> &[DiffLine] {
        let start = self.scroll_offset;
        let end = (start + max_lines).min(self.summaries.len());
        &self.summaries[start..end]
    }

    /// Number of summary lines.
    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    /// Whether the preview has no content.
    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }
}

impl Default for DiffPreview {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a list of human-readable change summaries into DiffLines.
pub fn summaries_to_diff_lines(summaries: &[String]) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    lines.push(DiffLine {
        text: "Proposed Changes".to_string(),
        kind: DiffLineKind::Header,
    });
    lines.push(DiffLine {
        text: "─".repeat(40),
        kind: DiffLineKind::Context,
    });

    for summary in summaries {
        let kind = classify_summary(summary);
        lines.push(DiffLine {
            text: summary.clone(),
            kind,
        });
    }

    lines.push(DiffLine {
        text: "─".repeat(40),
        kind: DiffLineKind::Context,
    });
    lines.push(DiffLine {
        text: "Enter: Accept  |  Esc: Reject".to_string(),
        kind: DiffLineKind::Context,
    });

    lines
}

/// Classify a summary string into a DiffLineKind based on keywords.
fn classify_summary(summary: &str) -> DiffLineKind {
    let lower = summary.to_lowercase();
    if lower.starts_with("added") || lower.starts_with("+ ") {
        DiffLineKind::Addition
    } else if lower.starts_with("removed") || lower.starts_with("- ") {
        DiffLineKind::Removal
    } else if lower.starts_with("changed") || lower.starts_with("~ ") {
        DiffLineKind::Modification
    } else {
        DiffLineKind::Context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_preview_is_hidden() {
        let preview = DiffPreview::new();
        assert!(!preview.visible);
        assert!(preview.is_empty());
        assert_eq!(preview.scroll_offset, 0);
    }

    #[test]
    fn show_makes_visible() {
        let mut preview = DiffPreview::new();
        preview.show(vec![DiffLine {
            text: "test".to_string(),
            kind: DiffLineKind::Context,
        }]);
        assert!(preview.visible);
        assert_eq!(preview.len(), 1);
    }

    #[test]
    fn hide_clears_state() {
        let mut preview = DiffPreview::new();
        preview.show(vec![DiffLine {
            text: "test".to_string(),
            kind: DiffLineKind::Context,
        }]);
        preview.hide();
        assert!(!preview.visible);
        assert!(preview.is_empty());
        assert_eq!(preview.scroll_offset, 0);
    }

    #[test]
    fn scroll_up_clamps_at_zero() {
        let mut preview = DiffPreview::new();
        preview.scroll_up();
        assert_eq!(preview.scroll_offset, 0);
    }

    #[test]
    fn scroll_down_clamps_at_max() {
        let mut preview = DiffPreview::new();
        let lines: Vec<DiffLine> = (0..5)
            .map(|i| DiffLine {
                text: format!("line {i}"),
                kind: DiffLineKind::Context,
            })
            .collect();
        preview.show(lines);

        // Viewport of 3 lines, 5 total → max scroll = 2
        preview.scroll_down(3);
        assert_eq!(preview.scroll_offset, 1);
        preview.scroll_down(3);
        assert_eq!(preview.scroll_offset, 2);
        preview.scroll_down(3);
        assert_eq!(preview.scroll_offset, 2); // Clamped
    }

    #[test]
    fn visible_lines_respects_scroll() {
        let mut preview = DiffPreview::new();
        let lines: Vec<DiffLine> = (0..5)
            .map(|i| DiffLine {
                text: format!("line {i}"),
                kind: DiffLineKind::Context,
            })
            .collect();
        preview.show(lines);
        preview.scroll_offset = 2;

        let visible = preview.visible_lines(3);
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[0].text, "line 2");
        assert_eq!(visible[2].text, "line 4");
    }

    #[test]
    fn summaries_to_diff_lines_classifies() {
        let summaries = vec![
            "Added track bass".to_string(),
            "Removed track drums".to_string(),
            "Changed tempo 120 → 140".to_string(),
            "Some context info".to_string(),
        ];
        let lines = summaries_to_diff_lines(&summaries);

        // Header + separator + 4 summaries + separator + instructions = 8
        assert_eq!(lines.len(), 8);
        assert_eq!(lines[0].kind, DiffLineKind::Header);
        assert_eq!(lines[2].kind, DiffLineKind::Addition);
        assert_eq!(lines[3].kind, DiffLineKind::Removal);
        assert_eq!(lines[4].kind, DiffLineKind::Modification);
        assert_eq!(lines[5].kind, DiffLineKind::Context);
    }

    #[test]
    fn show_resets_scroll() {
        let mut preview = DiffPreview::new();
        let lines: Vec<DiffLine> = (0..5)
            .map(|i| DiffLine {
                text: format!("line {i}"),
                kind: DiffLineKind::Context,
            })
            .collect();
        preview.show(lines.clone());
        preview.scroll_offset = 3;

        // Show again should reset scroll
        preview.show(lines);
        assert_eq!(preview.scroll_offset, 0);
    }

    #[test]
    fn classify_summary_keywords() {
        assert_eq!(classify_summary("Added track"), DiffLineKind::Addition);
        assert_eq!(classify_summary("+ new track"), DiffLineKind::Addition);
        assert_eq!(classify_summary("Removed drums"), DiffLineKind::Removal);
        assert_eq!(classify_summary("- old track"), DiffLineKind::Removal);
        assert_eq!(
            classify_summary("Changed tempo"),
            DiffLineKind::Modification
        );
        assert_eq!(
            classify_summary("~ modified section"),
            DiffLineKind::Modification
        );
        assert_eq!(classify_summary("other text"), DiffLineKind::Context);
    }
}
