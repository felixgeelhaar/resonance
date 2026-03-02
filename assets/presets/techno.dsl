---
name: Techno
description: Driving techno — 130 BPM, minimal snare, industrial feel
genre: techno
---
tempo 130

macro feel = 0.3
macro space = 0.2
macro drive = 0.4

map feel -> cutoff (100.0..4000.0) exp
map space -> reverb_mix (0.0..0.4) linear
map space -> delay_mix (0.0..0.2) linear
map drive -> drive (0.0..0.8) linear

track drums {
  kit: default
  section intro [2 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    hat:   [x . x . x . x . x . x . x . x .]
  }
  section main [4 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    snare: [. . . . . . . . X . . . . . . .]
    hat:   [x . x . x . x . x . x . x . x .]
    clap:  [. . . . X . . . . . . . . . . .]
  }
}

track bass {
  bass
  section intro [2 bars] {
    note: [C1 . . . . . . . . . . . . . . .]
  }
  section main [4 bars] {
    note: [C1 . . C1 . . . . C1 . . . . . C1 .]
  }
}
