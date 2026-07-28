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

pub fn run(source: &[u8]) -> Result<Option<Value>, Error> {
    let tokens = Lexer::new(source).tokenize()?;
    let program = Parser::new(&tokens).parse()?;

    Sema::new().check(&program)?;

    Ok(Interpreter::new().run(&program)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_program() {
        assert_eq!(run(b"let x = 5; x + 1;"), Ok(Some(Value::Number(6))));
    }

    #[test]
    fn empty_program() {
        assert_eq!(run(b""), Ok(None));
    }

    #[test]
    fn lex_error() {
        let err = run(b"1 + @;").unwrap_err();
        assert_eq!(err.to_string(), "unexpected character '@'");
    }

    #[test]
    fn parse_error() {
        let err = run(b"let x = ;").unwrap_err();
        assert_eq!(err.to_string(), "expected an expression");
    }

    #[test]
    fn sema_error() {
        let err = run(b"x;").unwrap_err();
        assert_eq!(err.to_string(), "undefined variable 'x'");
    }

    #[test]
    fn runtime_error() {
        let err = run(b"1 / 0;").unwrap_err();
        assert_eq!(err.to_string(), "attempt to divide by zero");
    }
}
