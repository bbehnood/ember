//! Ember is a small, tree-walking interpreter for a toy scripting language.
//!
//! The pipeline for running a program is, in order:
//!
//! 1. [`lexer::Lexer`] turns raw source bytes into a stream of [`lexer::Token`]s.
//! 2. [`parser::Parser`] turns those tokens into an [`ast::Program`] (an AST).
//! 3. [`sema::Sema`] performs a static type-checking pass over the AST,
//!    catching things like undefined variables and type mismatches before
//!    anything runs.
//! 4. [`interpreter::Interpreter`] walks the AST and actually executes it.
//!
//! Each stage has its own error type, and [`error::Error`] unifies them so
//! callers of [`run`] only need to handle a single error type.

pub mod ast;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod sema;

pub use error::Error;
pub use interpreter::{Interpreter, Value};
pub use lexer::Lexer;
pub use parser::Parser;
pub use sema::Sema;

/// Runs Ember source code end-to-end: lex, parse, type-check, then execute.
///
/// This is the main entry point for embedding Ember in another program (the
/// `ember` binary itself is just a thin wrapper around this function). Each
/// stage is run in sequence and the first error encountered - whether it's a
/// lexical, syntax, type, or runtime error - short-circuits the pipeline and
/// is returned via [`Error`].
pub fn run(source: &[u8]) -> Result<(), Error> {
    let tokens_with_positions = Lexer::new(source).tokenize_with_positions()?;

    // The parser wants tokens and positions as two parallel slices rather
    // than one slice of pairs, so it can index into each independently
    // (see `Parser::current_pos`).
    let (tokens, positions): (Vec<_>, Vec<_>) =
        tokens_with_positions.into_iter().unzip();

    let program = Parser::new(&tokens, &positions).parse()?;

    // Type-check the whole program before running any of it, so that runtime
    // execution can rely on invariants (e.g. variables are defined, operand
    // types match) already having been verified.
    Sema::new().check(&program)?;
    Interpreter::new().run(&program)?;

    Ok(())
}
