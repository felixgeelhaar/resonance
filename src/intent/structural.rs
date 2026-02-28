//! Structural intents — diff-based code changes requiring user confirmation.
//!
//! Unlike performance intents (which fire immediately on beat boundaries),
//! structural intents produce AST diffs that must be accepted or rejected
//! by the user before being applied.

use crate::dsl::diff::AstDiff;

/// The state of a structural intent.
#[derive(Debug, Clone, PartialEq)]
pub enum StructuralIntentState {
    /// Awaiting user decision.
    Pending,
    /// User accepted the diff.
    Accepted,
    /// User rejected the diff.
    Rejected,
    /// Application failed with an error.
    Failed(String),
}

/// A structural intent: a proposed code change with diff and state.
#[derive(Debug, Clone)]
pub struct StructuralIntent {
    pub id: u64,
    pub description: String,
    pub diff: AstDiff,
    pub proposed_source: String,
    pub state: StructuralIntentState,
}

/// Processes structural intents with propose/accept/reject lifecycle.
#[derive(Debug, Clone)]
pub struct StructuralIntentProcessor {
    pending: Option<StructuralIntent>,
    history: Vec<StructuralIntent>,
    next_id: u64,
}

impl StructuralIntentProcessor {
    pub fn new() -> Self {
        Self {
            pending: None,
            history: Vec::new(),
            next_id: 1,
        }
    }

    /// Propose a new structural intent. Replaces any existing pending intent.
    pub fn propose(&mut self, description: String, diff: AstDiff, proposed_source: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        // If there was a pending intent, move it to history as rejected
        if let Some(mut old) = self.pending.take() {
            old.state = StructuralIntentState::Rejected;
            self.history.push(old);
        }

        self.pending = Some(StructuralIntent {
            id,
            description,
            diff,
            proposed_source,
            state: StructuralIntentState::Pending,
        });

        id
    }

    /// Get the current pending intent, if any.
    pub fn pending(&self) -> Option<&StructuralIntent> {
        self.pending.as_ref()
    }

    /// Accept the pending intent. Returns the diff if there was a pending intent.
    pub fn accept(&mut self) -> Option<AstDiff> {
        if let Some(mut intent) = self.pending.take() {
            let diff = intent.diff.clone();
            intent.state = StructuralIntentState::Accepted;
            self.history.push(intent);
            Some(diff)
        } else {
            None
        }
    }

    /// Reject the pending intent.
    pub fn reject(&mut self) {
        if let Some(mut intent) = self.pending.take() {
            intent.state = StructuralIntentState::Rejected;
            self.history.push(intent);
        }
    }

    /// Mark the last accepted intent as failed.
    pub fn mark_failed(&mut self, error: String) {
        if let Some(last) = self.history.last_mut() {
            if last.state == StructuralIntentState::Accepted {
                last.state = StructuralIntentState::Failed(error);
            }
        }
    }

    /// Get the intent history.
    pub fn history(&self) -> &[StructuralIntent] {
        &self.history
    }

    /// Whether there is a pending intent.
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
}

impl Default for StructuralIntentProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::*;
    use crate::dsl::diff::AstChange;

    fn sample_diff() -> AstDiff {
        AstDiff {
            changes: vec![AstChange::TempoChanged {
                old: 120.0,
                new: 140.0,
            }],
        }
    }

    fn track_diff() -> AstDiff {
        AstDiff {
            changes: vec![AstChange::TrackAdded {
                track: TrackDef {
                    name: "bass".to_string(),
                    instrument: InstrumentRef::Bass,
                    sections: vec![],
                },
            }],
        }
    }

    #[test]
    fn new_processor_has_no_pending() {
        let proc = StructuralIntentProcessor::new();
        assert!(!proc.has_pending());
        assert!(proc.pending().is_none());
        assert!(proc.history().is_empty());
    }

    #[test]
    fn propose_creates_pending() {
        let mut proc = StructuralIntentProcessor::new();
        let id = proc.propose("test".to_string(), sample_diff(), "tempo 140".to_string());
        assert_eq!(id, 1);
        assert!(proc.has_pending());
        let pending = proc.pending().unwrap();
        assert_eq!(pending.id, 1);
        assert_eq!(pending.description, "test");
        assert_eq!(pending.state, StructuralIntentState::Pending);
    }

    #[test]
    fn accept_returns_diff_and_moves_to_history() {
        let mut proc = StructuralIntentProcessor::new();
        proc.propose("test".to_string(), sample_diff(), "tempo 140".to_string());

        let diff = proc.accept().unwrap();
        assert_eq!(diff.changes.len(), 1);
        assert!(!proc.has_pending());
        assert_eq!(proc.history().len(), 1);
        assert_eq!(proc.history()[0].state, StructuralIntentState::Accepted);
    }

    #[test]
    fn reject_moves_to_history() {
        let mut proc = StructuralIntentProcessor::new();
        proc.propose("test".to_string(), sample_diff(), "tempo 140".to_string());

        proc.reject();
        assert!(!proc.has_pending());
        assert_eq!(proc.history().len(), 1);
        assert_eq!(proc.history()[0].state, StructuralIntentState::Rejected);
    }

    #[test]
    fn accept_with_no_pending_returns_none() {
        let mut proc = StructuralIntentProcessor::new();
        assert!(proc.accept().is_none());
    }

    #[test]
    fn reject_with_no_pending_is_noop() {
        let mut proc = StructuralIntentProcessor::new();
        proc.reject(); // Should not panic
        assert!(proc.history().is_empty());
    }

    #[test]
    fn new_proposal_replaces_pending() {
        let mut proc = StructuralIntentProcessor::new();
        proc.propose("first".to_string(), sample_diff(), "a".to_string());
        proc.propose("second".to_string(), track_diff(), "b".to_string());

        // First should be rejected in history
        assert_eq!(proc.history().len(), 1);
        assert_eq!(proc.history()[0].description, "first");
        assert_eq!(proc.history()[0].state, StructuralIntentState::Rejected);

        // Second should be pending
        let pending = proc.pending().unwrap();
        assert_eq!(pending.description, "second");
        assert_eq!(pending.id, 2);
    }

    #[test]
    fn ids_increment() {
        let mut proc = StructuralIntentProcessor::new();
        let id1 = proc.propose("a".to_string(), sample_diff(), "x".to_string());
        proc.accept();
        let id2 = proc.propose("b".to_string(), sample_diff(), "y".to_string());
        proc.reject();
        let id3 = proc.propose("c".to_string(), sample_diff(), "z".to_string());

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn history_tracks_all_decisions() {
        let mut proc = StructuralIntentProcessor::new();
        proc.propose("a".to_string(), sample_diff(), "x".to_string());
        proc.accept();
        proc.propose("b".to_string(), track_diff(), "y".to_string());
        proc.reject();
        proc.propose("c".to_string(), sample_diff(), "z".to_string());
        proc.accept();

        assert_eq!(proc.history().len(), 3);
        assert_eq!(proc.history()[0].state, StructuralIntentState::Accepted);
        assert_eq!(proc.history()[1].state, StructuralIntentState::Rejected);
        assert_eq!(proc.history()[2].state, StructuralIntentState::Accepted);
    }

    #[test]
    fn mark_failed_updates_last_accepted() {
        let mut proc = StructuralIntentProcessor::new();
        proc.propose("a".to_string(), sample_diff(), "x".to_string());
        proc.accept();
        proc.mark_failed("apply error".to_string());

        assert_eq!(
            proc.history()[0].state,
            StructuralIntentState::Failed("apply error".to_string())
        );
    }

    #[test]
    fn mark_failed_only_affects_accepted() {
        let mut proc = StructuralIntentProcessor::new();
        proc.propose("a".to_string(), sample_diff(), "x".to_string());
        proc.reject();
        proc.mark_failed("should not change".to_string());

        // Rejected state should not change
        assert_eq!(proc.history()[0].state, StructuralIntentState::Rejected);
    }

    #[test]
    fn proposed_source_preserved() {
        let mut proc = StructuralIntentProcessor::new();
        proc.propose(
            "change tempo".to_string(),
            sample_diff(),
            "tempo 140\ntrack drums { ... }".to_string(),
        );

        let pending = proc.pending().unwrap();
        assert_eq!(pending.proposed_source, "tempo 140\ntrack drums { ... }");
    }
}
