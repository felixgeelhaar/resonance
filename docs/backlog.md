
## Project Scaffolding & Repo Setup

Initialize Rust project with Cargo.toml, module structure (audio/, event/, dsl/, macro_engine/, section/, intent/, taste/, tui/), .gitignore, CI configuration. Establish the foundational crate dependencies: cpal, ratatui, crossterm, serde, ringbuf/crossbeam, hound.

---

## Audio Engine Skeleton

Set up the dedicated audio thread with cpal for cross-platform audio output. Implement lock-free ring buffer (ringbuf/crossbeam) for communication between UI and audio threads. Implement double-buffer swap mechanism for safe state updates. Add master limiter as safety net. No allocations on the audio thread. Pre-buffered event scheduling infrastructure.

---

## Event Scheduling Core

Implement the Event data model (time in beats/bars, duration, track_id, note/sample, velocity, params). Build deterministic scheduler that processes events in time order. Implement beat/bar timing system with configurable BPM. Seedable randomness for reproducible playback. Transform pipeline for event manipulation.

---

## Basic Drum Playback

Implement WAV sample loading (hound crate). Build sample-based drum kit instrument that can be triggered by events from the scheduler. Wire up the full path: hardcoded events → scheduler → audio engine → drum sample playback through speakers. This validates the entire audio pipeline end-to-end.

---

## DSL Parser

Build the DSL compiler: lexer (tokenizer), parser, and AST representation. Support two syntax styles — declarative and functional chain — that both compile to identical AST. The compilation pipeline: DSL Source → Lexer → Parser → AST. Include error reporting with line/column information for user-friendly feedback in the TUI editor.

---

## Pattern Engine

Compile AST into Track Graph, then into Event Stream. Pattern compilation pipeline: AST → Track Graph → Event Stream → Audio Scheduler. Support pattern repetition, variation, and composition. Deterministic output — same seed + same code = same events.

---

## Basic Instruments

Implement built-in synthesizers for the audio engine: bass synth (mono), poly synth (pad), pluck/keys synth, noise/riser generator. Each instrument receives events from the scheduler and produces audio samples. Basic FX chain: reverb, delay, drive, sidechain compression, limiter.

---

## Grid Visualization & TUI

Build the TUI interface with ratatui + crossterm. Panels: code editor, track list, grid visualization (projection of event stream), macro meters, performance controls. Hot reload of DSL blocks. The grid is a projection — event stream remains the source of truth.

---

## Macros & Mapping System

Implement the macro engine: explicit macro declaration (name + value), mapping to target parameters with range and curve type. Curves v1: linear, log, exp, smoothstep. Curve override support. Macro meters in TUI for visual feedback. Macros are explicit — no hidden state.

---

## Sections & Quantized Transitions

Implement Section model (name, length_in_bars, mapping_overrides). Quantized transition manager ensures all section changes land on bar boundaries. Scene jumping between sections. Safe state commit on transitions. DSL support for section declaration.

---

## Quantized Safe Updates

All state changes during performance snap to beat/bar boundaries. Hot reload of DSL blocks with safe apply on quantized boundaries. Crash-resistant update mechanism — failed updates don't interrupt playback. Double-buffer swap ensures audio thread never sees partial state.

---

## First-Run Feel Prompt

On first launch, prompt user: "What should it feel like?" Generate an editable DSL skeleton based on the response. Display the full DSL — no hidden state. The skeleton is a starting point, not a black box. User can edit everything immediately.

---

## Basic Intent System (Macro-Only)

Implement performance intent mode: macro deltas, mapping tweaks, layer toggles. Quantized apply on beat boundaries. Intent console panel in TUI. This is the macro-only subset — structural intent (diff mode) comes in Phase 2. Pipeline: Intent Input → Mode Detection → Apply (quantized).

---

## Structural Intent (Diff Mode)

Implement structural intent mode: code diffs at AST level, track-safe and mapping-safe updates. User must accept/reject before apply. Safe apply boundary ensures no silent structural changes. Pipeline extension: Intent Input → Mode Detection → Taste Bias → Diff Generator → Preview → Accept/Reject → Apply.

---

## Section Overrides

Section-aware mapping overrides that activate when entering a specific section. Override precedence: layer additions > section overrides > base mappings. Overrides are explicit in the DSL and visible in the TUI.

---

## Layers

Implement Layer model (name, mapping_additions, enabled). Layer enable/disable during performance via intent system. Stackable layers with mapping additions. Layer toggling is quantized to beat boundaries. Layers are declared in DSL and controllable from TUI and intent console.

---

## Advanced Mapping Behavior

Stackable layer mappings with proper resolution order. Curve override support per section and per layer. Section-aware overrides that compose with layer additions. Complex mapping chains with multiple macros affecting the same target through different curves.

---

## Taste System

Opt-in learning system stored at ~/.resonance/taste.yaml. Session learning (within session) and persistent learning (across sessions). Tracks: preferred ranges, accepted diffs, undo patterns, macro movement patterns. Influences intent proposal weighting and default mapping ranges. Inspectable, editable, resettable. Never auto-applies structural changes.

---

## Diff Preview UI

Intent console panel in TUI for previewing structural diffs before accept/reject. Visual diff of AST-level changes showing what will be modified. Clear accept/reject controls. Preview must show the full scope of changes — no hidden mutations.

---

## MIDI Controller Support

Accept MIDI input from external controllers. Map MIDI CC messages to macros. Map MIDI notes to triggers. MIDI learn mode for quick mapping. Cross-platform MIDI via midir crate.

---

## OSC Support

Open Sound Control protocol support for networked control. Send and receive OSC messages for integration with other music software and hardware. Map OSC addresses to macros and intent inputs.

---

## Advanced Visualization

Enhanced TUI visualizations: waveform display, spectrum analyzer, more expressive grid rendering, macro movement trails, section timeline view. Performance-oriented visual feedback that doesn't compromise audio latency.

---

## Stability Hardening

Crash-resistance during live performance. Graceful error recovery without audio dropout. Stress testing under sustained load. Buffer underrun handling. Watchdog for audio thread health. Comprehensive error boundaries between layers.

---

## Performance Macros UX Polish

Refined macro control UX for live performance: smooth transitions, visual feedback improvements, ergonomic keyboard shortcuts, macro grouping, quick-access performance controls. Optimized for stage use under pressure.

---

## Plugin API

Extensible plugin system for custom instruments, effects, and DSL extensions. Stable API for third-party contributions. Plugin discovery and loading mechanism. Sandboxed plugin execution to protect audio thread safety.

---

## Visualization Themes

Themeable TUI with customizable color schemes, layout configurations, and visual styles. Built-in theme presets. User-defined themes via configuration files. Theme hot-switching during performance.

---

## Instrument Packs

Distributable instrument and sample packs. Pack format specification for community sharing. Built-in pack manager for installing, updating, and removing packs. Curated starter packs for common genres.

---

## Community Contributions

Contribution guidelines, plugin registry, shared macro presets, community DSL snippets. Infrastructure for sharing and discovering user-created content. Review and quality assurance process for community submissions.

---
