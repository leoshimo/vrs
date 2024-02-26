# Lyric

Lisp-dialect focused on providing uniform code and data interface for
[vrs](https://github.com/leoshimo/vrs/).

## Goals

- Simple
- Dynamic scripting environment
- Embedding within Rust programs
- Ease of native bindings
- Serializable data structures and code

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
API. `Display`, `(display ...)`, and wire serialization remain unchanged.
`Form::RawString` is still verbatim display text, not a readable string literal.
