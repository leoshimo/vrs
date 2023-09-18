# Lemma

Lisp-dialect focused on providing uniform code and data interface for
[vrs](https://github.com/leoshimo/vrs/).

## Goals

- Simple
- Dynamic scripting environment
- Embedding within Rust programs
- Ease of native bindings
- Serializable data structures and code

## REPL

There is a binary target that exposes a simple REPL:

```sh
# in vrs root folder
$ cargo run --bin lemma
```

## Tokens vs Form vs Value

At a high level, lemma works in three stages, each accepting and returning
specific type:

1. `lex(expr) -> [Token]`
2. `parse(tokens) -> Form`
3. `eval(form) -> Value`
