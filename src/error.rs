use crate::{
    interpreter::RuntimeError, lexer::LexError, parser::ParseError,
    sema::SemaError,
};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Lex(#[from] LexError),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Sema(#[from] SemaError),

    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}
