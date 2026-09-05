# ember

Ember is a small, tree-walking interpreter for a toy scripting language,
written in Rust. It exists to be a compact, readable example of a
lexer/parser/type-checker/interpreter pipeline, not a production language.

## Building and running

```sh
cargo build
cargo run -- path/to/script.ember
```

There's no required file extension; `.ember` is just a convention used by
this README.

## Running the tests

```sh
cargo test
```

## The pipeline

Source code passes through four stages, in order:

1. **Lexer** (`src/lexer.rs`) turns raw source bytes into a stream of
   tokens.
2. **Parser** (`src/parser.rs`) turns those tokens into an AST
   (`src/ast.rs`).
3. **Sema** (`src/sema.rs`) statically checks the AST: every variable must
   be declared before use, and operand types must line up with what each
   operator expects.
4. **Interpreter** (`src/interpreter.rs`) walks the type-checked AST and
   executes it.

Each stage has its own error type; `src/error.rs` unifies them into one
`Error` so callers of the library only need to handle a single type.
Lexical and syntax errors report the `line:col` of the problem, e.g.:

```
Error: 2:9: unexpected character '@'
```

## The language

Ember has three types: `Number` (a signed 64-bit integer), `String`, and
`Boolean`. There is no `null`/`nil`, and there are no functions - programs
are a flat sequence of statements executed top to bottom.

```
let x = 1;
let message = "hello";

while x < 5 {
    if x == 3 {
        print("three!");
    } else {
        print(x);
    }

    x = x + 1;
}
```

### Statements

- `let name = expr;` declares a new variable in the current scope.
- `name = expr;` reassigns an existing variable (found in the current or
  an enclosing scope).
- `print(expr);` evaluates `expr` and prints it.
- `{ ... }` opens a new lexical scope.
- `if condition { ... } else { ... }` - the `else` is optional.
- `while condition { ... }`.

### Expressions

- Literals: integers (`42`), strings (`"hi"`), and `true`/`false`.
- Arithmetic: `+ - * /` (operate on and produce `Number`).
- Comparisons: `< <= > >=` (require `Number`, produce `Boolean`).
- Equality: `== !=` (either type, as long as both sides match; produce
  `Boolean`).
- Logical: `&& ||` (require `Boolean`, produce `Boolean`).

## License

Dual-licensed under either the [MIT license](LICENSE_MIT) or the
[Apache License, Version 2.0](LICENSE_APACHE), at your option.
