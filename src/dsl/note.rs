//! Note name parsing — converts "C2", "Eb4", "F#3" to MIDI note numbers.

/// Parse a note name string into a MIDI note number.
///
/// Format: `<letter><optional accidental><octave>`
/// - Letter: C, D, E, F, G, A, B
/// - Accidental: # (sharp) or b (flat)
/// - Octave: -1 to 9 (C4 = middle C = MIDI 60)
pub fn parse_note_name(name: &str) -> Option<u8> {
    let chars: Vec<char> = name.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let base = match chars[0] {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };

    let mut i = 1;
    let accidental: i32 = if i < chars.len() && chars[i] == '#' {
        i += 1;
        1
    } else if i < chars.len() && chars[i] == 'b' {
        i += 1;
        -1
    } else {
        0
    };

    // Rest should be octave number (possibly negative)
    let octave_str: String = chars[i..].iter().collect();
    let octave: i32 = octave_str.parse().ok()?;

    // MIDI note = (octave + 1) * 12 + base + accidental
    // C-1 = 0, C4 = 60, A4 = 69
    let midi = (octave + 1) * 12 + base + accidental;

    if !(0..=127).contains(&midi) {
        None
    } else {
        Some(midi as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_c() {
        assert_eq!(parse_note_name("C4"), Some(60));
    }

    #[test]
    fn a4_concert() {
        assert_eq!(parse_note_name("A4"), Some(69));
    }

    #[test]
    fn c_minus_1() {
        assert_eq!(parse_note_name("C-1"), Some(0));
    }

    #[test]
    fn c2_bass_note() {
        assert_eq!(parse_note_name("C2"), Some(36));
    }

    #[test]
    fn eb2() {
        assert_eq!(parse_note_name("Eb2"), Some(39));
    }

    #[test]
    fn f_sharp_3() {
        assert_eq!(parse_note_name("F#3"), Some(54));
    }

    #[test]
    fn g9_max() {
        assert_eq!(parse_note_name("G9"), Some(127));
    }

    #[test]
    fn invalid_empty() {
        assert_eq!(parse_note_name(""), None);
    }

    #[test]
    fn invalid_letter() {
        assert_eq!(parse_note_name("X4"), None);
    }

    #[test]
    fn invalid_no_octave() {
        assert_eq!(parse_note_name("C"), None);
    }

    #[test]
    fn b_flat_3() {
        assert_eq!(parse_note_name("Bb3"), Some(58));
    }

    #[test]
    fn all_naturals_octave_4() {
        assert_eq!(parse_note_name("C4"), Some(60));
        assert_eq!(parse_note_name("D4"), Some(62));
        assert_eq!(parse_note_name("E4"), Some(64));
        assert_eq!(parse_note_name("F4"), Some(65));
        assert_eq!(parse_note_name("G4"), Some(67));
        assert_eq!(parse_note_name("A4"), Some(69));
        assert_eq!(parse_note_name("B4"), Some(71));
    }
}
