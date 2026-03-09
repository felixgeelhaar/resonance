tempo 140

macro warmth = 0.5
macro space = 0.4

map warmth -> cutoff (400.0..4000.0) linear
map space -> reverb_mix (0.1..0.6) linear
map space -> delay_mix (0.05..0.25) linear
map space -> delay_feedback (0.2..0.5) linear

arrangement [intro x1, head_a x1, head_b x1, solo x2, head_a x1, head_b x1, ending x1]

track drums {
    kit: default
    section intro [2 bars] {
        hat:   [X . x X . x X . x X . x X . x .]
        kick:  [X . . . . . . . . X . . . . . .]
    }
    section head_a [2 bars] {
        hat:   [X . x X . x X . x X . x X . x .]
        kick:  [X . . . . . . . . X . . . . . .]
        snare: [. . . . . x . . . . . x . . . .]
    }
    section head_b [2 bars] {
        hat:   [X . x X . x X . x X . x X . x .]
        kick:  [X . . . . . X . . . . . X . . .]
        snare: [. . . . x . . . x . . . . . x .]
    }
    section solo [2 bars] {
        hat:   [X x x X . x X x x X . x X x x .]
        kick:  [X . . . . . . . . X . . . . . .]
        snare: [. . x . . x . . . . . x . . x .]
        clap:  [. . . . . . . . . . . . . . . X]
    }
    section ending [2 bars] {
        hat:   [X . x X . x X . x X . . . . . .]
        kick:  [X . . . . . . . . . . . X . . .]
        snare: [. . . . . . . . x . . . . . . .]
    }
}

track bass {
    bass
    section intro [2 bars] {
        line: [D2 . . A2 . . D2 . . . . . G2 . . D2] vel [X . . x . . x . . . . . x . . x]
    }
    section head_a [2 bars] {
        line: [D2 . F2 A2 . . G2 . B2 D3 . . C2 . E2 G2] vel [X . x x . . X . x x . . X . x x]
    }
    section head_b [2 bars] {
        line: [C2 . E2 G2 . . F2 . A2 C3 . . G2 . B2 D3] vel [X . x x . . X . x x . . X . x x]
    }
    section solo [2 bars] {
        line: [D2 . E2 F2 . G2 A2 . B2 C3 . . G2 . A2 B2] vel [X . x x . x x . x x . . X . x x]
    }
    section ending [2 bars] {
        line: [D2 . . A2 . . G2 . . E2 . . D2 . . .] vel [X . . x . . x . . x . . X . . .]
    }
}

track keys {
    pluck
    section intro [2 bars] {
        chord: [. . . . . . . . . . . . . . . .]
    }
    section head_a [2 bars] {
        chord: [. . D4 . . . . . G4 . . . C4 . . .] vel [. . x . . . . . x . . . x . . .]
    }
    section head_b [2 bars] {
        chord: [. . C4 . . . . . F4 . . . . . G4 .] vel [. . x . . . . . x . . . . . x .]
    }
    section solo [2 bars] {
        chord: [. . D4 . . . G4 . . . . . C4 . . F4] vel [. . x . . . x . . . . . x . . x]
    }
    section ending [2 bars] {
        chord: [. . D4 . . . . . . . . . D3 . . .] vel [. . x . . . . . . . . . X . . .]
    }
}

track melody {
    fm
    section intro [2 bars] {
        lead: [. . . . . . . . . . . . . . . .]
    }
    section head_a [2 bars] {
        lead: [. . D5 . F5 E5 . . A4 . . C5 . B4 . .] vel [. . X . x x . . X . . x . x . .]
    }
    section head_b [2 bars] {
        lead: [E5 . . D5 C5 . . . A4 . B4 C5 D5 . . .] vel [X . . x x . . . x . x x X . . .]
    }
    section solo [2 bars] {
        lead: [D5 . E5 F5 . G5 A5 . . G5 F5 E5 D5 . C5 D5] vel [x . x x . X X . . x x x x . x X]
    }
    section ending [2 bars] {
        lead: [. . D5 . . . A4 . . . . . D4 . . .] vel [. . x . . . x . . . . . X . . .]
    }
}

track pad {
    poly
    section intro [2 bars] {
        note: [D3 . . . . . . . . . . . . . . .]
    }
    section head_a [2 bars] {
        note: [D3 . . . . . . . G3 . . . . . . .]
    }
    section head_b [2 bars] {
        note: [C3 . . . . . . . F3 . . . . . . .]
    }
    section solo [2 bars] {
        note: [D3 . . . . . . . G3 . . . C3 . . .]
    }
    section ending [2 bars] {
        note: [D3 . . . . . . . . . . . . . . .]
    }
}
