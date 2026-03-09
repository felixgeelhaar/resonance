tempo 72

arrangement [intro x1, theme_a x2, theme_b x1, theme_a x1, ending x1]

track melody {
    fm
    section intro [2 bars] {
        lead: [E5 D#5 E5 D#5 E5 B4 D5 C5 A4 . . . C4 E4 A4 B4]
    }
    section theme_a [2 bars] {
        lead: [. . E4 G#4 B4 C5 . . E5 D#5 E5 D#5 E5 B4 D5 C5]
    }
    section theme_b [2 bars] {
        lead: [A4 . . . C4 E4 A4 B4 . . E4 G#4 B4 C5 . .]
    }
    section ending [2 bars] {
        lead: [E5 D#5 E5 D#5 E5 B4 D5 C5 A4 . . . . . . .] vel [x x x x x x x x X . . . . . . .]
    }
}

track bass {
    bass
    section intro [2 bars] {
        line: [. . . . . . . . A2 . . . . . . .] vel [. . . . . . . . x . . . . . . .]
    }
    section theme_a [2 bars] {
        line: [A2 . . . E2 . . . A2 . . . E2 . . .] vel [x . . . x . . . x . . . x . . .]
    }
    section theme_b [2 bars] {
        line: [A2 . . . A2 . . . E2 . . . E2 . . .] vel [x . . . x . . . x . . . x . . .]
    }
    section ending [2 bars] {
        line: [A2 . . . E2 . . . A2 . . . . . . .] vel [x . . . x . . . X . . . . . . .]
    }
}

track chords {
    poly
    section intro [2 bars] {
        pad: [. . . . . . . . A3 . . . . . . .]
    }
    section theme_a [2 bars] {
        pad: [A3 . . . E3 . . . A3 . . . E3 . . .]
    }
    section theme_b [2 bars] {
        pad: [A3 . . . . . . . E3 . . . . . . .]
    }
    section ending [2 bars] {
        pad: [A3 . . . E3 . . . A3 . . . . . . .]
    }
}

track counter {
    wavetable: basic
    section intro [2 bars] {
        note: [. . . . . . . . . . . . E4 . A4 .] vel [. . . . . . . . . . . . x . x .]
    }
    section theme_a [2 bars] {
        note: [A4 . E4 . A4 . C5 . E4 . G#4 . B4 . E4 .] vel [x . x . x . x . x . x . x . x .]
    }
    section theme_b [2 bars] {
        note: [A4 . C5 . E4 . A4 . G#4 . B4 . E4 . G#4 .] vel [x . x . x . x . x . x . x . x .]
    }
    section ending [2 bars] {
        note: [E4 . A4 . E4 . G#4 . A4 . . . . . . .] vel [x . x . x . x . x . . . . . . .]
    }
}
