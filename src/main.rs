use std::{error::Error, fs};

use ember::{Interpreter, Lexer, Parser};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: ember <file>");
        std::process::exit(1);
    });

    let source = fs::read_to_string(path)?;

    let tokens = Lexer::new(&source).tokenize().unwrap();
    let program = Parser::new(&tokens).parse_program().unwrap();

    let result = Interpreter::new().run(&program).unwrap();

    println!("{result:?}");

    Ok(())
}
