//! DSL compiler — declarative + functional syntax → AST → Track Graph → Event Stream.

pub mod ast;
pub mod compile;
pub mod error;
pub mod lexer;
pub mod note;
pub mod parser;
pub mod token;

pub use ast::*;
pub use compile::CompiledSong;
pub use error::CompileError;

use compile::compile_program;
use lexer::Lexer;
use parser::Parser;

/// The DSL compiler.
///
/// Parses source text through lexer → parser → AST, then compiles to events.
pub struct Compiler;

impl Compiler {
    /// Parse DSL source into a Program AST.
    pub fn parse(source: &str) -> Result<Program, CompileError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    /// Parse and compile DSL source into a CompiledSong.
    pub fn compile(source: &str) -> Result<CompiledSong, CompileError> {
        let program = Self::parse(source)?;
        compile_program(&program)
    }
}
