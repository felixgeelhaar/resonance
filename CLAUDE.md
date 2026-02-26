# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Resonance is a terminal-native live coding music instrument built in Rust. It enables users to create, structure, and perform full songs using code, macros, and semantic intent. It is not a DAW, genre generator, or black-box AI composer — it is a deterministic music engine with a semantic steering system layered over code.

## Core Principles

- **Event stream is the source of truth** — the grid is a projection, code is the authority
- **AI never silently commits structural changes** — intent produces diffs, user must accept/reject
- **Macros are explicit and mapped** — no hidden state, no magic
- **Live performance must be deterministic and safe** — quantized transitions, crash-resistant
- **Learning is transparent and opt-in** — taste system is inspectable, editable, resettable

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (optimized audio)
cargo run                      # Run in debug mode
cargo test                     # Run all tests
cargo test --lib               # Unit tests only
cargo test --test <name>       # Single integration test
cargo test <test_fn_name>      # Single test by name
cargo clippy                   # Lint
cargo fmt                      # Format
cargo bench                    # Run benchmarks (audio latency, scheduler)
```

## Architecture

Eight layers, each with clear boundaries:

```
┌─────────────────────────────────┐
│         TUI Interface           │  ratatui — panels: editor, tracks, grid, macros, intent console
├─────────────────────────────────┤
│       Intent Processor          │  performance intents (quantized) vs structural intents (diff-based)
├─────────────────────────────────┤
│         Taste Engine            │  opt-in learning, ~/.resonance/taste.yaml, proposal weighting
├─────────────────────────────────┤
│    Section/Layer Controller     │  quantized transitions, scene jumping, layer enable/disable
├─────────────────────────────────┤
│      Macro Mapping Engine       │  explicit macros → target params with curves (linear/log/exp/smoothstep)
├─────────────────────────────────┤
│        DSL Compiler             │  declarative + functional syntax → AST → Track Graph → Event Stream
├─────────────────────────────────┤
│      Event Stream Engine        │  deterministic scheduler, seedable randomness, transform pipeline
├─────────────────────────────────┤
│        Audio Engine             │  dedicated thread, lock-free queue, double-buffer swap, master limiter
└─────────────────────────────────┘
```

### Compilation Pipeline

```
DSL Source → Lexer → Parser → AST → Track Graph → Event Stream → Audio Scheduler
```

Both DSL styles (declarative and functional chain) compile to identical AST.

### Audio Thread Model

The audio engine runs on a **dedicated thread** with strict constraints:
- **No allocations** on the audio thread
- **Lock-free ring buffer** for communication between UI thread and audio thread
- **Double-buffer swap** for safe state updates
- **Pre-buffered scheduled events** — events are compiled ahead of time
- **Master limiter** as safety net

Use `cpal` for cross-platform audio output. All synths and FX run on the audio thread.

### Built-in Instruments (v1)

- Sample-based drum kit
- Bass synth (mono)
- Poly synth (pad)
- Pluck/keys synth
- Noise/riser generator

### Built-in FX (v1)

Reverb, delay, drive, sidechain compression, limiter.

### Event Data Model

```rust
struct Event {
    time: Beat,        // time in beats/bars
    duration: Beat,
    track_id: TrackId,
    note_or_sample: NoteOrSample,
    velocity: f32,
    params: Params,
}
```

### Macro System

```rust
struct Macro { name: String, value: f64 }
struct Mapping { macro_ref: MacroRef, target_param: ParamId, range: (f64, f64), curve: CurveType }
```

Curves: `Linear`, `Log`, `Exp`, `Smoothstep`. Section-aware overrides, stackable layers.

### Intent System — Two Modes

**Performance Intent** (quantized, immediate): macro deltas, mapping tweaks, layer toggles — applied on beat boundaries.

**Structural Intent** (diff-based, requires confirmation): code diffs at AST level, track-safe and mapping-safe updates — user must accept/reject before apply.

Pipeline: `Intent Input → Mode Detection → Taste Bias → Diff Generator → Preview → Apply`

### Section & Layer Model

```rust
struct Section { name: String, length_in_bars: u32, mapping_overrides: Vec<MappingOverride> }
struct Layer { name: String, mapping_additions: Vec<Mapping>, enabled: bool }
```

Quantized transition manager ensures all changes land on bar boundaries.

### Taste Engine

Stored at `~/.resonance/taste.yaml`. Tracks preferred ranges, accepted diffs, undo patterns, macro movement patterns. Influences proposal weighting and default mapping ranges. **Never mutates active code silently.**

## Key Design Constraints

- **No GC spikes in audio thread** — Rust's ownership model enforces this, but be vigilant with `Arc`, `Box`, or any heap allocation in audio path
- **Deterministic playback** — same seed + same code = same output, always
- **Seedable randomness** — all randomness must accept a seed for reproducibility
- **Quantized updates** — state changes snap to beat/bar boundaries during performance
- **Cross-platform** — Linux and macOS (v1), use platform-agnostic abstractions

## Crate Dependencies (expected)

- `cpal` — cross-platform audio I/O
- `ratatui` + `crossterm` — terminal UI
- `serde` + `serde_yaml` — taste config serialization
- `ringbuf` or `crossbeam` — lock-free communication
- `hound` — WAV sample loading

## Testing Strategy

- **Audio engine**: benchmark latency, test buffer underrun handling, verify lock-free invariants
- **Event engine**: determinism tests (same seed → same output), scheduling accuracy
- **DSL compiler**: parser round-trip tests, both syntaxes produce identical AST
- **Macro engine**: curve accuracy, mapping resolution, section override precedence
- **Intent system**: diff correctness, quantization boundary tests
- **Integration**: end-to-end from DSL source to scheduled events (no audio output needed)

## Roadmap

### Phase 0 — Foundation (current)
Repo setup, audio engine skeleton, event scheduling core, basic drum playback.

### Phase 1 — Core Instrument (MVP)
DSL parser, pattern engine, basic instruments, grid visualization, macros + mapping, sections, quantized safe updates, first-run feel prompt, basic intent (macro-only). **Goal**: performable loop-based songs live.

### Phase 2 — Song-Level Power
Structural intent (diff mode), section overrides, layers, advanced mapping, taste system (session + persistent), diff preview UI. **Goal**: structured full-song performance.

### Phase 3 — Performance Refinement
MIDI controller support, OSC support, advanced visualization, stability hardening, performance macros UX polish. **Goal**: stage-ready instrument.

### Phase 4 — Ecosystem
Plugin API, visualization themes, instrument packs, community contributions. **Goal**: open ecosystem growth.

## Out of Scope (v1)

Plugin ecosystem, custom curve functions, web version, collaboration features, advanced synthesis engines, full ML training pipelines.
