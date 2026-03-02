---
name: Ambient
description: Ambient textures — 85 BPM, poly pad, pluck, heavy reverb
genre: ambient
---
tempo 85

macro feel = 0.6
macro space = 0.7

map feel -> cutoff (300.0..3000.0) linear
map feel -> attack (0.05..0.8) linear
map space -> reverb_mix (0.2..0.8) linear
map space -> delay_mix (0.1..0.5) linear
map space -> delay_feedback (0.3..0.7) linear

track pad {
  poly
  section drift [4 bars] {
    note: [C4 . . . . . . . G3 . . . . . . .]
  }
  section bloom [4 bars] {
    note: [Eb4 . . . . . . . D4 . . . . . . .]
  }
}

track texture {
  pluck
  section drift [4 bars] {
    note: [. . G4 . . . C5 . . . . . . . . .]
  }
  section bloom [4 bars] {
    note: [. . . . Bb4 . . . . . G4 . . . . .]
  }
}
