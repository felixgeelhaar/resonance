//! Token types for the Resonance DSL lexer.

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

/// The kind of token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Tempo,
    Track,
    Section,
    Macro,
    Map,
    Override,
    Layer,
    Kit,
    Bass,
    Poly,
    Pluck,
    Noise,
    Vel,
    Bars,

    // Literals
    Ident(String),
    Number(f64),
    Integer(u64),
    StepPattern(Vec<StepToken>),
    NotePattern(Vec<NoteToken>),

    // Delimiters
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Colon,
    Comma,
    Dot,
    Pipe,   // |>
    Arrow,  // ->
    DotDot, // ..
    Eq,     // =

    // Special
    Newline,
    Eof,
}

/// A step in a pattern grid.
#[derive(Debug, Clone, PartialEq)]
pub enum StepToken {
    Hit,    // X or x
    Rest,   // .
    Accent, // X (uppercase) — high velocity
    Ghost,  // x (lowercase) — low velocity
}

/// A note reference in a pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum NoteToken {
    Note(String), // e.g. "C2", "Eb4", "F#3"
    Rest,         // .
}
