//! Rule-based natural language parser — ~30 keyword commands for music control.
//!
//! Parses plain English input into structured commands that can be dispatched
//! by the TUI. Falls through to `Unknown` for inputs that don't match any rule.

use crate::dsl::Compiler;

/// A command parsed from natural language input.
#[derive(Debug, Clone, PartialEq)]
pub enum NlCommand {
    /// Set tempo to absolute value.
    SetTempo(f64),
    /// Adjust tempo by relative delta.
    AdjustTempo(f64),
    /// Adjust a named macro by delta.
    AdjustMacro { name: String, delta: f64 },
    /// Toggle play/stop.
    TogglePlayback,
    /// Structural DSL modification — proposed new source.
    ModifyDsl(String),
    /// Load a named preset.
    LoadPreset(String),
    /// Start the tutorial.
    StartTutorial,
    /// Show help screen.
    ShowHelp,
    /// Show DSL reference.
    ShowReference,
    /// Unrecognized input — falls through to LLM if available.
    Unknown(String),
}

/// Parse natural language input into a command.
pub fn parse(input: &str, current_source: &str) -> NlCommand {
    let lower = input.trim().to_lowercase();

    if lower.is_empty() {
        return NlCommand::Unknown(input.to_string());
    }

    // Tempo commands
    if let Some(cmd) = parse_tempo(&lower) {
        return cmd;
    }

    // Playback
    if matches!(lower.as_str(), "play" | "start" | "go") {
        return NlCommand::TogglePlayback;
    }
    if matches!(lower.as_str(), "stop" | "pause") {
        return NlCommand::TogglePlayback;
    }

    // Macro adjustments
    if let Some(cmd) = parse_macro_adjust(&lower) {
        return cmd;
    }

    // Structural DSL modifications
    if let Some(cmd) = parse_structural(&lower, current_source) {
        return cmd;
    }

    // Preset loading
    if let Some(name) = lower.strip_prefix("load ") {
        return NlCommand::LoadPreset(name.trim().to_string());
    }
    if let Some(name) = lower.strip_prefix("preset ") {
        return NlCommand::LoadPreset(name.trim().to_string());
    }

    // Navigation
    if lower == "tutorial" || lower == "learn" {
        return NlCommand::StartTutorial;
    }
    if lower == "help" {
        return NlCommand::ShowHelp;
    }
    if lower == "reference" || lower == "ref" || lower == "syntax" {
        return NlCommand::ShowReference;
    }

    NlCommand::Unknown(input.to_string())
}

fn parse_tempo(lower: &str) -> Option<NlCommand> {
    // "faster", "speed up"
    if lower == "faster" || lower == "speed up" {
        return Some(NlCommand::AdjustTempo(10.0));
    }
    if lower == "slower" || lower == "slow down" {
        return Some(NlCommand::AdjustTempo(-10.0));
    }
    if lower == "half time" {
        return Some(NlCommand::AdjustTempo(-0.5));
    }
    if lower == "double time" {
        return Some(NlCommand::AdjustTempo(2.0));
    }

    // "tempo NNN", "bpm NNN"
    let numeric = lower
        .strip_prefix("tempo ")
        .or_else(|| lower.strip_prefix("bpm "));
    if let Some(num_str) = numeric {
        if let Ok(bpm) = num_str.trim().parse::<f64>() {
            return Some(NlCommand::SetTempo(bpm));
        }
    }

    // "NNN bpm"
    if let Some(prefix) = lower.strip_suffix(" bpm") {
        if let Ok(bpm) = prefix.trim().parse::<f64>() {
            return Some(NlCommand::SetTempo(bpm));
        }
    }

    None
}

fn parse_macro_adjust(lower: &str) -> Option<NlCommand> {
    // Reverb/space
    if lower == "more reverb" || lower == "add reverb" || lower == "wet" || lower == "wetter" {
        return Some(NlCommand::AdjustMacro {
            name: "space".to_string(),
            delta: 0.1,
        });
    }
    if lower == "dry" || lower == "drier" || lower == "less reverb" {
        return Some(NlCommand::AdjustMacro {
            name: "space".to_string(),
            delta: -0.1,
        });
    }

    // Filter/feel
    if lower == "darker" || lower == "muffled" || lower == "warmer" {
        return Some(NlCommand::AdjustMacro {
            name: "feel".to_string(),
            delta: -0.1,
        });
    }
    if lower == "brighter" || lower == "open" || lower == "crisp" {
        return Some(NlCommand::AdjustMacro {
            name: "feel".to_string(),
            delta: 0.1,
        });
    }

    // Drive/intensity
    if lower == "more drive" || lower == "harder" || lower == "more distortion" {
        return Some(NlCommand::AdjustMacro {
            name: "drive".to_string(),
            delta: 0.1,
        });
    }
    if lower == "softer" || lower == "gentler" || lower == "less drive" || lower == "cleaner" {
        return Some(NlCommand::AdjustMacro {
            name: "drive".to_string(),
            delta: -0.1,
        });
    }

    // Compound: "make it ambient"
    if lower.contains("make it ambient") || lower.contains("ambient vibe") {
        return Some(NlCommand::AdjustMacro {
            name: "space".to_string(),
            delta: 0.3,
        });
    }

    None
}

fn parse_structural(lower: &str, current_source: &str) -> Option<NlCommand> {
    // Add hi-hats
    if lower.contains("add hi-hat")
        || lower.contains("add hats")
        || lower.contains("add hihat")
        || lower.contains("add hi hat")
    {
        return try_modify(add_hihats(current_source));
    }

    // Add kick
    if lower.contains("add kick") {
        return try_modify(add_kick(current_source));
    }

    // Add bass
    if lower.contains("add bass") {
        return try_modify(add_bass_track(current_source));
    }

    // Add pad
    if lower.contains("add pad") {
        return try_modify(add_pad_track(current_source));
    }

    // Deeper bass
    if lower.contains("deeper bass") || lower.contains("bass lower") {
        return try_modify(lower_bass_octave(current_source));
    }

    // 4 on the floor
    if lower.contains("4 on the floor")
        || lower.contains("four on the floor")
        || lower.contains("4 on floor")
    {
        return try_modify(set_four_on_floor(current_source));
    }

    // Breakbeat
    if lower.contains("breakbeat") || lower.contains("break beat") {
        return try_modify(set_breakbeat(current_source));
    }

    // Add snare
    if lower.contains("add snare") {
        return try_modify(add_snare(current_source));
    }

    // Add clap
    if lower.contains("add clap") {
        return try_modify(add_clap(current_source));
    }

    None
}

/// Validate proposed source compiles, return ModifyDsl if valid.
fn try_modify(proposed: String) -> Option<NlCommand> {
    if Compiler::compile(&proposed).is_ok() {
        Some(NlCommand::ModifyDsl(proposed))
    } else {
        None
    }
}

// --- DSL modification helpers ---

/// Add a hi-hat line to the first drum track section.
fn add_hihats(source: &str) -> String {
    insert_pattern_line(source, "hat", "[. x . x . x . x . x . x . x . x]")
}

/// Add a kick line to the first drum track section.
fn add_kick(source: &str) -> String {
    insert_pattern_line(source, "kick", "[X . . . X . . . X . . . X . . .]")
}

/// Add a snare line to the first drum track section.
fn add_snare(source: &str) -> String {
    insert_pattern_line(source, "snare", "[. . . . X . . . . . . . X . . .]")
}

/// Add a clap line to the first drum track section.
fn add_clap(source: &str) -> String {
    insert_pattern_line(source, "clap", "[. . . . X . . . . . . . X . . .]")
}

/// Insert a pattern line into the first section of the first drum track.
fn insert_pattern_line(source: &str, instrument: &str, pattern: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result = Vec::new();
    let mut inserted = false;

    for (i, line) in lines.iter().enumerate() {
        result.push(line.to_string());
        // Find the first pattern line inside a section, insert after it
        if !inserted {
            let trimmed = line.trim();
            let is_pattern =
                trimmed.contains('[') && trimmed.contains(']') && trimmed.contains(':');
            if is_pattern {
                // Check that the instrument isn't already present
                let already_exists = lines.iter().any(|l| {
                    let t = l.trim();
                    t.starts_with(&format!("{instrument}:"))
                        || t.starts_with(&format!("{instrument} :"))
                });
                if !already_exists {
                    // Determine indentation from the current line
                    let indent = &line[..line.len() - line.trim_start().len()];
                    result.push(format!("{indent}{instrument}: {pattern}"));
                    inserted = true;
                }
            }
        }
        // Fallback: if we didn't find a pattern line but the next line closes a section
        if !inserted && i + 1 < lines.len() {
            let next_trimmed = lines[i + 1].trim();
            if next_trimmed == "}" && line.trim().contains('{') {
                // Empty section — insert inside
                let indent = &line[..line.len() - line.trim_start().len()];
                result.push(format!("{indent}  {instrument}: {pattern}"));
                inserted = true;
            }
        }
    }

    // If we couldn't insert anywhere, return original
    if !inserted {
        return source.to_string();
    }

    result.join("\n")
}

/// Add a bass track to the source.
fn add_bass_track(source: &str) -> String {
    if source.contains("track bass") {
        return source.to_string();
    }
    format!(
        "{}\n\ntrack bass {{\n  bass\n  section main [2 bars] {{\n    note: [C2 . . C2 . . Eb2 . F2 . . F2 . . C2 .]\n  }}\n}}",
        source.trim_end()
    )
}

/// Add a pad track to the source.
fn add_pad_track(source: &str) -> String {
    if source.contains("track pad") {
        return source.to_string();
    }
    format!(
        "{}\n\ntrack pad {{\n  poly\n  section main [4 bars] {{\n    note: [C4 . . . . . . . Eb4 . . . . . . .]\n  }}\n}}",
        source.trim_end()
    )
}

/// Lower bass octave numbers by 1 (e.g., C2 → C1).
fn lower_bass_octave(source: &str) -> String {
    let mut result = String::new();
    let mut in_bass_track = false;

    for line in source.lines() {
        if line.contains("track bass") {
            in_bass_track = true;
        }
        if in_bass_track && line.trim() == "}" && !line.contains('{') {
            // Check if this closes the bass track (heuristic: unindented })
            if line.trim_start().len() == line.trim().len()
                || line.starts_with('}')
                || line.starts_with("  }")
            {
                in_bass_track = false;
            }
        }

        if in_bass_track && line.contains("note:") {
            let lowered = lower_octave_in_line(line);
            result.push_str(&lowered);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove trailing extra newline
    if result.ends_with('\n') && !source.ends_with('\n') {
        result.pop();
    }

    result
}

/// Lower octave numbers in a pattern line (e.g., C2 → C1, Eb3 → Eb2).
fn lower_octave_in_line(line: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Look for note patterns: letter + optional 'b'/'#' + digit
        if i + 1 < chars.len() && chars[i].is_ascii_uppercase() && "ABCDEFG".contains(chars[i]) {
            let start = i;
            i += 1;
            // Skip accidental
            if i < chars.len() && (chars[i] == 'b' || chars[i] == '#') {
                i += 1;
            }
            // Parse octave digit
            if i < chars.len() && chars[i].is_ascii_digit() {
                let octave = chars[i].to_digit(10).unwrap();
                let new_octave = octave.saturating_sub(1);
                for c in &chars[start..i] {
                    result.push(*c);
                }
                result.push(char::from_digit(new_octave, 10).unwrap());
                i += 1;
            } else {
                // Not a note pattern, push what we consumed
                for c in &chars[start..i] {
                    result.push(*c);
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Set the kick pattern to a 4-on-the-floor pattern.
fn set_four_on_floor(source: &str) -> String {
    let mut result = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("kick:") && trimmed.contains('[') {
            let indent = &line[..line.len() - line.trim_start().len()];
            result.push(format!("{indent}kick:  [X . . . X . . . X . . . X . . .]"));
        } else {
            result.push(line.to_string());
        }
    }
    result.join("\n")
}

/// Set the kick and snare pattern to a breakbeat pattern.
fn set_breakbeat(source: &str) -> String {
    let mut result = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("kick:") && trimmed.contains('[') {
            let indent = &line[..line.len() - line.trim_start().len()];
            result.push(format!("{indent}kick:  [X . . . . . X . . . X . . . . .]"));
        } else if trimmed.starts_with("snare:") && trimmed.contains('[') {
            let indent = &line[..line.len() - line.trim_start().len()];
            result.push(format!("{indent}snare: [. . . . X . . . . . . X X . . .]"));
        } else {
            result.push(line.to_string());
        }
    }
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASIC_SOURCE: &str = "tempo 120\n\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . . X . . . X . . . X . . .]\n  }\n}";

    #[test]
    fn parse_faster() {
        assert_eq!(parse("faster", ""), NlCommand::AdjustTempo(10.0));
        assert_eq!(parse("speed up", ""), NlCommand::AdjustTempo(10.0));
    }

    #[test]
    fn parse_slower() {
        assert_eq!(parse("slower", ""), NlCommand::AdjustTempo(-10.0));
    }

    #[test]
    fn parse_set_tempo() {
        assert_eq!(parse("tempo 140", ""), NlCommand::SetTempo(140.0));
        assert_eq!(parse("bpm 120", ""), NlCommand::SetTempo(120.0));
        assert_eq!(parse("130 bpm", ""), NlCommand::SetTempo(130.0));
    }

    #[test]
    fn parse_playback() {
        assert_eq!(parse("play", ""), NlCommand::TogglePlayback);
        assert_eq!(parse("stop", ""), NlCommand::TogglePlayback);
    }

    #[test]
    fn parse_reverb_commands() {
        assert_eq!(
            parse("more reverb", ""),
            NlCommand::AdjustMacro {
                name: "space".to_string(),
                delta: 0.1
            }
        );
        assert_eq!(
            parse("dry", ""),
            NlCommand::AdjustMacro {
                name: "space".to_string(),
                delta: -0.1
            }
        );
    }

    #[test]
    fn parse_filter_commands() {
        assert_eq!(
            parse("darker", ""),
            NlCommand::AdjustMacro {
                name: "feel".to_string(),
                delta: -0.1
            }
        );
        assert_eq!(
            parse("brighter", ""),
            NlCommand::AdjustMacro {
                name: "feel".to_string(),
                delta: 0.1
            }
        );
    }

    #[test]
    fn parse_drive_commands() {
        assert_eq!(
            parse("harder", ""),
            NlCommand::AdjustMacro {
                name: "drive".to_string(),
                delta: 0.1
            }
        );
        assert_eq!(
            parse("softer", ""),
            NlCommand::AdjustMacro {
                name: "drive".to_string(),
                delta: -0.1
            }
        );
    }

    #[test]
    fn parse_add_hihats() {
        if let NlCommand::ModifyDsl(src) = parse("add hi-hats", BASIC_SOURCE) {
            assert!(src.contains("hat:"));
            assert!(Compiler::compile(&src).is_ok());
        } else {
            panic!("expected ModifyDsl");
        }
    }

    #[test]
    fn parse_add_bass() {
        let source = "tempo 120\n\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . . X . . . X . . . X . . .]\n  }\n}";
        if let NlCommand::ModifyDsl(src) = parse("add bass", source) {
            assert!(src.contains("track bass"));
            assert!(Compiler::compile(&src).is_ok());
        } else {
            panic!("expected ModifyDsl");
        }
    }

    #[test]
    fn parse_add_pad() {
        let source = "tempo 120\n\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . . X . . . X . . . X . . .]\n  }\n}";
        if let NlCommand::ModifyDsl(src) = parse("add pad", source) {
            assert!(src.contains("track pad"));
        } else {
            panic!("expected ModifyDsl");
        }
    }

    #[test]
    fn parse_load_preset() {
        assert_eq!(
            parse("load techno", ""),
            NlCommand::LoadPreset("techno".to_string())
        );
        assert_eq!(
            parse("preset house", ""),
            NlCommand::LoadPreset("house".to_string())
        );
    }

    #[test]
    fn parse_tutorial() {
        assert_eq!(parse("tutorial", ""), NlCommand::StartTutorial);
    }

    #[test]
    fn parse_unknown() {
        match parse("something random here", "") {
            NlCommand::Unknown(s) => assert_eq!(s, "something random here"),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn add_bass_track_no_duplicate() {
        let source = "tempo 120\ntrack bass {\n  bass\n  section main [1 bars] {\n    note: [C2 . . .]\n  }\n}";
        let result = add_bass_track(source);
        assert_eq!(result, source);
    }

    #[test]
    fn four_on_floor_modifies_kick() {
        let source = "tempo 120\n\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick:  [X . X . X . X . X . X . X . X .]\n  }\n}";
        let result = set_four_on_floor(source);
        assert!(result.contains("[X . . . X . . . X . . . X . . .]"));
    }

    #[test]
    fn lower_octave_works() {
        let line = "    note: [C2 . . C2 . . Eb2 . F2 . . .]";
        let result = lower_octave_in_line(line);
        assert!(result.contains("C1"));
        assert!(result.contains("Eb1"));
        assert!(result.contains("F1"));
    }

    #[test]
    fn empty_input_returns_unknown() {
        match parse("", "") {
            NlCommand::Unknown(_) => {}
            other => panic!("expected Unknown for empty input, got {other:?}"),
        }
    }
}
