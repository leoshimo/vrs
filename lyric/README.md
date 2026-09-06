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

`(pretty VALUE [WIDTH])` returns a formatted string (default width 90; explicit
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

## User-defined macros

```lisp
(defmacro unless (test & body)
  `(if ,test nil (begin ,@body)))
(unless! false 42)
(macroexpand_1 '(unless! false 42))
# => (if false nil (begin 42))
```

`name!` marks invocation explicitly; macro arguments remain unevaluated forms.
The optional final `& rest` parameter collects remaining source arguments.
A transformer returns one source form, using `begin` for multiple expressions.
`macroexpand_1` runs one outer transformer; `macroexpand` repeats only at the
outermost position. Neither executes its returned program or walks quoted data.

Define phase helpers in `(for_syntax (defn helper (...) ...))`. Their completed
block is frozen, and each macro captures the helper revision present when it is
defined. Runtime variables and host capabilities are unavailable in this phase.
Local calculation is allowed; mutation of captured phase bindings is rejected.
Use `(gensym "hint")` for fresh printable local names. This is a datum macro
system, with no implicit qualification or automatic hygiene for free identifiers.

Top-level definitions and evaluations run in order, including inside a top-level
`begin`. A later compile error can follow earlier effects. Macros in lambda
bodies expand when the function is compiled; redefining the macro requires
reevaluating that function to change its expansion. `try` and explicit `eval`
remain deferred compilation boundaries and use the current process namespace.
No macros are transported with quoted source data; the evaluating process must
have the needed definitions.

Expansion allows up to 1,000,000 phase instructions, 10,000 macro invocations,
1,000,000 accumulated output nodes, 256 outer expansions, and 64 nested
transformer executions per budget. Intermediate phase values also have size
bounds. Source nesting is limited to 256 levels, including in runtime `read`.
Generated top-level sequences share their preparation budget; a later
independent evaluation starts a fresh budget and can recover after an error.
