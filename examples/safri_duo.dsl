tempo 137

arrangement [intro x2, buildup x1, drop x2, break x1, drop x2, outro x1]

track drums {
    kit: default
    section intro [2 bars] {
        kick:  [. . . . . . . . . . . . . . . .]
        hat:   [X . X . X . X . X . X . X . X .]
        clap:  [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        kick:  [. . . . X . . . . . . . X . . .]
        snare: [. . . . . . . . . . . . . . X .]
        hat:   [X . X . X . X . X . X . X . X .]
        clap:  [. . . . X . . . . . . . X . . .]
    }
    section drop [2 bars] {
        kick:  [X . . . X . . . X . . . X . . .]
        snare: [. . . . X . . . . . . . X . . .]
        hat:   [X . X . X . X . X . X . X . X .]
        clap:  [. . . . X . . . . . . . X . . .]
    }
    section break [2 bars] {
        kick:  [X . . . . . . . X . . . . . . .]
        hat:   [. . X . . . X . . . X . . . X .]
    }
    section outro [2 bars] {
        kick:  [X . . . X . . . . . . . . . . .]
        hat:   [X . X . X . X . . . . . . . . .]
    }
}

track bongos {
    kit: default
    section intro [2 bars] {
        hat:   [X . X X . X X . X . X X . X X .]
        clap:  [. X . . X . . X . X . . X . . X]
    }
    section buildup [2 bars] {
        hat:   [X . X X . X X . X . X X . X X .]
        clap:  [. X . . X . . X . X . . X . . X]
        snare: [. . . . . . . X . . . . . . . X]
    }
    section drop [2 bars] {
        hat:   [X X X X . X X . X X X X . X X .]
        clap:  [. X . . X . . X . X . . X . . X]
        snare: [. . . . . . . X . . . . . . . X]
    }
    section break [2 bars] {
        hat:   [X . . X . . X . X . . X . . X .]
        clap:  [. . X . . X . . . . X . . X . .]
    }
    section outro [2 bars] {
        hat:   [X . X X . X X . . . . . . . . .]
        clap:  [. X . . X . . . . . . . . . . .]
    }
}

track bass {
    bass
    section intro [2 bars] {
        line: [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        line: [D2 . . . D2 . . . C2 . . . C2 . . .] vel [x . . . x . . . x . . . x . . .]
    }
    section drop [2 bars] {
        line: [D2 . D2 . D2 . . . C2 . C2 . C2 . . .] vel [X . x . x . . . X . x . x . . .]
    }
    section break [2 bars] {
        line: [D2 . . . . . . . . . . . . . . .] vel [x . . . . . . . . . . . . . . .]
    }
    section outro [2 bars] {
        line: [D2 . . . D2 . . . . . . . . . . .] vel [x . . . x . . . . . . . . . . .]
    }
}

track synth {
    fm
    section intro [2 bars] {
        lead: [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        lead: [. . . . . . . . D4 . F4 . A4 . D5 .]
    }
    section drop [2 bars] {
        lead: [D5 . . A4 . . F4 . D5 . . A4 . . F4 .]
    }
    section break [2 bars] {
        lead: [D5 . . . . . . . . . . . . . . .]
    }
    section outro [2 bars] {
        lead: [D5 . . A4 . . . . . . . . . . . .]
    }
}

track pad {
    poly
    section intro [2 bars] {
        chord: [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        chord: [D3 . . . . . . . C3 . . . . . . .]
    }
    section drop [2 bars] {
        chord: [D3 . . . . . . . C3 . . . . . . .]
    }
    section break [2 bars] {
        chord: [D3 . . . . . . . . . . . . . . .]
    }
    section outro [2 bars] {
        chord: [D3 . . . . . . . . . . . . . . .]
    }
}
