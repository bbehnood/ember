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

pub fn run(source: &[u8]) -> Result<(), Error> {
    let tokens = Lexer::new(source).tokenize()?;
    let program = Parser::new(&tokens).parse()?;

    Sema::new().check(&program)?;
    Interpreter::new().run(&program)?;

    Ok(())
}
