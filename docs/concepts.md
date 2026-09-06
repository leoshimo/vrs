# Core Concepts

## Forms

Lyric represents code and data as values: symbols, keywords, numbers, strings,
and lists. A list can hold data or describe an expression to evaluate.

`pretty` formats a value as a string, using a default width of 90 columns:

```lyric
(pretty (ls_srv))
(pretty (ls_srv) 60)
```

The terminal and Emacs format results automatically. Piped `vrsctl` output stays
compact; use `--format pretty` when piping to `less`.

## Quotation and Templates

Quote (`'`) keeps an expression as data. `eval` executes it:

```lyric
'(+ 40 2)        # => (+ 40 2)
(eval '(+ 40 2)) # => 42
```

Backtick builds a template. Comma inserts a value; comma-at inserts the elements
of a list:

```lyric
(def amount 40)
`(+ ,amount 2) # => (+ 40 2)

(def numbers '(10 20 12))
`(+ ,@numbers) # => (+ 10 20 12)
```

When generating a call that takes a list as data, quote the inserted list:

```lyric
(def window '(:os/window :id 42))
(focus_window window)    # Passes the value of window.

`(focus_window ,window)  # => (focus_window (:os/window :id 42))
`(focus_window ',window) # => (focus_window '(:os/window :id 42))
```

Evaluating the first template would try to call `:os/window`. The second passes
the window record to `focus_window`. Strings and numbers need no extra quote.

## Macros

A macro receives unevaluated expressions and returns code. Define it with
`defmacro` and call it with `!`:

```lyric
(defmacro unless (condition & body)
  `(when! (not? ,condition) ,@body))

(unless! false (+ 20 22))                  # => 42
(unless! true (error "should not run"))    # => nil
```

`& body` collects the remaining expressions. Unlike a function, a macro can
choose which arguments get evaluated.

Expansion happens when execution reaches the call. Lyric runs the macro,
compiles its result, and executes that code in the caller's scope. Calls inside
functions expand on each invocation; redefining a macro affects the next call.

### Inspecting an Expansion

```lyric
(macroexpand_1 '(unless! false (+ 20 22)))
# => (when! (not? false) (+ 20 22))

(macroexpand '(unless! false (+ 20 22)))
# => (if (not? false) (begin (+ 20 22)) nil)
```

`macroexpand_1` expands once. `macroexpand` repeats while the result is another
outer macro call; it does not walk nested expressions. Quote the call to inspect
it. Both functions run the macro body, including any side effects, but leave the
generated code unevaluated.

### Scope and Helpers

Macros and their helpers use the scope where they were defined. They can call
ordinary functions, inspect definitions, change variables, and perform I/O.
`(for_syntax ...)` behaves like `(begin ...)`; define helpers with `defn`.

Use `eval_caller` to evaluate source in the macro call's scope:

```lyric
(defmacro remember (expression)
  (def value (eval_caller expression))
  `(quote ,value))

(defn example (x)
  (remember! (+ x 1)))

(example 41) # => 42
```

Here `eval_caller` evaluates `(+ x 1)` where `x` is 41. Ordinary `eval` uses its
own lexical scope. `eval_caller` is available during expansion, including in
helpers called by a macro. Returned code cannot embed live functions or process
handles.

Macro definitions belong to a process. Spawned processes inherit the definitions
and can redefine them independently.

### Generated Names

`gensym` makes a fresh symbol for a variable introduced by a macro:

```lyric
(defmacro or_else (expression fallback)
  (def temp (gensym "value"))
  `(let ((,temp ,expression))
     (if ,temp ,temp ,fallback)))

(def value 42)
(or_else! false value) # => 42
```

The temporary evaluates `expression` once. A fixed name such as `value` would
hide the caller's `value` in the fallback. `gensym` avoids that collision, using
a name such as `value__p7Fq2mR8tK4vW9xB`. Lyric does not rename generated variables
automatically.

### In Emacs

In `lyric-mode`, put point on or just after a call's closing parenthesis and press
`C-c C-m` to show its expansion. `C-u C-c C-m` uses `macroexpand`.

Each editor command opens a fresh connection. Include local definitions in the
same evaluation:

```lyric
(begin
  (defn echo (x) x)
  (macroexpand_1 '(srv! :test :interface '(echo))))
```

At the final closing parenthesis, `C-u C-c C-e` replaces this block with its
formatted expansion. To evaluate a selected region, use `C-c C-r`; add `C-u` to
replace it. `C-c C-e` always evaluates one expression, regardless of selection.

`C-g` cancels a waiting evaluation. It does not undo effects already performed.

## Function Metadata

`interactive` declares argument types for the palette. `meta` returns these
annotations, argument names, and documentation. The annotations do not enforce
types at runtime.

```lyric
(defn focus_window (window)
  "Focus Window"
  (interactive :os/window)
  (exec "yabai" "-m" "window" (str (get window :id)) "--focus"))

(get (meta focus_window) :args)
# => ((:type :os/window :name window))
```

`call_interactively` uses this metadata to prompt for arguments; see
[Interactive Calls](#interactive-calls).

## Entities and Completions

An entity is a list with a type tag followed by properties:

```lyric
'(:os/window :id 123 :app "Safari" :title "VRS")
```

Completion providers return entities of a given type. The palette calls them
with no arguments and presents the results for selection.

```lyric
(set_entity_completions :os/window 'get_windows)
(get_entity_completions :os/window) # => (get_windows)
```

`get_entity_completions` returns provider names without calling them.
`set_entity_completions` accepts one name or a list. Local providers override
those imported by `bind_srv`: `nil` restores imported defaults, while `'()`
disables completions for that type. Defaults from multiple services combine.

## Processes and Messages

Each VRS process has an environment, a mailbox, and a process ID that includes
its node. `spawn` starts a process; `send` and `recv` exchange values:

```lyric
(def parent (self))
(spawn (fn () (send parent '(:greeting "hello"))))
(match (recv)
  ((:greeting message) message)) # => "hello"
```

`call` sends a request and waits for a reply. `call_timeout` sets the deadline
in seconds for calls made by the current process:

```lyric
(call_timeout 10)
(call (find_srv :echo) '(:echo "hello"))
```

## Services

A service is a process registered under a name. `srv!` registers the current
process and serves its exported functions. `spawn_srv!` starts a child service
and waits for registration:

```lyric
(defn echo (x) x)
(spawn_srv! :echo :interface '(echo))

(find_srv :echo)
(info_srv :echo :interface) # => ((:echo x))
```

`srv!` reads the handlers' parameters and generates a receive loop containing:

```lyric
(match message
  ((:echo x) (echo x))
  (_ '(:err "Unrecognized message")))
```

`message` abbreviates a generated name. The patterns are fixed at startup.
Calls look up the handlers again, so their implementations can be replaced
while the service runs.

The interface is evaluated once during expansion. It can use variables already
defined at the call:

```lyric
(defn start (exports)
  (defn echo (x) x)
  (spawn_srv! :test :interface exports))

(start '(echo))
```

`bind_srv` creates functions that send requests to the service:

```lyric
(bind_srv :echo)
(echo "hello") # => "hello"
```

Bindings preserve metadata and import completion providers exported by the
service. Rebinding refreshes those defaults and preserves local overrides.

## Pub/Sub

Topics broadcast updates to subscribers' mailboxes. Subscriptions receive
future publications; messages are not retained for later subscribers.

```lyric
(subscribe :clock)
(publish :clock '(:tick 1))
(recv) # => (:topic_updated :clock (:tick 1))
```

## External Commands

`exec` runs a host program and returns its exit status, stdout, and stderr.
`decode` parses text into values:

```lyric
(def result (exec "printf" "one\ntwo\n"))
(decode :lines (get result :stdout)) # => ("one" "two")
```

## Scripts and Initialization

`run` evaluates a file in a fresh process, with an implicit `begin` around its
forms:

```lyric
(run "./scripts/counter.ll")
```

`vrsd --init` runs a script before accepting clients. Services started by the
script continue running after it finishes:

```sh
cargo run --bin vrsd -- --init ./scripts/init.ll
```

## Nodes and Peering

VRS daemons exchange service registrations and route messages between nodes.
Programs can discover services by name across configured nodes:

```lyric
(node_name) # => "laptop"
(configure :nodes '("ssh://home-server"))
(ls_srv)
```

## Hypermedia (WIP)

[vrsjmp.ll](../scripts/vrsjmp.ll) describes pages and actions with Lyric values.
The GUI displays them and sends selected commands back to the service for
evaluation.

### Pages and Navigation

`push_page` names a function that supplies page items and a prompt:

```lyric
(defn read_later_page ()
  (+ (push_page 'read_later_items "Search saved pages…")
     '(:title "Read Later" :debounce_ms 200)))
```

The GUI calls `get_items` with the callback name, fixed arguments, and query.
The service calls the callback with those arguments followed by the query.
The GUI maintains a page stack; Escape returns to the previous page.

### Rows and Actions

Rows contain display text, a command, and optional secondary actions:

```lyric
'(:title "VRS"
  :subtitle "github.com"
  :on_click (open_url "https://github.com/leoshimo/vrs")
  :actions ((:title "Copy URL"
             :on_click (set_clipboard "https://github.com/leoshimo/vrs"))))
```

The service's `on_click` evaluates the selected command and returns a
`:push_page` instruction or `:close`. Secondary actions use the same record
format and appear in the Cmd-K menu.

### Interactive Calls

`call_interactively` reads a function's metadata and prompts for its arguments:

```lyric
(call_interactively 'focus_window)
```

For a `:os/window` argument, it calls the window completion providers and shows
their results. Selecting a window supplies the argument and calls the function.
Functions with more arguments get another selection page for each one.

`entity_actions` works from a selected entity: it finds interactive functions
whose first argument has the same type tag, supplies that entity, and prompts
for any remaining arguments.

## Introspection

Inspect definitions, documentation, processes, and services from `vrsctl` or
Emacs:

```lyric
(ls_env)
(help recv)
(ps)
(ls_srv)
(info_srv :echo :interface_doc)
```
