# Contributing to Resonance

Thank you for your interest in contributing to Resonance! This document explains how to get started, our coding standards, and the pull request process.

## Getting Started

```bash
git clone https://github.com/YOUR_USERNAME/resonance.git
cd resonance
cargo build
cargo test
cargo run
```

## Development Setup

- **Rust 1.75+** — install via [rustup](https://rustup.rs/)
- **Audio device** — `cpal` requires a working audio output (speakers or headphones)
- **Editor** — any editor with Rust support (VS Code + rust-analyzer, Neovim, etc.)

## Architecture Overview

Resonance is built in eight layers, each with clear boundaries:

```
TUI Interface           — ratatui terminal panels (editor, tracks, grid, macros, intent, status)
Intent Processor        — performance intents (quantized) vs structural intents (diff-based)
Taste Engine            — opt-in learning, proposal weighting, ~/.resonance/taste.yaml
Section/Layer Controller— quantized transitions, scene jumping, layer enable/disable
Macro Mapping Engine    — explicit macros → target params with curves
DSL Compiler            — lexer → parser → AST → compiled song → event stream
Event Stream Engine     — deterministic scheduler, seedable randomness, transform pipeline
Audio Engine            — dedicated thread, lock-free ring buffer, master limiter
```

Key directories:
- `src/audio/` — cpal-based audio engine, effects
- `src/dsl/` — lexer, parser, AST, compiler, diff
- `src/instrument/` — Instrument trait, DrumKit, BassSynth, PolySynth, PluckSynth, NoiseGen
- `src/tui/` — terminal UI, editor, grid, theme system
- `src/plugin/` — config-based plugin API
- `src/content/` — presets, tutorials, packs

## Code Style

- Run `cargo fmt` before every commit
- Run `cargo clippy -- -D warnings` — all warnings must be resolved
- Follow existing patterns in the codebase
- Keep functions focused and under 40 lines where practical

## Testing

Run the full test suite:

```bash
cargo test              # all tests (900+)
cargo test --lib        # unit tests only
cargo test <test_name>  # single test by name
```

Guidelines:
- Write tests for all new functionality
- Use descriptive test names: `test_parser_recognizes_plugin_keyword`
- Determinism: all randomness must be seeded via `ChaCha8Rng`
- Use `tempfile` for tests that need filesystem access

## Audio Thread Rules

The audio engine runs on a dedicated thread with strict constraints:

- **No heap allocations** on the audio thread
- **No locks** (mutex, rwlock) on the audio thread
- **No panics** — use safe fallbacks
- **Lock-free communication** only (ring buffer, atomic)

If your change touches `src/audio/` or instrument rendering, verify these invariants.

## Pull Request Process

1. **Fork** the repository and create a feature branch:
   - `feature/your-feature-name` for new features
   - `fix/issue-description` for bug fixes
   - `docs/what-you-documented` for documentation

2. **Write tests** for your changes

3. **Verify** everything passes:
   ```bash
   cargo build && cargo test && cargo clippy -- -D warnings && cargo fmt --check
   ```

4. **Open a PR** with a clear description of what changed and why

5. **Respond to review feedback** — maintainers may request changes

## Commit Messages

Follow conventional commits:

```
feat: add reverb dry/wet control
fix: prevent panic on empty pattern
docs: update plugin guide with sampler example
test: add integration test for section transitions
refactor: extract envelope into shared module
```

## Areas for Contribution

- **Plugins & Instrument Packs** — create new instruments and sample packs
- **DSL features** — new pattern types, effects, modulation sources
- **TUI improvements** — visualization, accessibility, keybindings
- **Documentation** — tutorials, examples, guides
- **Bug fixes** — check the issue tracker for open bugs
- **Performance** — profiling and optimization of the audio path

## Questions?

Open an issue for questions or feature discussions. We're happy to help newcomers get oriented in the codebase.
