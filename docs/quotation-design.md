Quotation design and implementation — 6 September 2026. The reader, compiler, VM, compact/pretty printers, Emacs support, and executable-template script migrations described here are implemented. The initial audit and before examples refer to baseline `a0f0e6c`. Macro expansion, native service redesign, and the recorded-command double-evaluation fix remain separate work; their discussion below describes integration contracts or future options.

Add ordinary datum quasiquotation: backtick starts a template, comma inserts one computed value, and comma-at inserts the elements of a computed list. Preserve apostrophe quotation. The result is an ordinary Lyric value, suitable for inspection, storage, transport when serializable, or later evaluation.

```lisp
# Existing: fixed executable form, stored as data
'(open_url "https://example.com")

# Template: capture url now; call open_url when the result is evaluated
`(open_url ,url)

# Template: capture a list as a literal argument to a future call
`(focus_window ',window)

# Template: assemble several executable forms without executing them
`(begin ,@commands)
```

This addresses code templates rather than string interpolation. Lyric already has `(format "Hello {}" name)` and `(str ...)`; double-quoted and triple-quoted strings should retain their current contents and evaluation rules. Backticks and commas inside either string remain text.

The current implementation already supports quotation end to end. [lex.rs](../lyric/src/lex.rs) recognizes apostrophe, [parse.rs](../lyric/src/parse.rs) converts it to `(quote FORM)`, and [compile_quote](../lyric/src/codegen.rs) pushes its operand as a constant. Both value printers abbreviate two-element quote forms in [types.rs](../lyric/src/types.rs). The missing pieces are quasiquote, unquote, and splicing, already named in [TODO.org](../TODO.org).

This fits the existing execution model. Parsing yields a `Form`, a serializable subset of `Val`; lists are vectors, with no cons-cell or dotted-tail representation. Compilation operates on `Val`, resolves symbols during execution, and treats other non-list values as constants. [eval](../lyric/src/fiber.rs) compiles a value and runs it in the current environment. [list](../lyric/src/builtin/list.rs) evaluates all its arguments, which explains the nested quotes in today's templates. [apply](../lyric/src/builtin/list.rs) already calls a function with argument values without evaluating them again.

The useful precedents support a small choice:

| Language | Template / insertion / splicing | Consequence for Lyric |
| --- | --- | --- |
| Scheme | Backtick / comma / comma-at, with explicit `quasiquote`, `unquote`, and `unquote-splicing` forms; nesting changes the active quotation depth. | Use the familiar surface and depth model, restricted to Lyric's existing value types. [R7RS §4.2.8](https://small.r7rs.org/attachment/r7rs.pdf#page=20) |
| Emacs Lisp | Backtick / comma / comma-at construct partly computed list structure. | Familiar to the user's editor workflow; exact internal expansion need not follow Emacs. [Emacs Lisp manual](https://www.gnu.org/software/emacs/manual/html_node/elisp/Backquote.html) |
| Janet | Tilde / comma / comma-semicolon; `;x` means `(splice x)`, also usable in ordinary function calls and literal constructors. Backticks delimit long strings. | Its template ergonomics are good, but its general call-splicing feature is a separate language decision. Lyric already uses triple quotes for raw blocks and has `apply`. [Syntax](https://janet-lang.org/docs/syntax.html#shorthand), [special forms](https://janet-lang.org/docs/specials.html#splice-x) |
| Clojure | Backtick / tilde / tilde-at; syntax quotation qualifies symbols and supports automatic generated names with `#`. | Useful macro conveniences, but they would change the identity of symbols in service data and require decisions about namespaces and binding. Defer them. [Reader reference](https://clojure.org/reference/reader#_syntax_quote_note_the_backquote_character_unquote_and_unquote_splicing) |

Janet is particularly relevant because it separates parsing from execution and demonstrates code templates replacing tuple construction directly. Its macro guide also shows why avoiding duplicate argument evaluation and accidental variable capture remains the macro author's responsibility, with `gensym` as an explicit tool. Lyric should preserve that distinction: quasiquote builds a value; the macro design decides expansion and binding policy. [Janet parser](https://janet-lang.org/docs/syntax.html), [macro guide](https://janet-lang.org/docs/macros.html)

Use only one spelling for each reader abbreviation:

| Source | Exact reader result |
| --- | --- |
| `'x` | `(quote x)` |
| `` `x `` | `(quasiquote x)` |
| `,x` | `(unquote x)` |
| `,@x` | `(unquote-splicing x)` |
| `',x` | `(quote (unquote x))` |
| `,@(get record :commands)` | `(unquote-splicing (get record :commands))` |

These are ordinary lists, not new `Form` variants. The reader recursively attaches each prefix to exactly one following form. It performs no evaluation, symbol resolution, expansion, or contextual unquote validation. Consequently `(read ",x")` returns `(unquote x)` as data; evaluating that result outside a quasiquote is an error. Prefixes and long forms have identical semantics, including when constructed from Rust or list operations.

The lexer reserves comma and backtick as delimiters everywhere outside strings and comments. Comma immediately followed by `@` is one prefix token; `, @name` instead unquotes the symbol `@name`. `@`, `!`, tilde, and semicolon acquire no independent new meaning. Existing `name!` symbols remain available for the separate macro invocation design. Whitespace and comments may separate a prefix from its operand, but cannot separate the two characters of `,@`.

Quasiquote evaluates its active holes in the surrounding lexical environment, once each, in left-to-right, depth-first order. It introduces no new binding scope. The rest of the template is data: lists are assembled, symbols retain their exact names, and literal values retain their values. Inserting a value does not recursively interpret that value as more template syntax or execute it.

With `x = 7` and `xs = '(8 9)`, these are the results, shown as printed values:

| Expression | Result |
| --- | --- |
| `` `x `` | `x` (a symbol) |
| `` `,x `` | `7` |
| `` `(a ,x ,xs ,@xs) `` | `(a 7 (8 9) 8 9)` |
| `` `(a ,nil) `` | `(a nil)` |
| `` `(a ,@'() b) `` | `(a b)` |
| `` `(,@'()) `` | `()` |
| `` `(,@'(f) 1) `` | `(f 1)` |
| `` `"Hello ,x" `` | `"Hello ,x"` |

Splicing accepts a `Val::List`, including an empty list, and inserts exactly one level of elements in order. Reject `nil`, strings, numbers, and other values; `nil` and `()` are distinct in Lyric. Splicing is legal only in an active list-element position, including the first or last element. There is no top-level splice, argument spreading in ordinary calls, dotted-tail support, or implicit flattening. The template may construct `()` as data; evaluating `()` afterward still produces Lyric's existing empty-expression error. Spliced inputs are not mutated.

The distinction between insertion and later execution is essential for the launcher. If `name` is the symbol `focus_window`, then:

```lisp
`(call_interactively ,name)
# Produces: (call_interactively focus_window)
# On later eval: looks up focus_window and passes its function value.

`(call_interactively ',name)
# Produces: (call_interactively 'focus_window)
# On later eval: passes the symbol focus_window, as required.
```

The same rule applies to list arguments. Use `,form` to insert executable syntax, and `',value` to make the future expression receive a literal value. String, numeric, keyword, boolean, and nil arguments are already self-evaluating, so their extra quote is usually unnecessary. Quasiquote does not capture the environment for literal symbols: a generated function name is resolved when the generated code runs. Captured hole values are fixed when the template runs, subject to the existing reference semantics of any runtime objects they contain.

Ordinary quote inside a template is deliberately ordinary list structure. It does **not** block the template traversal. That is what lets `',name` create the quote around the captured name. Conversely, an outer ordinary quote blocks all compilation and template processing in its operand:

```lisp
# Given x = 7
`(quote ,x)              # value: (quote 7)
'(quasiquote (a ,x))     # value: (quasiquote (a (unquote x)))

# Insert literal marker-shaped data without interpreting its contents:
`(tag ,'(unquote missing))
# value: (tag (unquote missing)); missing is not looked up.
```

Nesting should be specified by depth rather than textual substitution. Enter the outer template at depth 1. A nested `(quasiquote T)` is preserved, and its operand is processed at depth + 1. At depth 1, `(unquote E)` evaluates `E` normally and inserts its result. At deeper depths, preserve the unquote marker and process its operand at depth − 1. Apply the same depth rule to `unquote-splicing`, with actual flattening only at depth 1 in a list-element position. A nested marker's operand is a single-template position; an active splice there is invalid. A future stage can reject a preserved splice if it is eventually used in an invalid position.

```lisp
# Given x = 7
`(outer `(inner ,x ,,x))
# value, deliberately in long form:
# (outer (quasiquote (inner (unquote x) (unquote 7))))
```

The first `,x` belongs to the inner template and remains unevaluated. The second comma in `,,x` reaches the outer stage and captures 7. This behavior supports templates that generate templates. An active hole is normal Lyric code: any quasiquote appearing inside that expression starts its own independent template evaluation.

More precisely, the compiler needs two operations: compile one template value at a specified depth, and append one template element to a list under construction. The first recognizes quotation markers before treating a list as ordinary structure. The second recognizes an active `unquote-splicing` element, evaluates its operand once, checks that it is a list, and extends the destination; otherwise it compiles one template value and appends it. Reconstruct deeper markers with their original head symbol and exactly one processed operand. Neither operation walks values returned by active holes.

Errors should be predictable across short and long forms:

| Situation | Proposed diagnostic and phase |
| --- | --- |
| EOF after a prefix | `IncompleteExpression`: expected a form after the named prefix. |
| Closing parenthesis where a prefix needs an operand | Reader error identifying the missing operand. |
| Evaluated `unquote` or `unquote-splicing` outside quasiquote | `InvalidExpression`: marker outside quasiquote. |
| Zero or multiple operands to a quotation marker | `InvalidExpression`: marker expects exactly one argument. |
| Active splice in a single-template position, such as `` `,@xs `` | `InvalidExpression`: splice requires a list-element position. |
| Active splice evaluates to a non-list | Runtime `UnexpectedArguments`: unquote-splicing expects a list; include the received value. |

Arity validation applies to quasiquote/unquote/splice marker lists encountered during template compilation, including inactive nested markers. Bare marker symbols in data remain symbols. Ordinary quote outside templates keeps its current behavior: `'(unquote)` is valid data. Quote inside a template is not a marker for this traversal, so the template may build even a malformed future quote expression; its later evaluation is the point that validates that expression. Fail immediately when an active hole or splice fails, before evaluating subsequent holes. Preserve normal `try` behavior and existing compile timing, including delayed compilation beneath `try`/`eval`; this proposal does not add an eager whole-program validation pass. Source positions would improve diagnostics but need not block the feature: current tokens and forms have no spans.

The following replacements preserve actual launcher construction and evaluation timing. The “after” examples are implemented; “before” examples preserve the baseline for comparison.

At [vrsjmp.ll:235](../scripts/vrsjmp.ll), the search URL is computed while rendering items; opening the browser happens on click:

```lisp
# Before
(make_item "Search Google"
  (list 'open_url (format "http://google.com/search?q={}" query)))
# After
(make_item "Search Google"
  `(open_url ,(format "http://google.com/search?q={}" query)))
```

At [vrsjmp.ll:140](../scripts/vrsjmp.ll), preserve the symbol argument. At [vrsjmp.ll:149](../scripts/vrsjmp.ll), preserve both the name and the one-element list containing an entity:

```lisp
# Before
(def choose (list 'call_interactively (list 'quote name)))
(list 'continue_call (list 'quote name) (list 'quote (list entity)))
# After
(def choose `(call_interactively ',name))
`(continue_call ',name '(,entity))
```

For example, `name = 'focus_window` and `entity = '(:os/window :id 42)` produce `(continue_call 'focus_window '((:os/window :id 42)))`. The entity is not executable, and the singleton wrapper is not removed. The analogous first-match path at line 145 becomes `` `(continue_call ',name '(,(get matches 0))) ``.

At [vrsjmp.ll:214](../scripts/vrsjmp.ll), collect the next argument now, then pass that complete collection as data on click:

```lisp
# Before
(list 'continue_call (list 'quote name) (list 'quote (push values entity)))
# After
`(continue_call ',name ',(push values entity))
```

At [vrsjmp.ll:269](../scripts/vrsjmp.ll), preserve a quoted window and a dynamically chosen executable head:

```lisp
# Before
(list 'begin
  (list 'focus_window (list 'quote window))
  (list (get action 1)))
# After
`(begin
   (focus_window ',window)
   (,(get action 1)))
```

The chosen action symbol and window are captured at item creation. Neither focus nor layout runs then. On click, `begin` focuses that window before looking up and calling the layout command. At [vrsjmp.ll:313](../scripts/vrsjmp.ll), `(list 'open_things_task (list 'quote task))` similarly becomes `` `(open_things_task ',task) ``. The note and background-command paths need the same protection.

Splicing is useful when a variable number of executable forms are already available. For example, `(+ '(begin) commands)` is structurally equivalent to `` `(begin ,@commands) ``. Each element of `commands` is a form, retained as data during construction. In contrast, use `(apply callable values)` for an immediate call with already computed argument values. Splicing values into executable code does not automatically quote them for a later call.

There is a separate, verified behavioral issue at [vrsjmp.ll:457](../scripts/vrsjmp.ll): `(list 'eval (get m :cmds))` constructs `(eval (begin ...))`, and `on_click` then evaluates that whole form. The recorded commands run once, after which their final result is evaluated as code. A result `(:done 1)` therefore raises “Not a function object — :done.” The implemented syntax-only rewrite is `` `(eval ,(get m :cmds)) `` and preserves that behavior. A deliberate fix would supply `(get m :cmds)` directly as the `:on_click` value, because [on_click](../scripts/vrsjmp.ll) already evaluates it. Keep that fix distinct and test it separately. These recorded “macros” are command recordings, not user-defined language macros.

Migration scope is narrow enough to review by category:

| Location | Change to consider |
| --- | --- |
| `scripts/vrsjmp.ll` | Convert dynamic executable forms for query actions, interactive completion, window actions, Things, pages, notes, clipboard, and recording controls. Preserve static quoted forms. |
| `scripts/vrsjmp_demo.ll` | Three active dynamic `open_url` construction sites. |
| `scripts/vrsjmp_interfacegen_demo.ll:21` | Replace the nested background invocation with `` `(run_in_background ',(get item :on_click)) ``; preserve the delayed evaluation in `run_in_background`. |
| `scripts/chat.ll:5` | Implemented: accumulate ordinary CLI argument strings, then return `` `(exec "cogni" ,@args) ``. Alternatively, remove the need for code construction through `apply`; that is a separate API refactor. |
| `scripts/cmd_macro.ll:50` | Keep recording metadata as ordinary lists and the incremental `(push existing_commands cmd)`. The existing `'(begin)` seed is already clear. |
| Other services | Keep message records, entity records, argument collections, search-field lists, and publication payloads as `list` constructors. |

The baseline parsed-source audit of all `scripts/*.ll` found 86 `list` forms in `vrsjmp.ll`, including 44 whose first argument is quoted. Those 44 include nested `(list 'quote ...)` wrappers, so they are not 44 independent actions. The same syntactic indicator found three in `vrsjmp_demo.ll` and two in `vrsjmp_interfacegen_demo.ll`; the latter includes its nested quote wrapper. No symbol or keyword parsed from these scripts contained comma or backtick. Comments and embedded strings were excluded by using Lyric's own parser. This is a repository audit, not a guarantee about `~/vrsjmp_local.ll`, persisted external forms, or other callers.

Do not mechanically replace every `list`. [make_item](../scripts/vrsjmp.ll), the page protocol at line 187, Feedbin messages at line 163, search fields at line 107, and collections of items are ordinary data construction. Quasiquote can express them too, but there is little benefit when nearly every element is computed. Static actions such as `'(download_video_active_tab)` are already concise. The important distinction is the eventual consumer of the value, not whether its first item is a keyword.

Native construction remains a related migration, not an automatic consequence of reader syntax. [service.rs](../libvrs/src/rt/bindings/service.rs) builds `spawn_srv`/`srv` forms in Rust, and [lambda_stub_for_interface](../libvrs/src/rt/bindings/service.rs) formats an AST to text and reparses it. The latter can independently use a direct `Val::List` AST and eliminate that text round trip; a backtick parser does not let Rust source embed Lyric variables. Later, a macro transformer can express the stub body as `` `(call (find_srv ,service) (list ,message ,@parameter_symbols)) ``: the parameter symbols are inserted as code and read their values only when the stub runs. Keep `(list ...)` inside that generated body because it constructs the actual request message at runtime.

Do not convert runtime service discovery into compile-time discovery as part of this work. `def_bind_interface` reads live interface metadata, `spawn_srv` embeds a runtime process ID and a readiness handshake, and `srv` assembles a live receive loop. Quasiquote can support local `Val::Extern` insertions without making them serializable. The macro sibling owns any redesign of these phases. Compiler rewrites for `fn`, `defn`, `let`, and `cond` in [codegen.rs](../lyric/src/codegen.rs) likewise remain intact until the macro design is approved. Rust helpers such as [protocol::quoted](../vrsjmp/src-tauri/src/protocol.rs) remain appropriate for assembling protocol values directly.

Implementation can be small and independent of user-defined macros. Add the three prefix token cases and parser desugarings; add `compile_quasiquote` and explicit errors for stray unquote/splice in the existing special-form dispatch. Compile active holes through the normal expression compiler. For list construction, use an initially empty `Val::List` on the VM stack and two internal instructions: append one value, and extend by a checked list. Each consumes the just-computed value above the destination list and leaves the growing list on the stack. Check splice type immediately, before evaluating the next element. Move values into the destination where possible to avoid repeated copying of the accumulated prefix.

This requires no named temporary variables, new lexical scope, first-class splice value, public `append` helper, or string reparse. It also avoids lowering into calls to the user-rebindable names `list` or `+`. A caller who shadows either name must not change quasiquotation. Constant subtrees can use `PushConst`; more aggressive folding can wait. The builder stack must survive `yield`, native async calls, and nested `eval`, and must unwind through `try` like other expression evaluation. Do not evaluate holes in a synchronous recursive helper outside the VM.

The macro design can use this unchanged. A transformer body such as `` `(if ,test (begin ,@body)) `` produces one form. Its holes run when the transformer runs; the inserted user's `test` and body forms execute only when the expanded result runs. `name!` remains a symbol in data and is recognized as a macro invocation only in an executable head position, according to that design. Generated local names require the macro design's explicit `gensym` facility; datum quasiquote itself has no hygiene guarantee.

The companion [macro proposal](macros-design.md) now specifies a separate expansion environment, `for_syntax` helpers, `macroexpand_1`, repeated-outer `macroexpand`, and printable generated symbols. These choices agree with this proposal. Quasiquote uses whichever environment executes it: ordinary runtime templates retain normal runtime capabilities, while transformer templates inherit the expansion environment's restrictions. Runtime quasiquote may insert a local runtime object; returning that object as public macro syntax still fails the macro proposal's source-value validation. No whole-tree expansion inspection is required for quotation support.

An expansion walker must honor quotation boundaries: skip ordinary quote operands completely; within quasiquote, traverse only executable active-hole expressions, respecting nested depth. Do not expand macro-looking lists in template data or walk the values returned by holes. It must also preserve existing deferred compilation boundaries such as the operand of `try`; recognizing an executable position does not imply expanding it earlier than its normal compilation phase. Explicit expansion inspection of a quoted form is allowed because the caller deliberately hands that value to the expander; it must return readable data without executing the expanded program. Leave quasiquote visible in that output unless the inspection API explicitly asks to show compiler lowering. Active holes must use the same context-aware compiler/expander as ordinary expressions once macros exist. The exact `defmacro`, `gensym`, and expansion API signatures belong to the sibling design.

Printing and editor support must agree with the reader. Both `Form` and `Val` writers may abbreviate exact two-element marker lists, and long forms remain a valid fallback. Never abbreviate malformed-arity lists. Printing `(unquote @name)` requires `, @name` or its long form; emitting `,@name` would change its meaning. Do not print comma as separator whitespace. Round-trip tests must include adjacent prefixes, nested quotation, and marker forms outside an active template, which the reader intentionally accepts as data. Existing unrestricted Rust-created symbol names are not all readable today; reserve the new characters explicitly and avoid claiming arbitrary-symbol round trips until escaped symbol syntax exists. Already-materialized ordinary launcher actions contain no new marker forms and should retain their existing wire representation.

[lyric-mode.el](../emacs/lyric-mode.el) derives from Janet mode but changes backtick from string syntax to punctuation. Quotation support needs backtick and comma treated as expression prefixes for navigation and evaluation bounds. `,@` needs contextual handling so that `@` stays a symbol constituent elsewhere, including the distinction between `,@name` and `, @name`. Preserve raw source in `lyric--last-sexp-source`; do not read and print through Emacs Lisp. Test selection of the entire prefixed expression, not just the following list.

The formatting task can retain ordinary data-list indentation for quoted templates, with active unquote expressions using code indentation. Prefer backtick before executable templates and long forms when explaining nesting. Ensure prefix syntax does not turn punctuation inside regular/raw strings or comments into structure; existing triple-quote normalization and source extraction tests remain relevant. Tilde and semicolon should not be highlighted as newly supported Lyric quotation operators merely because Janet mode knows them. Coordinate these changes with the sibling's `(pretty VALUE [WIDTH])` work rather than maintaining competing writer rules.

Acceptance tests should exercise behavior that could regress:

1. Reader equality for every prefix and long form, chained prefixes, comments between prefix and operand, missing operands, adjacent `,@`, and separated `, @name`. Parsing never evaluates a counter or expands a macro. Check existing regular and triple-quoted string cases.
2. Runtime equality for atoms, nested lists, one-value insertion versus one-level splicing, multiple/empty/head/tail splices, and the documented nesting example. Test that marker-shaped values inserted through a hole are not rescanned.
3. Quote staging: assert exact values for `` `(call_interactively ',name) ``, `` `(continue_call ',name '(,entity)) ``, and `` `(begin (focus_window ',window) (,(get action 1))) ``. Outer quote remains opaque; inner quote does not block template holes.
4. Evaluation timing: a counter records left-to-right hole execution once, ordinary template calls remain inert, and a failed splice prevents later holes. Include holes that yield, await a native function, raise inside `try`, and reference a surrounding lexical binding. Shadow `list` and `+` to prove compiler construction does not depend on those bindings.
5. Runtime shape errors: stray markers, bad marker arity, root splice, and splice values `nil`, string, and integer. Test the same cases through explicit forms built by `list` and through `eval`, not only reader sugar. Verify literal marker data remains readable.
6. Migration equivalence: evaluate each old constructor and proposed constructor under identical fixtures and compare full `Val`/`Form` equality. Include entity lists containing symbols, an empty argument list, quoted strings with escapes, and a dynamically chosen command. Use stub functions with call logs to verify building/ranking items has no command side effects and clicking invokes each intended command once in order.
7. Protocol and editor round trips: format/read representative forms with both compact and pretty writers, including `(unquote @name)`. Exercise the existing launcher query/action protocol tests in [main.rs](../vrsjmp/src-tauri/src/main.rs), preserving quoted payloads and cached context. Add ERT navigation/selection/indentation tests for each prefix, nested templates, strings, and comments.
8. Once macros exist, assert that an inert template containing `when!` is not expanded, an invocation in an active hole is expanded in its proper phase, and explicit inspection produces the agreed expansion without running user body forms. Add generated-name and nested-template fixtures in that task.

The implementation followed three stages: reader/compiler/VM semantics with focused tests and language documentation; coordinated printer/Emacs support; launcher and demo rewrites with equality and timing tests. Handle the recorded-command double-evaluation fix as a separate behavioral change. Finally consider native stub construction and macro-backed wrappers when the macro design's phase and binding decisions are settled. Run `cargo test -p lyric`, relevant `libvrs` and launcher tests, and ERT for their respective stages; run the workspace suite at integration, without launching services as a substitute for controlled tests.

The approved semantic boundary is: ordinary datum quasiquote with strict list-only splicing, using backtick/comma/comma-at, while ordinary call spreading and Clojure-style symbol rewriting remain separate proposals. The implementation uses the two internal list-builder instructions described above. Existing quote behavior and action evaluation timing are requirements, not options.

Verification: `cargo test --workspace --offline --quiet` passes (two existing ignored tests). Lyric includes 18 quotation integration tests for reading, nesting, evaluation order, quote staging, errors, yield/await, compact/pretty/JSON round trips, constructor equivalence, and actual script behavior. `cargo test -p vrsjmp --offline --quiet` and `cargo test -p vrs --offline --quiet` pass. The CLI/PTY harness `JANET_MODE_DIR=/path/to/janet-mode python3 vrsctl/tests/terminal.py` runs an isolated daemon and also exercises Emacs evaluation and replacement. All seven harness tests and all 18 ERT tests pass, including quotation through the real client/runtime wire path. The double-evaluation playback issue remains deliberately unchanged.
