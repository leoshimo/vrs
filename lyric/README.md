# Lyric

Lisp-dialect focused on providing uniform code and data interface for
[vrs](https://github.com/leoshimo/vrs/).

## Goals

- Simple
- Dynamic scripting environment
- Embedding within Rust programs
- Ease of native bindings
- Serializable data structures and code

## Quotation and code templates

`'FORM` is shorthand for `(quote FORM)`: return the form as data. Backtick
introduces a partly computed template. Comma inserts one evaluated value;
comma-at inserts the elements of an evaluated list:

```lyric
(def url "https://example.com")
(def action `(open_url ,url))
# action is (open_url "https://example.com"); open_url has not run.

(def commands '((notify "first") (notify "second")))
`(begin ,@commands)
# => (begin (notify "first") (notify "second"))
```

The exact reader forms are `(quasiquote FORM)`, `(unquote EXPR)`, and
`(unquote-splicing EXPR)`. Reading never executes them. Quasiquote evaluates
active holes once, in left-to-right order, in its surrounding lexical scope.
Inserted values are not rescanned or executed. Splices require lists: `()` is
empty, while `nil` and strings are errors. A splice must occupy a list-element
position; ordinary calls still use `apply` for a list of argument values.

When generating a call that should receive a symbol or list **as data**, keep
a quote around the inserted value:

```lyric
(def window '(:os/window :id 42))
`(focus_window ',window)
# => (focus_window '(:os/window :id 42))
```

Quote inside a template is ordinary structure, so `',window` captures the
value inside a future quote. An outer quote makes its entire operand literal.
Nested backticks increase quotation depth; commas remove one level:

```lyric
(def x 7)
`(outer `(inner ,x ,,x))
# => (outer (quasiquote (inner (unquote x) (unquote 7))))
```

Unquote outside a template, misplaced splices, and marker forms with the wrong
number of arguments are errors. Regular and triple-quoted strings do not
interpolate. Comma and backtick are reserved outside strings/comments; `!` and
`@` remain symbol characters. `,@name` splices `name`; `, @name` inserts the
value of `@name` without splicing. Both compact and pretty printers preserve
that distinction. Templates use ordinary `Form` lists and retain the existing
serialization shape. They do not qualify symbols or provide macro hygiene.

Keep `list` for ordinary records, messages, and collections of computed values.
Use templates where showing the structure of generated code helps.

## Readable values

`(pretty VALUE [WIDTH])` returns a formatted string (default width 80; explicit
widths must be positive integers). Small lists stay on one line. Larger lists
wrap with indentation, keeping small keyword/value pairs together. Oversized
nested values start below their keyword:

```lyric
(pretty '((:name :echo :node "alpha" :interface ((ping x) (pong y)))
          (:name :clock :interface ((now)))) 40)
```

The string contains:

```lyric
((:name :echo
  :node "alpha"
  :interface ((ping x) (pong y)))
 (:name :clock :interface ((now))))
```

Serializable values round-trip through `(read (pretty VALUE))`. Strings are
escaped, quote prefixes are preserved, and opaque runtime values keep their
existing display notation. Width is measured in display columns and is a target:
atoms are not split, scalar keyword/value pairs stay together, and deeply nested
values may exceed it. The formatter never truncates a value.

Rust embedders can use `Form::to_pretty_string(width)` and
`Val::to_pretty_string(width)`. Width zero is treated as one column in this Rust
API. `Display` and `(display ...)` stay compact, abbreviating quotation forms;
wire serialization retains the same atoms-and-lists representation.
`Form::RawString` is still verbatim display text, not a readable string literal.
