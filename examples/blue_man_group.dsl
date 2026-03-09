tempo 140

arrangement [intro x2, groove x2, riff_a x2, accent x1, riff_b x2, peak x2, riff_a x1, ending x1]

track drums {
    kit: default
    section intro [2 bars] {
        kick:  [X . . . X . . . X . . . X . . .]
        snare: [. . . . X . . . . . . . X . . .]
    }
    section groove [2 bars] {
        kick:  [X . . X . . X . X . . X . . X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X . X . X . X . X . X . X . X .]
    }
    section riff_a [2 bars] {
        kick:  [X . . X . . X . X . . X . . X .]
        snare: [. . . . X . . X . . . . X . . X]
        hat:   [X . X . X . X . X . X . X . X .]
        clap:  [. . . . X . . . . . . . X . . .]
    }
    section accent [2 bars] {
        kick:  [X X X . X X X . X X . . . . . .]
        snare: [X X X . X X X . X X . . . . . .]
        clap:  [X X X . X X X . X X . . . . . .]
    }
    section riff_b [2 bars] {
        kick:  [X . X . X . X . X . X . X . X .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X . X . X . X . X . X . X . X .]
    }
    section peak [2 bars] {
        kick:  [X . . X . . X . X . . X . . X .]
        snare: [. . . . X . . X . . . . X . . X]
        hat:   [X X X X X X X X X X X X X X X X]
        clap:  [. . . . X . . . . . . . X . . .]
    }
    section ending [2 bars] {
        kick:  [X . . . X . . . X . . . . . . .]
        snare: [. . . . X . . . . . . . X . . .]
    }
}

track bass {
    bass
    section intro [2 bars] {
        line: [E2 E2 E2 E2 E2 E2 E2 E2 E2 E2 E2 E2 E2 E2 E2 E2]
    }
    section groove [2 bars] {
        line: [E2 E2 E2 E2 E2 E2 E2 E2 C#2 C#2 C#2 C#2 C#2 C#2 C#2 C#2]
    }
    section riff_a [2 bars] {
        line: [G#1 G#1 F#1 F#1 G#1 G#1 F#1 F#1 G#1 G#1 F#1 F#1 E1 E1 F#1 F#1]
    }
    section accent [2 bars] {
        line: [G#1 G#1 G#1 . G#1 G#1 G#1 . G#1 G#1 . . . . . .]
    }
    section riff_b [2 bars] {
        line: [G#1 . F#1 . E1 . F#1 . G#1 . F#1 . E1 . F#1 .]
    }
    section peak [2 bars] {
        line: [G#1 G#1 F#1 F#1 G#1 G#1 F#1 F#1 G#1 G#1 F#1 F#1 E1 E1 F#1 F#1]
    }
    section ending [2 bars] {
        line: [G#1 . . . G#1 . . . . . . . . . . .]
    }
}

track riff {
    fm
    section intro [2 bars] {
        hit: [. . . . . . . . . . . . . . . .]
    }
    section groove [2 bars] {
        hit: [. . . . . . . . . . . . . . . .]
    }
    section riff_a [2 bars] {
        hit: [G#3 G#3 . F#3 F#3 F#3 F#3 . G#3 G#3 . F#3 F#3 F#3 F#3 . ]
    }
    section accent [2 bars] {
        hit: [G#3 G#3 G#3 . G#3 G#3 G#3 . G#3 G#3 . . . . . .]
    }
    section riff_b [2 bars] {
        hit: [G#3 . F#3 . E3 . F#3 . G#3 . F#3 . E3 . F#3 .]
    }
    section peak [2 bars] {
        hit: [G#3 G#3 . F#3 F#3 F#3 F#3 . G#3 G#3 . F#3 F#3 F#3 F#3 .]
    }
    section ending [2 bars] {
        hit: [G#3 . . . . . . . . . . . . . . .]
    }
}

track power {
    poly
    section intro [2 bars] {
        chord: [. . . . . . . . . . . . . . . .]
    }
    section groove [2 bars] {
        chord: [. . . . . . . . . . . . . . . .]
    }
    section riff_a [2 bars] {
        chord: [G#3 . . . F#3 . . . G#3 . . . E3 . F#3 .]
    }
    section accent [2 bars] {
        chord: [G#3 G#3 G#3 . G#3 G#3 G#3 . G#3 G#3 . . . . . .]
    }
    section riff_b [2 bars] {
        chord: [G#3 . F#3 . E3 . F#3 . G#3 . F#3 . E3 . F#3 .]
    }
    section peak [2 bars] {
        chord: [G#3 . . . F#3 . . . G#3 . . . E3 . F#3 .]
    }
    section ending [2 bars] {
        chord: [G#3 . . . . . . . . . . . . . . .]
    }
}

track lead {
    pluck
    section intro [2 bars] {
        note: [. . . . . . . . . . . . . . . .]
    }
    section groove [2 bars] {
        note: [. . . . . . . . . . . . . . . .]
    }
    section riff_a [2 bars] {
        note: [. . . . . . . . . . . . . . . .]
    }
    section accent [2 bars] {
        note: [. . . . . . . . . . . . . . . .]
    }
    section riff_b [2 bars] {
        note: [G#4 . F#4 . E4 . F#4 . G#4 . F#4 . E4 . F#4 .] vel [x . x . x . x . x . x . x . x .]
    }
    section peak [2 bars] {
        note: [G#4 . G#4 . F#4 . F#4 . G#4 . G#4 . E4 . F#4 .] vel [X . x . X . x . X . x . X . x .]
    }
    section ending [2 bars] {
        note: [G#4 . . . . . . . . . . . . . . .] vel [x . . . . . . . . . . . . . . .]
    }
}
