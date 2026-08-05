//! The top-level error type returned by [`crate::run`].

use thiserror::Error;

use crate::{
    interpreter::RuntimeError, lexer::LexError, parser::ParseError,
    sema::SemaError,
};

/// Unifies the error types produced by each stage of the pipeline
/// ([`LexError`], [`ParseError`], [`SemaError`], [`RuntimeError`]) so that
/// [`crate::run`] can return a single error type via `?`.
///
/// Each variant is `#[error(transparent)]`, meaning its `Display`
/// implementation just delegates to the wrapped error's own message.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// A lexical error, e.g. an unexpected character.
    #[error(transparent)]
    Lex(#[from] LexError),

    /// A syntax error, e.g. a missing semicolon.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// A type error caught during static analysis, e.g. adding a boolean
    /// to a number.
    #[error(transparent)]
    Sema(#[from] SemaError),

    /// An error raised while executing an already type-checked program,
    /// e.g. division by zero.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}
