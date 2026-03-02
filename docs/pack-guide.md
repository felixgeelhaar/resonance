# Instrument Pack Guide

Instrument packs bundle kits, plugins, and presets into a single distributable package.

## Overview

A pack is a directory under `~/.resonance/packs/<pack_name>/` containing a `manifest.yaml` and organized subdirectories for samples, plugins, and presets.

## Directory Structure

```
~/.resonance/packs/
  my_pack/
    manifest.yaml           # Pack metadata (required)
    samples/                # Kit sample directories
      808/
        kick.wav
        snare.wav
        hat.wav
    plugins/                # Plugin instruments
      warm_pad/
        plugin.yaml
    presets/                # DSL preset files
      groove.dsl
      chill.dsl
```

## Manifest Format

The `manifest.yaml` describes the pack contents:

```yaml
name: "electronic_essentials"
version: "1.0.0"
author: "Producer Name"
description: "Essential electronic music samples and instruments"
genre: "electronic"

kits:
  808:
    - kick.wav
    - snare.wav
    - hat.wav
    - clap.wav
  vinyl:
    - kick_vinyl.wav
    - snare_vinyl.wav

plugins:
  - warm_pad
  - acid_bass

presets:
  - groove.dsl
  - chill.dsl
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique pack name |
| `version` | Yes | Semantic version |
| `author` | No | Creator name |
| `description` | No | Short description |
| `genre` | No | Target genre |
| `kits` | No | Map of kit name to sample file list |
| `plugins` | No | List of plugin directory names |
| `presets` | No | List of preset `.dsl` filenames |

## Creating a Kit Pack

To create a pack with just drum kits:

1. Create the directory structure:
   ```
   my_kit_pack/
     manifest.yaml
     samples/
       my_kit/
         kick.wav
         snare.wav
         hat.wav
   ```

2. Write the manifest:
   ```yaml
   name: "my_kit_pack"
   version: "1.0.0"
   description: "Custom drum samples"
   kits:
     my_kit:
       - kick.wav
       - snare.wav
       - hat.wav
   ```

3. Copy to `~/.resonance/packs/my_kit_pack/`

4. Use in DSL:
   ```
   track drums {
     kit: my_kit
     section main [2 bars] {
       kick: [X . . . X . . .]
     }
   }
   ```

Kit names from packs are resolved automatically when referenced in the DSL.

## Creating a Full Pack

A full pack includes kits, plugins, and presets:

1. Organize your content:
   ```
   full_pack/
     manifest.yaml
     samples/
       deep_kit/
         kick.wav
         snare.wav
     plugins/
       deep_bass/
         plugin.yaml
     presets/
       deep_groove.dsl
   ```

2. Each plugin directory must contain a valid `plugin.yaml` (see the [Plugin Guide](plugin-guide.md)).

3. Each preset file should be a valid `.dsl` file, optionally with YAML frontmatter:
   ```
   ---
   name: "Deep Groove"
   description: "Deep house groove template"
   genre: "house"
   ---
   tempo 122
   track drums {
     kit: deep_kit
     ...
   }
   ```

## Installing Packs

Copy the pack directory to `~/.resonance/packs/`:

```bash
cp -r my_pack ~/.resonance/packs/
```

Resonance scans this directory on startup. Pack kits, plugins, and presets become available immediately.

## Listing Installed Packs

Use the `:packs` command in the TUI command bar to list all installed packs.

## How Packs Integrate

- **Kits**: Pack sample directories are checked during kit resolution. Use the kit name directly in your DSL (`kit: 808`).
- **Plugins**: Pack plugins are scanned alongside `~/.resonance/plugins/`. Use `plugin: name` in DSL.
- **Presets**: Pack presets appear in `:presets` listing and can be loaded with `:preset name`.

## Example Pack

See `assets/packs/example/manifest.yaml` in the Resonance repository for a reference manifest.
