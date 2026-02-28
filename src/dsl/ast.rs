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
}

/// A track definition with instrument and sections.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackDef {
    pub name: String,
    pub instrument: InstrumentRef,
    pub sections: Vec<SectionDef>,
}

/// Reference to a built-in instrument.
#[derive(Debug, Clone, PartialEq)]
pub enum InstrumentRef {
    Kit(String),
    Bass,
    Poly,
    Pluck,
    Noise,
}

/// A section within a track.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionDef {
    pub name: String,
    pub length_bars: u32,
    pub patterns: Vec<PatternDef>,
}

/// A pattern for a specific target (drum hit or note line).
#[derive(Debug, Clone, PartialEq)]
pub struct PatternDef {
    pub target: String,
    pub steps: Vec<Step>,
    pub velocities: Option<Vec<f64>>,
}

/// A single step in a pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Hit,
    Rest,
    Accent(f64),
    Note(String),
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

/// Curve type for mappings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveKind {
    Linear,
    Log,
    Exp,
    Smoothstep,
}
