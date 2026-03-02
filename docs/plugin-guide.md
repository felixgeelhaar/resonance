# Plugin Guide

Resonance supports config-based plugins — custom instruments defined by a YAML manifest and optional WAV samples. No dynamic libraries or compilation required.

## Overview

A plugin is a directory containing a `plugin.yaml` manifest file and optionally WAV sample files. Plugins can define either a **sampler** (sample-based) or **synth** (oscillator-based) instrument.

## Directory Structure

```
~/.resonance/plugins/
  warm_pad/
    plugin.yaml       # Instrument definition (required)
  808_drums/
    plugin.yaml
    kick.wav           # Sample files (for sampler type)
    snare.wav
    hat.wav
```

## Plugin Manifest

The `plugin.yaml` file defines the instrument:

```yaml
name: "warm_pad"
version: "1.0.0"
author: "Your Name"
description: "A warm saw pad with slow attack"

instrument:
  kind: synth              # "synth" or "sampler"
  waveform: "saw"          # sine, saw, square, triangle
  envelope:
    attack: 0.5
    decay: 0.3
    sustain: 0.6
    release: 1.0
  filter_cutoff: 0.8       # 0.0 to 1.0
```

### Required Fields

| Field | Description |
|-------|-------------|
| `name` | Unique plugin name (used in DSL) |
| `version` | Semantic version string |

### Optional Fields

| Field | Description |
|-------|-------------|
| `author` | Creator name |
| `description` | Short description |
| `instrument` | Instrument definition (see below) |

### Instrument Definition

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `synth` or `sampler` | Instrument type |
| `waveform` | string | Oscillator waveform (synth only): `sine`, `saw`, `square`, `triangle` |
| `envelope` | object | ADSR envelope with `attack`, `decay`, `sustain`, `release` (seconds) |
| `filter_cutoff` | float | Simple filter cutoff 0.0-1.0 (default 1.0 = no filtering) |
| `samples` | map | Trigger name to WAV file path mapping (sampler only) |

## Creating a Sampler Plugin

A sampler maps trigger names to WAV files:

```yaml
name: "808_kit"
version: "1.0.0"
description: "Classic 808 drum machine"

instrument:
  kind: sampler
  samples:
    kick: "kick.wav"
    snare: "snare.wav"
    hat: "hat.wav"
    clap: "clap.wav"
```

Place WAV files in the same directory as `plugin.yaml`. Paths are relative to the plugin directory. WAV files are converted to mono at load time if stereo.

## Creating a Synth Plugin

A synth uses built-in oscillators with an ADSR envelope:

```yaml
name: "plucky_lead"
version: "1.0.0"
description: "Short plucky lead sound"

instrument:
  kind: synth
  waveform: "square"
  envelope:
    attack: 0.01
    decay: 0.15
    sustain: 0.3
    release: 0.1
  filter_cutoff: 0.7
```

Available waveforms: `sine`, `saw`, `square`, `triangle`. If omitted, defaults to `sine`.

## Using Plugins in DSL

Reference plugins in your DSL code with the `plugin` keyword:

**Declarative syntax:**
```
track lead {
  plugin: warm_pad
  section main [4 bars] {
    note: [C4 E4 G4 C5]
  }
}
```

**Functional syntax:**
```
lead = plugin("warm_pad") |> pattern([C4 E4 G4 C5])
```

If a plugin is not found, Resonance falls back to the default bass synth.

## Listing Installed Plugins

Use the `:plugins` command in the TUI command bar to list all discovered plugins.

## WAV File Requirements

- Format: WAV (`.wav`)
- Sample format: 16/24/32-bit integer or 32-bit float
- Channels: mono or stereo (stereo is mixed to mono)
- Sample rate: any (no resampling — best results at 44100 Hz)
