#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr<'a> {
    Identifier(&'a [u8]),
    Number(i64),

    Binary { left: Box<Expr<'a>>, op: BinaryOp, right: Box<Expr<'a>> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub enum Statement<'a> {
    Let { name: &'a [u8], value: Expr<'a> },

    Expression(Expr<'a>),
}

pub struct Program<'a> {
    pub statements: Vec<Statement<'a>>,
}
