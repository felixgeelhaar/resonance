# Resonance

A terminal-native live coding music instrument built in Rust.

Write code. Hear music. Perform live — all from your terminal.

Resonance is not a DAW, not a genre generator, not an AI composer. It is a **deterministic music engine** with a live performance interface. You write patterns in a DSL, compile them into an event stream, and perform with macros, section jumps, and quantized transitions.

## Demo

```
tempo 128

track drums {
  kit: default
  section groove [2 bars] {
    kick:  [X . . x . X . .]  vel [95 . . 50 . 90 . .]
    snare: [. X . . . . X .]
    hat:   [x x x x x x x x]  vel [60 35 50 35 70 35 50 40]
  }
}

track bass {
  bass
  section groove [2 bars] {
    note: [C2 . . C2 . . Eb2 .]  vel [90 . . 80 . . 85 .]
  }
}
```

## Features

- **Custom DSL** — declarative syntax for drums, bass, synths, and patterns
- **Live performance** — Edit and Perform modes with quantized transitions
- **Built-in instruments** — drum kit, bass synth, poly pad, pluck/keys, noise generator
- **Macro system** — map named macros to parameters with curves (linear, log, exp, smoothstep)
- **Section navigation** — jump between sections on bar boundaries
- **TUI interface** — 6-panel layout: code editor, track list, grid visualization, macro meters, intent console, status bar
- **Deterministic** — same code + same seed = same output, always
- **Real-time audio** — dedicated audio thread, lock-free ring buffer, no allocations on the hot path

## Quick Start

```bash
cargo run
```

On first run, Resonance generates a starter template based on the default style. The TUI launches with the code editor ready.

### Key Bindings

| Key | Action |
|-----|--------|
| `Ctrl-Q` | Quit |
| `Ctrl-R` | Compile & reload |
| `Ctrl-P` | Toggle Edit / Perform mode |
| `Tab` | Cycle panel focus |
| `Space` | Play / Stop (Perform mode) |
| `1`–`9` | Jump to section (Perform mode) |
| `F1`–`F8` | Adjust macro (Perform mode) |

## Building

```bash
cargo build              # debug
cargo build --release    # optimized (recommended for audio)
cargo test               # 380+ tests
cargo clippy             # lint
```

Requires Rust 1.75+ and a working audio output device (`cpal`).

## Architecture

```
DSL Source → Lexer → Parser → AST → Compiler → Event Stream → Audio Scheduler
```

Eight layers with clear boundaries:

| Layer | Role |
|-------|------|
| **TUI** | ratatui terminal interface with editor, grid, macros, status |
| **Intent** | Performance intents quantized to beat boundaries |
| **Section/Layer** | Quantized transitions, scene jumping, layer control |
| **Macro Engine** | Named macros mapped to parameters with curve functions |
| **DSL Compiler** | Lexer → Parser → AST → Event compilation |
| **Event Engine** | Deterministic scheduler with seedable randomness |
| **Instruments** | Drum kit, bass, poly, pluck, noise — all trait-based |
| **Audio Engine** | Dedicated thread, lock-free ring buffer, master limiter |

## License

MIT
