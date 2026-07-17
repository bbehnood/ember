#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'a> {
    Identifier(&'a [u8]),
    Number(i64),

    Binary { left: Box<Expr<'a>>, operator: BinaryOp, right: Box<Expr<'a>> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'a> {
    Let { name: &'a [u8], value: Expr<'a> },

    Expression(Expr<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program<'a> {
    pub statements: Vec<Statement<'a>>,
}
