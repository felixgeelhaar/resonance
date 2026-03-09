# Resonance

A terminal-native live coding music instrument built in Rust.

Write code. Hear music. Perform live — all from your terminal.

Resonance is not a DAW, not a genre generator, not an AI composer. It is a **deterministic music engine** with a live performance interface. You write patterns in a DSL, compile them into an event stream, and perform with macros, section jumps, and quantized transitions.

## Install

```bash
brew tap felixgeelhaar/resonance
brew install resonance
```

Or build from source:

```bash
cargo install --path .
```

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

- **Custom DSL** — declarative and functional chain syntax for drums, bass, synths, and patterns
- **Mini-notation** — `[X.]*3` group repeat, `X!3` element repeat, `E(3,8)` Euclidean, `?` random, `<>` alternation, `^N` ratchet
- **12 pattern transforms** — `.fast(2)`, `.slow(2)`, `.rev()`, `.rotate(N)`, `.degrade(p)`, `.every(N, transform)`, `.chop(N)`, `.stutter(N)`, `.gain(x)`, `.legato(x)`, and more
- **8 built-in instruments** — drum kit, bass synth, poly pad, pluck/keys, noise generator, FM synth, wavetable synth
- **Effects** — reverb, delay, drive, sidechain compression, master limiter
- **Macro system** — map named macros to parameters with curves (linear, log, exp, smoothstep)
- **Arrangement system** — `arrangement [intro x2, verse x4, chorus x2]` with quantized transitions
- **Section & layer control** — jump between sections on bar boundaries, enable/disable layers
- **Intent system** — performance intents (quantized) and structural intents (diff-based with preview)
- **Taste engine** — opt-in learning from your editing patterns, stored at `~/.resonance/taste.yaml`
- **MIDI I/O** — MIDI input for controllers, MIDI output for external synths
- **OSC support** — receive OSC messages for remote control
- **WAV export** — offline render to WAV files
- **Plugin API** — custom instruments via YAML manifests and WAV samples
- **Instrument packs** — distributable bundles of kits, plugins, and presets
- **TUI interface** — 6-panel layout with syntax highlighting, grid visualization, VU meter, beat pulse
- **5 visual themes** — Default, Catppuccin, Gruvbox, Minimal, Strudel (or create your own in YAML)
- **Deterministic** — same code + same seed = same output, always
- **Real-time audio** — dedicated audio thread, lock-free ring buffer, no allocations on the hot path

## Quick Start

```bash
resonance                          # Launch TUI
resonance song.dsl                 # Open a file in TUI
resonance play song.dsl            # Headless playback
resonance play song.dsl --duration 30  # Play for 30 seconds
resonance export song.dsl -o out.wav   # Export to WAV
```

On first run, Resonance offers a genre selection (house, techno, ambient, drum & bass) and generates a starter template.

### Key Bindings

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit |
| `Ctrl+Enter` | Compile & play |
| `Ctrl+P` | Toggle Edit / Perform mode |
| `Ctrl+T` | Cycle visual theme |
| `Ctrl+,` | Open settings panel |
| `Ctrl+D` | Reconnect audio device |
| `Ctrl+;` | Command bar |
| `Tab` | Cycle panel focus |
| `Space` | Play / Stop (Perform mode) |
| `1`–`9` | Jump to section (Perform mode) |
| `F1` | Help overlay |

### Commands

Type in the command bar (`Ctrl+;`):

| Command | Action |
|---------|--------|
| `:plugins` | List installed plugins |
| `:packs` | List installed packs |
| `:export path [bars]` | Export to WAV |
| `:arrangement on\|off\|reset` | Control arrangement playback |
| `:midi_out` | List MIDI output devices |
| `:audio` / `:reconnect` | Reconnect audio device |
| `:settings` | Open settings panel |

## Examples

Seven genre examples are included:

| File | Style | BPM |
|------|-------|-----|
| `examples/monkey_island.dsl` | Game soundtrack | 120 |
| `examples/blue_monday.dsl` | Synth/electronic | 130 |
| `examples/fur_elise.dsl` | Classical | 72 |
| `examples/techno_drive.dsl` | Techno | 138 |
| `examples/jazz_waltz.dsl` | Jazz | 140 |
| `examples/safri_duo.dsl` | Percussion/dance | 137 |
| `examples/blue_man_group.dsl` | Tribal percussion | 140 |

```bash
resonance play examples/blue_man_group.dsl --duration 30
```

## Building

```bash
cargo build              # debug
cargo build --release    # optimized (recommended for audio)
cargo test               # 1180+ tests
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
| **Intent** | Performance and structural intents with diff preview |
| **Taste** | Opt-in learning from editing patterns, proposal weighting |
| **Section/Layer** | Quantized transitions, scene jumping, arrangement playback |
| **Macro Engine** | Named macros mapped to parameters with curve functions |
| **DSL Compiler** | Lexer → Parser → AST → Event compilation (both syntaxes) |
| **Event Engine** | Deterministic scheduler with seedable randomness (960 PPQN) |
| **Audio Engine** | Dedicated thread, lock-free ring buffer, master limiter + FX |

## Plugins

Resonance supports config-based plugins — custom instruments defined by YAML manifests and optional WAV samples. Place plugins in `~/.resonance/plugins/<name>/` with a `plugin.yaml` file. Use `plugin: name` in your DSL to reference them.

See [docs/plugin-guide.md](docs/plugin-guide.md) for details.

## Instrument Packs

Packs bundle kits, plugins, and presets into distributable packages. Place packs in `~/.resonance/packs/<name>/` with a `manifest.yaml`.

See [docs/pack-guide.md](docs/pack-guide.md) for details.

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards, and the PR process.

## License

MIT
