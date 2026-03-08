//! Abstract Syntax Tree for the Resonance DSL.
//!
//! Both declarative and functional chain syntaxes parse into these types.

/// A complete DSL program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub tempo: f64,
    pub tracks: Vec<TrackDef>,
    pub macros: Vec<MacroDef>,
    pub mappings: Vec<MappingDef>,
    pub layers: Vec<LayerDef>,
    /// Number of cycles to expand (default None = 1).
    pub cycles: Option<u32>,
    /// Arrangement plan for auto-advancing sections.
    pub arrangement: Option<ArrangementDef>,
}

/// A track definition with instrument and sections.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackDef {
    pub name: String,
    pub instrument: InstrumentRef,
    pub sections: Vec<SectionDef>,
    /// Optional MIDI output routing.
    pub midi_out: Option<MidiOutDef>,
}

/// MIDI output routing definition.
#[derive(Debug, Clone, PartialEq)]
pub struct MidiOutDef {
    pub device: String,
    pub channel: u8,
}

/// Reference to a built-in instrument.
#[derive(Debug, Clone, PartialEq)]
pub enum InstrumentRef {
    Kit(String),
    Bass,
    Poly,
    Pluck,
    Noise,
    Plugin(String),
    Fm,
    Wavetable(String),
}

/// A section within a track.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionDef {
    pub name: String,
    pub length_bars: u32,
    pub patterns: Vec<PatternDef>,
    pub overrides: Vec<MappingOverrideDef>,
}

/// A per-section mapping override.
#[derive(Debug, Clone, PartialEq)]
pub struct MappingOverrideDef {
    pub macro_name: String,
    pub target_param: String,
    pub range: (f64, f64),
    pub curve: CurveKind,
}

/// A pattern transform applied in functional chain syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum Transform {
    /// `.fast(2)` — repeat pattern N times (double speed).
    Fast(f64),
    /// `.slow(2)` — keep first 1/N of pattern (half speed).
    Slow(f64),
    /// `.rev()` — reverse pattern order.
    Rev,
    /// `.rotate(2)` — rotate steps right by N positions.
    Rotate(i32),
    /// `.degrade(0.3)` — randomly remove steps at given probability.
    Degrade(f64),
    /// `.every(4, rev)` — apply inner transform every N cycles.
    Every(u32, Box<Transform>),
    /// `.sometimes(0.5, fast(2))` — apply inner transform with probability.
    Sometimes(f64, Box<Transform>),
    /// `.chop(4)` — subdivide each step into N equal parts.
    Chop(u32),
    /// `.stutter(3)` — repeat each non-rest step N times in place.
    Stutter(u32),
    /// `.add(7)` — transpose notes by N semitones (post-event).
    Add(i32),
    /// `.gain(0.5)` — scale velocity by factor (post-event).
    Gain(f64),
    /// `.legato(2.0)` — scale duration by factor (post-event).
    Legato(f64),
}

/// A pattern for a specific target (drum hit or note line).
#[derive(Debug, Clone, PartialEq)]
pub struct PatternDef {
    pub target: String,
    pub steps: Vec<Step>,
    pub velocities: Option<Vec<f64>>,
    pub transforms: Vec<Transform>,
}

/// A single step in a pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Hit,
    Rest,
    Accent(f64),
    Note(String),
    /// Random hit with probability (0.0–1.0). `?` = 0.5, `?0.7` = 0.7.
    Random(f64),
    /// Alternate per cycle: `<X x .>` picks one per cycle.
    Alternate(Vec<Step>),
    /// Subdivide: `{X . X}` fits 3 steps into 1 slot.
    Subdivided(Vec<Step>),
    /// Ratchet: `X^3` plays 3 rapid hits in 1 step.
    Ratchet(Box<Step>, u32),
    /// Stacked: `K+H` plays multiple targets simultaneously.
    Stacked(Vec<String>),
}

/// A macro definition.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub name: String,
    pub default_value: f64,
}

/// A mapping definition.
#[derive(Debug, Clone, PartialEq)]
pub struct MappingDef {
    pub macro_name: String,
    pub target_param: String,
    pub range: (f64, f64),
    pub curve: CurveKind,
}

/// A layer definition with named mapping additions.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerDef {
    pub name: String,
    pub mappings: Vec<MappingDef>,
    pub enabled_by_default: bool,
}

/// Curve type for mappings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveKind {
    Linear,
    Log,
    Exp,
    Smoothstep,
}

/// An arrangement plan for auto-advancing sections.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangementDef {
    pub entries: Vec<ArrangementEntry>,
}

/// A single entry in an arrangement.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangementEntry {
    pub section_name: String,
    pub repeats: u32,
}
