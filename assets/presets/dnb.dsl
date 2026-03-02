---
name: Drum & Bass
description: Fast breakbeats — 170 BPM, syncopated patterns, driving bass
genre: dnb
---
tempo 170

macro feel = 0.5
macro space = 0.3

map feel -> cutoff (200.0..8000.0) exp
map space -> reverb_mix (0.0..0.4) linear
map space -> delay_mix (0.0..0.25) linear

track drums {
  kit: default
  section intro [2 bars] {
    kick:  [X . . . . . X . . . X . . . . .]
    snare: [. . . . X . . . . . . . X . . .]
    hat:   [x x x x x x x x x x x x x x x x]
  }
  section main [4 bars] {
    kick:  [X . . . . . X . . . X . . . . .]
    snare: [. . . . X . . . . . . X X . . .]
    hat:   [x x x x x x x x x x x x x x x x]
    clap:  [. . . . X . . . . . . . . . X .]
  }
}

track bass {
  bass
  section intro [2 bars] {
    note: [C2 . . . . . . . Eb2 . . . . . . .]
  }
  section main [4 bars] {
    note: [C2 . C2 . . . Eb2 . F2 . . . C2 . . .]
  }
}
