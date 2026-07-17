use ember::{Lexer, Parser};

fn main() {
    let source = "let x = 1 + 2;";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(&tokens);
    let program = parser.parse_program().unwrap();

    println!("{:#?}", program);
}
