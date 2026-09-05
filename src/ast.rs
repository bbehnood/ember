//! The abstract syntax tree (AST) produced by [`crate::parser::Parser`] and
//! consumed by [`crate::sema::Sema`] and [`crate::interpreter::Interpreter`].
//!
//! The AST borrows byte slices (identifiers) directly from the original
//! source buffer rather than owning `String`s, which is why every type here
//! carries a lifetime `'a` tied to the source.

/// An expression: something that evaluates to a [`crate::interpreter::Value`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr<'a> {
    /// A reference to a variable by name, e.g. `x`.
    Identifier(&'a [u8]),

    /// An integer literal, e.g. `42`.
    Number(i64),

    /// A string literal, e.g. `"hello"`
    String(&'a [u8]),

    /// A boolean literal, e.g. `true` or `false`.
    Boolean(bool),

    /// A binary operation, e.g. `left + right`.
    Binary { left: Box<Expr<'a>>, operator: BinaryOp, right: Box<Expr<'a>> },
}

/// The operator used in a [`Expr::Binary`] expression.
///
/// Grouped by category: arithmetic (`Add`..`Div`), comparison
/// (`Equal`..`GreaterEqual`), and logical (`And`, `Or`). See
/// `BinaryOp::check` (in `sema`) for the type rules each operator
/// enforces, and `Interpreter::eval` (in `interpreter`) for how each is
/// evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,

    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    And,
    Or,
}

/// A statement: a unit of execution that does not itself produce a value
/// (unlike [`Expr`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement<'a> {
    /// Declares a new variable in the current scope, e.g. `let x = 1;`.
    ///
    /// It is an error to declare a variable that already exists in the same
    /// scope (see [`crate::sema::SemaError::DuplicateVariable`]).
    Let { name: &'a [u8], value: Expr<'a> },

    /// Reassigns an existing variable, e.g. `x = 2;`.
    ///
    /// The variable must already be defined in the current or an enclosing
    /// scope (see [`crate::sema::SemaError::UndefinedVariable`]).
    Assign { name: &'a [u8], value: Expr<'a> },

    /// Evaluates an expression and prints its value, e.g. `print(x);`.
    Print(Expr<'a>),

    /// A `{ ... }` block. Introduces a new lexical scope: variables declared
    /// inside are only visible for the lifetime of the block.
    Block(Vec<Statement<'a>>),

    /// An `if condition { ... } else { ... }` statement. `else_clause` is
    /// `None` when there is no `else` branch. `condition` must evaluate to
    /// a boolean.
    If {
        condition: Expr<'a>,
        statement: Box<Statement<'a>>,
        else_clause: Option<Box<Statement<'a>>>,
    },

    /// A `while condition { ... }` loop. `condition` is re-evaluated before
    /// each iteration and must evaluate to a boolean.
    While { condition: Expr<'a>, statement: Box<Statement<'a>> },

    /// An expression evaluated purely for its side effects, e.g. a bare
    /// `1 + 2;`. The resulting value is discarded.
    Expression(Expr<'a>),
}

/// A complete Ember program: an ordered sequence of top-level statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program<'a> {
    pub statements: Vec<Statement<'a>>,
}
