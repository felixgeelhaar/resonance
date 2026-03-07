//! DSL reference — quick reference overlay for Resonance DSL syntax.
//!
//! Follows the same pattern as HelpScreen.

use super::help::HelpLine;

/// DSL reference overlay state.
#[derive(Debug, Clone)]
pub struct DslReference {
    pub visible: bool,
    pub scroll_offset: usize,
    content: Vec<HelpLine>,
}

impl DslReference {
    pub fn new() -> Self {
        Self {
            visible: false,
            scroll_offset: 0,
            content: Self::build_content(),
        }
    }

    /// Toggle visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.scroll_offset = 0;
        }
    }

    /// Show the reference.
    pub fn show(&mut self) {
        self.visible = true;
        self.scroll_offset = 0;
    }

    /// Hide the reference.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Scroll up.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down.
    pub fn scroll_down(&mut self, max_visible: usize) {
        let max_scroll = self.content.len().saturating_sub(max_visible);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Get all content lines.
    pub fn lines(&self) -> &[HelpLine] {
        &self.content
    }

    fn build_content() -> Vec<HelpLine> {
        let mut lines = Vec::new();

        let h = |text: &str| HelpLine {
            text: text.to_string(),
            is_header: true,
        };
        let l = |text: &str| HelpLine {
            text: text.to_string(),
            is_header: false,
        };

        lines.push(h("TEMPO"));
        lines.push(l("  tempo 120              Set BPM (20-999)"));
        lines.push(l(""));

        lines.push(h("MACROS"));
        lines.push(l("  macro feel = 0.5       Define macro (0.0-1.0)"));
        lines.push(l("  macro space = 0.3      Multiple macros allowed"));
        lines.push(l(""));

        lines.push(h("MAPPINGS"));
        lines.push(l("  map MACRO -> PARAM (MIN..MAX) CURVE"));
        lines.push(l("  map feel -> cutoff (200.0..6000.0) exp"));
        lines.push(l("  map space -> reverb_mix (0.0..0.8) linear"));
        lines.push(l("  Curves: linear, exp, log, smoothstep"));
        lines.push(l(""));

        lines.push(h("TRACKS"));
        lines.push(l("  track NAME { TYPE sections... }"));
        lines.push(l("  Types: kit: default, bass, poly, pluck, noise"));
        lines.push(l(""));

        lines.push(h("DRUM PATTERNS"));
        lines.push(l("  Voices: kick, snare, hat, clap, tom, rim"));
        lines.push(l("  Pattern: [X . . . X . . . X . . . X . . .]"));
        lines.push(l("  X = loud hit, x = soft hit, . = silence"));
        lines.push(l("  16 steps per bar (each step = 1 sixteenth note)"));
        lines.push(l(""));

        lines.push(h("NOTE PATTERNS"));
        lines.push(l("  note: [C2 . . C2 . . Eb2 . F2 . . .]"));
        lines.push(l("  Notes: C D E F G A B (+ # or b for sharps/flats)"));
        lines.push(l("  Octaves: 0-8 (lower = deeper)"));
        lines.push(l(""));

        lines.push(h("SECTIONS"));
        lines.push(l("  section NAME [N bars] { patterns... }"));
        lines.push(l("  section intro [2 bars] { ... }"));
        lines.push(l("  section main [4 bars] { ... }"));
        lines.push(l("  In Perform mode: 1-9 to jump sections"));
        lines.push(l(""));

        lines.push(h("LAYERS"));
        lines.push(l("  layer NAME { MACRO -> PARAM (MIN..MAX) CURVE }"));
        lines.push(l("  layer fx { filter -> cutoff (0.0..1.0) linear }"));
        lines.push(l("  In Perform mode: Shift+1-9 to toggle layers"));
        lines.push(l(""));

        lines.push(h("PARAMETERS"));
        lines.push(l("  cutoff        Filter frequency"));
        lines.push(l("  reverb_mix    Reverb wet/dry (0.0-1.0)"));
        lines.push(l("  reverb_decay  Reverb tail length"));
        lines.push(l("  delay_mix     Delay wet/dry (0.0-1.0)"));
        lines.push(l("  delay_feedback Delay feedback amount"));
        lines.push(l("  drive         Distortion amount"));
        lines.push(l("  attack        Note attack time"));
        lines.push(l(""));

        lines.push(h("FUNCTIONAL CHAIN SYNTAX"));
        lines.push(l("  name = instrument() |> target.method(args)"));
        lines.push(l("  drums = kit(\"default\")"));
        lines.push(l("    |> kick.pattern(\"X.x.X.x.\")"));
        lines.push(l("    |> snare.pattern(\"..X...X.\")"));
        lines.push(l("  Methods: .pattern(\"...\"), .at([1,3]), .every(N)"));
        lines.push(l(""));

        lines.push(h("PATTERN TRANSFORMS"));
        lines.push(l("  .fast(N)          Repeat pattern N times (speed up)"));
        lines.push(l(
            "  .slow(N)          Keep first 1/N of pattern (slow down)",
        ));
        lines.push(l("  .rev()            Reverse pattern order"));
        lines.push(l("  .rotate(N)        Shift steps right by N positions"));
        lines.push(l("  .degrade(P)       Remove steps randomly (P = 0.0-1.0)"));
        lines.push(l("  .chop(N)          Subdivide each step into N parts"));
        lines.push(l("  .stutter(N)       Repeat each hit N times in place"));
        lines.push(l("  .every(N, T)      Apply transform T every N cycles"));
        lines.push(l(
            "  .sometimes(P, T)  Apply transform T with probability P",
        ));
        lines.push(l("  .add(N)           Transpose notes by N semitones"));
        lines.push(l("  .gain(F)          Scale velocity by factor F"));
        lines.push(l("  .legato(F)        Scale duration by factor F"));
        lines.push(l("  Example: kick.pattern(\"X.x.\").fast(2).rev()"));
        lines.push(l(""));

        lines.push(h("EXAMPLE"));
        lines.push(l("  tempo 124"));
        lines.push(l("  macro feel = 0.4"));
        lines.push(l("  map feel -> cutoff (200.0..6000.0) exp"));
        lines.push(l("  track drums {"));
        lines.push(l("    kit: default"));
        lines.push(l("    section main [4 bars] {"));
        lines.push(l("      kick:  [X . . . X . . . X . . . X . . .]"));
        lines.push(l("      snare: [. . . . X . . . . . . . X . . .]"));
        lines.push(l("      hat:   [. x . x . x . x . x . x . x . x]"));
        lines.push(l("    }"));
        lines.push(l("  }"));

        lines
    }
}

impl Default for DslReference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hidden() {
        let r = DslReference::new();
        assert!(!r.visible);
    }

    #[test]
    fn toggle_shows_and_hides() {
        let mut r = DslReference::new();
        r.toggle();
        assert!(r.visible);
        r.toggle();
        assert!(!r.visible);
    }

    #[test]
    fn content_not_empty() {
        let r = DslReference::new();
        assert!(!r.lines().is_empty());
    }

    #[test]
    fn has_section_headers() {
        let r = DslReference::new();
        let headers: Vec<_> = r.lines().iter().filter(|l| l.is_header).collect();
        assert!(headers.len() >= 5);
    }

    #[test]
    fn scroll_bounds() {
        let mut r = DslReference::new();
        r.scroll_up(); // Should not underflow
        assert_eq!(r.scroll_offset, 0);

        for _ in 0..200 {
            r.scroll_down(10);
        }
        assert!(r.scroll_offset <= r.lines().len());
    }

    #[test]
    fn toggle_resets_scroll() {
        let mut r = DslReference::new();
        r.show();
        r.scroll_down(5);
        assert!(r.scroll_offset > 0);
        r.hide();
        r.toggle();
        assert_eq!(r.scroll_offset, 0);
    }
}
