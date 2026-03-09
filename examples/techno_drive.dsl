tempo 138

macro intensity = 0.5
macro space = 0.15
macro drive = 0.6
macro acid = 0.4

map intensity -> cutoff (200.0..6000.0) exp
map space -> reverb_mix (0.0..0.35) linear
map space -> delay_mix (0.0..0.15) linear
map drive -> drive (0.0..0.9) linear
map acid -> cutoff (300.0..8000.0) exp

arrangement [intro x2, buildup x1, drop x2, break x1, drop x2, outro x1]

track drums {
    kit: default
    section intro [2 bars] {
        kick:  [X . . . X . . . X . . . X . . .]
        hat:   [. . x . . . x . . . x . . . x .]
        vel    [. . x . . . x . . . x . . . X .]
    }
    section buildup [2 bars] {
        kick:  [X . . . X . . . X . . . X . . .]
        hat:   [. . x . . . x . . . X . . . x .]
        snare: [. . . . . . . . . . . . X . . .]
        clap:  [. . . . . . . . . . . . . . X .]
        vel    [. . x . . . x . . . X . . . X X]
    }
    section drop [2 bars] {
        kick:  [X . . . X . . . X . . . X . . .]
        hat:   [. . X . . . X . . . X . . . X .]
        snare: [. . . . X . . . . . . . X . . .]
        clap:  [. . . . X . . . . . . . . . . .]
        vel    [X . X . X . X . X . X . X . X .]
    }
    section break [2 bars] {
        hat:   [. . x . . . . . . . x . . . . .]
        vel    [. . x . . . . . . . x . . . . .]
    }
    section outro [2 bars] {
        kick:  [X . . . X . . . X . . . . . . .]
        hat:   [. . x . . . x . . . x . . . . .]
        vel    [X . x . X . x . x . x . . . . .]
    }
}

track bass {
    bass
    section intro [2 bars] {
        line: [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        line: [E1 . . . . . E1 . . . . . E1 . . .]
        vel   [X . . . . . x . . . . . x . . .]
    }
    section drop [2 bars] {
        line: [E1 . . E1 . . . E1 . . E1 . . . E1 .]
        vel   [X . . x . . . X . . x . . . X .]
    }
    section break [2 bars] {
        line: [. . . . . . . . . . . . . . . .]
    }
    section outro [2 bars] {
        line: [E1 . . . . . . . . . . . . . . .]
        vel   [x . . . . . . . . . . . . . . .]
    }
}

track synth {
    fm
    section intro [2 bars] {
        stab: [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        stab: [. . . . . . . . B3 . . . . . . .]
        vel   [. . . . . . . . x . . . . . . .]
    }
    section drop [2 bars] {
        stab: [. . B3 . . . . . . . E3 . . . B3 .]
        vel   [. . X . . . . . . . x . . . X .]
    }
    section break [2 bars] {
        stab: [. . . . . . . . . . . . . . . .]
    }
    section outro [2 bars] {
        stab: [. . B3 . . . . . . . . . . . . .]
        vel   [. . x . . . . . . . . . . . . .]
    }
}

track pad {
    poly
    section intro [2 bars] {
        chord: [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        chord: [E3 . . . . . . . . . . . . . . .]
        vel    [x . . . . . . . . . . . . . . .]
    }
    section drop [2 bars] {
        chord: [E3 . . . . . . . G3 . . . . . . .]
        vel    [x . . . . . . . x . . . . . . .]
    }
    section break [2 bars] {
        chord: [E3 . . . . . . . B2 . . . . . . .]
        vel    [x . . . . . . . x . . . . . . .]
    }
    section outro [2 bars] {
        chord: [E3 . . . . . . . . . . . . . . .]
        vel    [x . . . . . . . . . . . . . . .]
    }
}

track lead {
    wavetable: basic
    section intro [2 bars] {
        acid: [. . . . . . . . . . . . . . . .]
    }
    section buildup [2 bars] {
        acid: [. . . . . . . . . . . . . . . .]
    }
    section drop [2 bars] {
        acid: [E4 . . B3 . . E4 . G4 . . E4 . . B3 .]
        vel   [X . . x . . X . X . . x . . x .]
    }
    section break [2 bars] {
        acid: [E4 . . . . . B3 . . . . . G3 . . .]
        vel   [x . . . . . x . . . . . x . . .]
    }
    section outro [2 bars] {
        acid: [E4 . . . . . . . . . . . . . . .]
        vel   [x . . . . . . . . . . . . . . .]
    }
}
