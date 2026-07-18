pub mod ast;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod sema;

pub use error::Error;
pub use interpreter::{Interpreter, RuntimeError, Value};
pub use lexer::Lexer;
pub use parser::Parser;
pub use sema::Sema;

pub fn run(source: &[u8]) -> Result<Option<Value>, Error> {
    let tokens = Lexer::new(source).tokenize()?;
    let program = Parser::new(&tokens).parse_program()?;

    Sema::new().check_program(&program)?;

    Ok(Interpreter::new().run(&program)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_a_full_program() {
        assert_eq!(run(b"let x = 5; x + 1;"), Ok(Some(Value::Number(6))));
    }

    #[test]
    fn empty_program_returns_none() {
        assert_eq!(run(b""), Ok(None));
    }

    #[test]
    fn surfaces_lex_errors() {
        let err = run(b"1 + @;").unwrap_err();
        assert_eq!(err.to_string(), "unexpected character '@'");
    }

    #[test]
    fn surfaces_parse_errors() {
        let err = run(b"let x = ;").unwrap_err();
        assert_eq!(err.to_string(), "expected an expression");
    }

    #[test]
    fn surfaces_sema_errors() {
        let err = run(b"x;").unwrap_err();
        assert_eq!(err.to_string(), "undefined variable 'x'");
    }

    #[test]
    fn surfaces_runtime_errors() {
        let err = run(b"1 / 0;").unwrap_err();
        assert_eq!(err.to_string(), "attempt to divide by zero");
    }
}
