tempo 110

arrangement [intro x2, groove_a x2, groove_b x2, build x1, peak x2, groove_a x1, ending x1]

track drums {
    kit: default
    section intro [2 bars] {
        kick:  [X . . . . . X . X . . . . . X .]
        snare: [. . . . X . . . . . . . X . . .]
    }
    section groove_a [2 bars] {
        kick:  [X . . X . . X . X . . X . . X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [. . X . . . X . . . X . . . X .]
    }
    section groove_b [2 bars] {
        kick:  [X . X . . X X . X . X . . X X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X . X . X . X . X . X . X . X .]
        clap:  [. . . . . . . . . . . . X . . .]
    }
    section build [2 bars] {
        kick:  [X . X . X . X . X . X . X . X .]
        snare: [. . . . X . . . . . X . X . X .]
        hat:   [X X X X X X X X X X X X X X X X]
    }
    section peak [2 bars] {
        kick:  [X . X . . X X . X . X . . X X .]
        snare: [. . . . X . . X . . . . X . . X]
        hat:   [X . X . X . X . X . X . X . X .]
        clap:  [. . . . X . . . . . . . X . . .]
    }
    section ending [2 bars] {
        kick:  [X . . . X . . . X . . . . . . .]
        snare: [. . . . X . . . . . . . X . . .]
    }
}

track pipes_low {
    fm
    section intro [2 bars] {
        hit: [E2 . . . E2 . . . D2 . . . D2 . . .]
    }
    section groove_a [2 bars] {
        hit: [E2 . E2 . . . E2 . D2 . D2 . . . D2 .]
    }
    section groove_b [2 bars] {
        hit: [E2 E2 . E2 . . E2 . D2 D2 . D2 . . D2 .]
    }
    section build [2 bars] {
        hit: [E2 . E2 . E2 . E2 . D2 . D2 . D2 . D2 .]
    }
    section peak [2 bars] {
        hit: [E2 E2 . E2 . E2 E2 . D2 D2 . D2 . D2 D2 .]
    }
    section ending [2 bars] {
        hit: [E2 . . . E2 . . . . . . . . . . .]
    }
}

track pipes_mid {
    fm
    section intro [2 bars] {
        hit: [. . . . . . . . . . . . . . . .]
    }
    section groove_a [2 bars] {
        hit: [. . E3 . . . . . . . D3 . . . . .]
    }
    section groove_b [2 bars] {
        hit: [E3 . . E3 . . . . D3 . . D3 . . . .]
    }
    section build [2 bars] {
        hit: [E3 . E3 . . . E3 . D3 . D3 . . . D3 .]
    }
    section peak [2 bars] {
        hit: [E3 . E3 . G3 . E3 . D3 . D3 . F3 . D3 .]
    }
    section ending [2 bars] {
        hit: [E3 . . . . . . . . . . . . . . .]
    }
}

track pipes_high {
    pluck
    section intro [2 bars] {
        note: [. . . . . . . . . . . . . . . .]
    }
    section groove_a [2 bars] {
        note: [. . . . . . . . . . . . . . . .]
    }
    section groove_b [2 bars] {
        note: [. . . . E4 . . . . . . . D4 . . .] vel [. . . . x . . . . . . . x . . .]
    }
    section build [2 bars] {
        note: [E4 . . . G4 . . . D4 . . . F4 . . .] vel [x . . . x . . . x . . . x . . .]
    }
    section peak [2 bars] {
        note: [E4 . G4 . B4 . G4 . D4 . F4 . A4 . F4 .] vel [X . x . x . x . X . x . x . x .]
    }
    section ending [2 bars] {
        note: [E4 . . . . . . . . . . . . . . .] vel [x . . . . . . . . . . . . . . .]
    }
}

track bass {
    bass
    section intro [2 bars] {
        line: [E1 . . . . . . . D1 . . . . . . .]
    }
    section groove_a [2 bars] {
        line: [E1 . . . E1 . . . D1 . . . D1 . . .]
    }
    section groove_b [2 bars] {
        line: [E1 . E1 . . . E1 . D1 . D1 . . . D1 .]
    }
    section build [2 bars] {
        line: [E1 . E1 . E1 . E1 . D1 . D1 . D1 . D1 .]
    }
    section peak [2 bars] {
        line: [E1 . E1 . . E1 . . D1 . D1 . . D1 . .]
    }
    section ending [2 bars] {
        line: [E1 . . . . . . . . . . . . . . .]
    }
}
