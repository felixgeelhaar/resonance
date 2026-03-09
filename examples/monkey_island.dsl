tempo 120

arrangement [intro x1, theme_a x1, theme_b x1, dev_a x1, dev_b x1, theme_a x1, theme_b x1, ending x1]

track drums {
    kit: default
    section intro [2 bars] {
        kick:  [X . . . . . X . X . . . . . . .]
        hat:   [. . X . . . X . . . X . . . X .]
    }
    section theme_a [2 bars] {
        kick:  [X . . . X . . . X . . . . . X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X . X . X . X . X . X . X . X .]
    }
    section theme_b [2 bars] {
        kick:  [X . . . X . . . X . . . . . X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X . X . X . X . X . X . X . X .]
    }
    section dev_a [2 bars] {
        kick:  [X . . . . . X . X . . . . . X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X X X X X X X X X X X X X X X X]
        clap:  [. . . . . . . . . . . . X . . .]
    }
    section dev_b [2 bars] {
        kick:  [X . . . . . X . X . . . . . X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X X X X X X X X X X X X X X X X]
        clap:  [. . . . . . . . . . . . X . . .]
    }
    section ending [2 bars] {
        kick:  [X . . . X . . . X . . . . . . .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X . X . X . X . X . X . X . X .]
    }
}

track bass {
    bass
    section intro [2 bars] {
        line: [. . E2 E2 E2 . . . . D2 D2 . E2 . . .] vel [. . x x x . . . . x x . x . . .]
    }
    section theme_a [2 bars] {
        line: [. D2 D2 E2 . . . . . D2 D2 C2 C2 C2 G2 G2] vel [. x x x . . . . . x x x x x x x]
    }
    section theme_b [2 bars] {
        line: [. . . . A2 A2 E2 E2 E2 . . . . . D2 D2] vel [. . . . x x x x x . . . . . x x]
    }
    section dev_a [2 bars] {
        line: [D2 C2 C2 C2 . . G2 G2 G2 A2 A2 A2 D2 . . .] vel [x x x x . . x x x x x x x . . .]
    }
    section dev_b [2 bars] {
        line: [. G2 G2 G2 D#2 D#2 D#2 E2 . . . . G2 G2 G2 B2] vel [. x x x x x x x . . . . x x x x]
    }
    section ending [2 bars] {
        line: [B2 B2 E2 . . . . C2 C2 C2 C2 . . . . G2] vel [x x x . . . . x x x x . . . . x]
    }
}

track melody {
    fm
    section intro [2 bars] {
        lead: [. . . . . . . . . . . . . . . .]
    }
    section theme_a [2 bars] {
        lead: [. . . . . E5 E5 G5 F#5 E5 D5 E5 . . . D5]
    }
    section theme_b [2 bars] {
        lead: [D5 C5 B4 D5 C5 C5 B4 . . . E5 E5 . G5 F#5 E5]
    }
    section dev_a [2 bars] {
        lead: [. E5 . . . . F#5 G5 G5 A5 . . F#5 . G5 F#5]
    }
    section dev_b [2 bars] {
        lead: [D5 F#5 G5 G5 F#5 . . E5 . G5 F#5 E5 D5 F#5 G5 F#5]
    }
    section ending [2 bars] {
        lead: [. . E5 . G5 F#5 E5 D5 E5 E5 E5 . . . E5 D5]
    }
}

track arpeggio {
    wavetable: basic
    section intro [2 bars] {
        note: [. D5 E5 G4 E4 G4 E4 G3 D4 F#4 D4 E4 E4 B4 E4 G3] vel [. x x x x x x x x x x x x x x x]
    }
    section theme_a [2 bars] {
        note: [D4 F#4 D4 E4 G4 B4 G4 E4 D3 F#4 A3 E4 G4 C5 . D4] vel [x x x x x x x x x x x x x x . x]
    }
    section theme_b [2 bars] {
        note: [. . D4 . E4 E4 . E4 E4 E4 G4 B4 G4 E4 D3 F#4] vel [. . x . x x . x x x x x x x x x]
    }
    section dev_a [2 bars] {
        note: [A3 E4 G4 C5 . E4 E4 . D4 . A3 C4 D5 F#4 D5 .] vel [x x x x . x x . x . x x x x x .]
    }
    section dev_b [2 bars] {
        note: [D5 G4 B4 B4 D#4 C#5 D#5 E5 E4 E5 F#4 D5 . . G4 B4] vel [x x x x x x x x x x x x . . x x]
    }
    section ending [2 bars] {
        note: [C#5 D#5 E5 E5 . F#4 D5 E4 E4 E4 E4 C5 C5 . E4 E4] vel [x x x x . x x x x x x x x . x x]
    }
}

track chords {
    poly
    section intro [2 bars] {
        pad: [E3 . . . . . . . . . . . . . . .]
    }
    section theme_a [2 bars] {
        pad: [E3 . . . . . D3 . C3 . . . G3 . . .]
    }
    section theme_b [2 bars] {
        pad: [A3 . . . E3 . . . . . D3 . C3 . . .]
    }
    section dev_a [2 bars] {
        pad: [D3 . . . C3 . . . G3 . . . A3 . . .]
    }
    section dev_b [2 bars] {
        pad: [B3 . . . E3 . . . D3 . . . . . . .]
    }
    section ending [2 bars] {
        pad: [E3 . . . D3 . . . C3 . . . G3 . . .]
    }
}

track counter {
    fm
    section intro [2 bars] {
        hit: [. . . . . . . . . . . . . . . .]
    }
    section theme_a [2 bars] {
        hit: [. . . . . . . . . . . . . . B4 G4]
    }
    section theme_b [2 bars] {
        hit: [G4 . B4 A4 A4 . G4 . . . . . . . . .]
    }
    section dev_a [2 bars] {
        hit: [. . . C5 . G4 G4 . B4 A4 A4 . F#4 . . .]
    }
    section dev_b [2 bars] {
        hit: [. . . . . . . . . . . . . . . .]
    }
    section ending [2 bars] {
        hit: [. . . . . . . . . . . . . C5 G4 G4]
    }
}
