# Core Concepts

## Lyric Forms

Lyric uses the same representation for code and structured data. A form can be
constructed, stored, sent to another process, or evaluated later.

```lyric
(def command '(+ 40 2))
(eval command) # => 42
```

Large results are displayed with indentation in the terminal and Emacs. Use
`(pretty VALUE)` to get that display as a string; the default target width is 90
columns, or pass a width such as `(pretty (ls_srv) 60)`.

Compact output has no added line breaks. `vrsctl` uses it by default when piped,
so scripts can keep treating each result as one line. Use `--format pretty`
when piping to a reader such as `less`. Ordinary data can be printed and read
back: `(read (pretty '(1 2 3)))` returns `(1 2 3)`. Printed functions and process
handles describe runtime objects; reading those descriptions cannot recreate
the original objects.

## Quotation and Code Templates

Quote (`'`) keeps an expression as data. `eval` executes it later:

```lyric
'(+ 40 2)        # => (+ 40 2)
(eval '(+ 40 2)) # => 42
```

Backtick makes a template. Comma inserts one computed value; comma-at inserts
the elements of a computed list:

```lyric
(def amount 40)
(def command `(+ ,amount 2))
command        # => (+ 40 2)
(eval command) # => 42

(def numbers '(10 20 12))
`(+ ,@numbers) # => (+ 10 20 12)
```

This is how vrsjmp builds commands to store in an item's `:on_click` field.
When a command should receive an entity as data, the generated source needs a
quote around that entity:

```lyric
(def window '(:os/window :id 42))

# In a normal call, the variable's value is passed directly:
(focus_window window)

# This template inserts the list as another expression to execute:
`(focus_window ,window)
# => (focus_window (:os/window :id 42))

# Keep the inserted list as literal data in the later call:
`(focus_window ',window)
# => (focus_window '(:os/window :id 42))
```

The first generated form would try to call `:os/window` when evaluated. The
second passes the window record. A window entity is an ordinary list, not a
special object literal. An already evaluated function argument is not evaluated
again; this distinction matters because a template constructs source for a
future evaluation. Strings and numbers are already literal expressions, so
`(open_url ,url)` inside a backtick template needs no extra quote around a URL
string.

## Macros

A macro receives source forms and generates code that becomes part of the
calling program. Define one with `defmacro` and invoke it with `!`:

```lyric
(defmacro unless (condition & body)
  `(when! (not? ,condition) ,@body))

(unless! false (+ 20 22)) # => 42
(unless! true (error "should not run")) # => nil
```

Here `& body` collects the remaining expressions. An ordinary function receives
already evaluated arguments; a macro receives their source. This lets a macro
arrange for some expressions to be skipped, as in the second call above.

Inspect the generated code with `macroexpand_1` or `macroexpand`:

```lyric
(macroexpand_1 '(unless! false (+ 20 22)))
# => (when! (not? false) (+ 20 22))

(macroexpand '(unless! false (+ 20 22)))
# => (if (not? false) (begin (+ 20 22)) nil)
```

`macroexpand_1` performs one outer expansion. `macroexpand` keeps expanding while
the result's outer expression is another macro call; it does not recursively
expand every nested expression. Both return source data without running it.
The quote is necessary because these inspection functions evaluate their
arguments normally.

For example, inspect the service macro with:

```lyric
(macroexpand_1 '(srv! :test :interface '()))
```

Without the outer quote, `srv!` runs first and enters its service loop. In
Emacs, put point on or just after the closing parenthesis of
`(srv! :test :interface '())` and press `C-c C-m`; the command adds the quote.
`C-u C-c C-m` uses `macroexpand`. `C-g` cancels a waiting evaluation and
terminates its client, leaving the source intact. Cancellation does not undo
effects already performed. For custom macros, evaluate their definitions and
an explicit expansion call together in a region: each editor command uses a
fresh connection.

Macros generate code in a separate environment. `for_syntax` defines helpers
for that environment, rather than ordinary runtime functions:

```lyric
(for_syntax
  (defn add_one_form (expression) `(+ ,expression 1)))

(defmacro increment (expression) (add_one_form expression))
(increment! 41) # => 42
```

`add_one_form` runs while generating the `(+ 41 1)` form. It cannot read the
program's runtime variables or call running services; the code it generates
can use them later. Macros capture the helpers present when defined. If you
change a helper, reevaluate the macro definition too.

`gensym` creates a fresh symbol for a variable introduced by generated code:

```lyric
(defmacro or_else (expression fallback)
  (def temp (gensym "value"))
  `(let ((,temp ,expression))
     (if ,temp ,temp ,fallback)))

(def value 42)
(or_else! false value) # => 42
```

The generated temporary stores `expression` so it runs only once. If the macro
had hardcoded its temporary's name as `value`, it would hide the caller's
`value` in the fallback expression. `gensym` avoids that collision; Lyric does
not automatically protect all generated names this way.

Macro definitions belong to a process. Functions retain the expansions they
were compiled with, so reevaluate a function after changing a macro it uses.
VRS's `srv!` and `spawn_srv!` use macros to generate service control flow;
`bind_srv` is an ordinary runtime function that discovers the live interface.

## Function Metadata

Functions can declare the entity types their arguments accept with a leading
`interactive` declaration, after an optional docstring. `meta` exposes this
annotation together with the argument names and documentation. These types guide
interaction; they do not enforce type checking when the function is called.

```lyric
(defn focus_window (window)
  "Focus Window"
  (interactive :os/window)
  (exec "yabai" "-m" "window" (str (get window :id)) "--focus"))

(get (meta focus_window) :args)
# => ((:type :os/window :name window))
```

One concrete use is `vrsjmp.ll`'s `call_interactively`: it reads this metadata to
offer argument-selection pages. That helper is described under
[Hypermedia (WIP)](#hypermedia-wip), rather than being a runtime primitive itself.

## Entities and Completions

An entity is an ordinary list whose first item identifies its type, followed by
properties. This is a convention for exchanging values, not a separate object
store or a type hierarchy.

```lyric
'(:os/window :id 123 :app "Safari" :title "VRS")
```

An environment associates each entity type with provider function names.
`get_entity_completions` retrieves those names without invoking them. The current
interactive palette calls providers with no arguments; each returns a list of
entities, which the palette can filter and present.

```lyric
(set_entity_completions :os/window 'get_windows)
(get_entity_completions :os/window) # => (get_windows)
```

Providers can be imported as service defaults. Defaults for the same type combine
across bound services; a local `set_entity_completions` replaces them for that
type. It accepts one provider symbol or a list of symbols. Passing `nil` removes
the local override and restores imported defaults; an empty list disables
completions for that type locally.

## Processes

VRS programs run in lightweight, isolated processes. Each process has its own
environment and mailbox and is identified by a node-qualified process ID.

```lyric
(def parent (self))
(spawn (lambda () (send parent :hello)))
(recv) # => :hello
```

## Message Passing

Processes communicate by sending ordinary Lyric values to one another. A
receiver can use pattern matching to interpret those values.

```lyric
(send (self) '(:greeting "hello"))
(match (recv)
  ((:greeting message) message)) # => "hello"
```

## Calls and Deadlines

`call` provides request-response communication on top of message passing. Calls
have a configurable deadline so an unresponsive process does not block its
caller indefinitely.

```lyric
(call_timeout 10)
(call (find_srv :echo) '(:echo "hello"))
```

## Services

A service is a process registered under a stable name. It runs a message loop
and can publish selected Lyric functions as its interface.

```lyric
(defn echo (message) message)
(spawn_srv! :echo :interface '(echo))

(find_srv :echo)
(info_srv :echo :interface) # => ((:echo message))
```

## Service Binding

`bind_srv` turns a published service interface into local-looking functions.
Calling one of these functions sends a message to the service and waits for its
reply.

Bindings preserve function metadata, including `interactive` annotations. They
also import entity-completion defaults whose provider functions are exported in
the service's interface. Binding does not run those providers, and rebinding
refreshes that service's defaults without replacing local completion overrides.

```lyric
(bind_srv :echo)
(echo "hello") # => "hello"
```

## Service Discovery

The registry lets programs inspect and locate services by name without knowing
their process IDs or where they are running.

```lyric
(ls_srv)
(find_srv :echo)
(info_srv :echo :interface_doc)
```

## Pub/Sub

Pub/sub broadcasts transient updates through named topics. Subscribers receive
future publications through their normal process mailboxes.

```lyric
(subscribe :clock)
(publish :clock '(:tick 1))
(recv) # => (:topic_updated :clock (:tick 1))
```

## External Commands

`exec` invokes host programs and returns their exit status, standard output,
and standard error as Lyric data. `decode` can turn common text formats into
structured values.

```lyric
(def result (exec "printf" "one\ntwo\n"))
(decode :lines (get result :stdout)) # => ("one" "two")
```

## Script Execution and Initialization

`run` evaluates every top-level form in a file in a fresh process, as though the
file had an implicit `begin`. `vrsd` can run an initialization script before it
accepts clients; services spawned by that script continue running.

```lyric
(run "./scripts/counter.ll")
```

```sh
cargo run --bin vrsd -- --init ./scripts/init.ll
```

## Nodes and Peering

Multiple VRS daemons can exchange service registrations and route process
messages. Programs continue to discover services by name regardless of which
configured node currently provides them.

```lyric
(node_name) # => "laptop"
(configure :nodes '("ssh://home-server"))
(ls_srv)
```

## Hypermedia (WIP)

A service can return not just information to display, but actions the client can
take next. In vrsjmp, Lyric defines the contents and behavior; the GUI provides
the input field, list, and navigation stack. The client does not need to know
about individual integrations such as Feedbin or window management.

This is currently an application protocol, not a set of new language primitives.
The helpers below are ordinary functions in [vrsjmp.ll](../scripts/vrsjmp.ll).
The GUI carries command expressions back to that service; it does not evaluate
Lyric itself.

### Pages and Navigation

`push_page` returns an instruction naming the function that will produce a page's
items and the input field's prompt. Optional properties include a title, fixed
arguments, and a debounce delay in milliseconds.

```lyric
(defn read_later_page ()
  (+ (push_page 'read_later_items "Search saved pages…")
     '(:title "Read Later" :debounce_ms 200)))
```

The GUI pushes the page and requests its contents lazily through
`get_items(callback, args, query)`. That Lyric function invokes the named callback
with any fixed arguments followed by the input text. For Read Later, an empty
query shows the last 20 saved pages; other queries search them. Debounce defaults
to zero, while this page waits 200 milliseconds after typing stops.

The GUI owns the stack. Escape goes back, restoring the previous page's input and
selection; at the root it closes the palette. Reopening retains navigation for
eight minutes. The `begin_interaction` hook in `vrsjmp.ll` captures context and
returns the root page when opening, including when a retained page is restored.
Context capture is currently awaited before showing the palette.

### Rows and Actions

Rows describe a title, optional subtitle and right-aligned aside, a primary
command, and optional secondary actions for the Cmd-K menu.

```lyric
'(:title "VRS"
  :subtitle "github.com"
  :on_click (open_url "https://github.com")
  :actions ((:title "Copy URL"
             :on_click (set_clipboard "https://github.com"))))
```

For either a primary or secondary action, the GUI sends the selected command
record to the service's `on_click`. The service evaluates its `:on_click`
expression and returns a `:push_page` instruction or `:close`. Errors leave the
palette open and appear in a toast. Secondary actions may be explicitly supplied
by a page or generated from an entity's eligible commands.

### Interactive Calls

`call_interactively` uses function metadata and entity completions to build a
sequence of argument-selection pages, then calls the function with the selected
values. These helpers run inside the `vrsjmp.ll` service.

```lyric
(call_interactively 'focus_window)
```

For Focus Window, the service reads the `:os/window` argument annotation, invokes
the configured window providers, and returns rows representing windows. Selecting
one completes the call. Move Window to Display uses the same mechanism for two
arguments: first a window, then a display, with Back available between pages.

The same metadata works in reverse. Given an entity, `entity_actions` finds
interactive functions in the service's environment whose first argument has the
same type. It supplies that entity and prompts for any remaining arguments. This
supports context-derived commands and secondary actions without a separate
action registry; matching currently uses exact type tags.

## Introspection and Live Evaluation

The runtime exposes its current environments, processes, services, interfaces,
and documentation. These operations support experimenting from `vrsctl` or an
editor while the runtime remains active.

```lyric
(ls_env)
(help recv)
(ps)
(ls_srv)
```
