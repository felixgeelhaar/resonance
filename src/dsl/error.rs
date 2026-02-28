//! Error types for the DSL compiler.

use std::fmt;

/// An error that occurred during DSL compilation.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub line: usize,
    pub col: usize,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    LexError,
    ParseError,
    CompileError,
}

impl CompileError {
    pub fn lex(message: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            message: message.into(),
            line,
            col,
            kind: ErrorKind::LexError,
        }
    }

    pub fn parse(message: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            message: message.into(),
            line,
            col,
            kind: ErrorKind::ParseError,
        }
    }

    pub fn compile(message: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            message: message.into(),
            line,
            col,
            kind: ErrorKind::CompileError,
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}:{}] {:?}: {}",
            self.line, self.col, self.kind, self.message
        )
    }
}

impl std::error::Error for CompileError {}
