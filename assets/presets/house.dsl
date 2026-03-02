---
name: House
description: Classic 4/4 house — 124 BPM, offbeat hats, bass groove, pad
genre: house
---
tempo 124

macro feel = 0.4
macro space = 0.3

map feel -> cutoff (200.0..6000.0) exp
map space -> reverb_mix (0.0..0.5) linear
map space -> delay_mix (0.0..0.3) linear

track drums {
  kit: default
  section intro [2 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    hat:   [. . x . . . x . . . x . . . x .]
  }
  section main [4 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    snare: [. . . . X . . . . . . . X . . .]
    hat:   [. x . x . x . x . x . x . x . x]
    clap:  [. . . . X . . . . . . . X . . .]
  }
}

track bass {
  bass
  section intro [2 bars] {
    note: [C2 . . . . . . . C2 . . . . . Eb2 .]
  }
  section main [4 bars] {
    note: [C2 . . C2 . . Eb2 . F2 . . F2 . . C2 .]
  }
}

track pad {
  poly
  section main [4 bars] {
    note: [C4 . . . . . . . Eb4 . . . . . . .]
  }
}
