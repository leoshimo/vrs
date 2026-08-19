# Core Concepts

## Evaluating Expressions

In a terminal:

```sh
vrsctl -c '(ls_srv)'
vrsctl --format pretty -c '(ls_srv)' | less
```

In Emacs's `vrs-mode`, put point on or just after the closing parenthesis:

| Keys | Action |
| --- | --- |
| `C-c C-e` | Evaluate the expression. |
| `C-u C-c C-e` | Replace it with the result. |
| `C-c C-r` | Evaluate the selected region. |
| `C-u C-c C-r` | Replace the region with its result. |
| `C-g` | Cancel a waiting evaluation and reset its session; keep the source. |

Results are formatted to 90 columns. Change `vrs-result-width` in Emacs or use
`--width` in the terminal. `(pretty VALUE)` returns formatted text;
`(pretty VALUE 60)` chooses a different width.

Evaluations share state: define a variable, function, or macro once, then use
it in later evaluations and macro expansions. Buffers using the same
`vrs-vrsctl-command` share that session. Editing a definition takes effect when
you evaluate it again.

`M-x vrs-reset-session` starts fresh, clearing definitions and bindings added
during evaluation. `C-g` also clears them when cancelling a blocked evaluation.
Neither undoes actions already performed or stops services you spawned.

## Source-embedded debug tools

Leave a useful observation beside the code with `dbg!`:

```lyric
(dbg! (map '(2 3) (fn (x) (+ x x))))
```

The block returns its ordinary result. Inspect its calls, actual arguments,
results, and source locations with `vrsctl dbg` or `vrsctl dbg --web`. Both
viewers can follow new observations or inspect previous invocations, including
nested callbacks. `C-g` still cancels the whole editor evaluation.

See [Source-embedded debug tools](source-embedded-debug-tools.md) for the execution
contract, source-transcript filters, browser workflow, and pub/sub data interface.

## Services

Services expose functions that other programs can call. Find them with:

```lyric
(ls_srv)
(info_srv :os_window :interface)
(info_srv :os_window :interface_doc)
```

Bind a service to use its functions:

```lyric
(bind_srv :os_window)
(get_windows)
(help focus_window)
```

In Emacs, `C-c C-b` (`vrs-browse-functions`) searches the functions bound in
your session by name, signature, service, and documentation. It inserts a call;
`C-u C-c C-b` fills arguments through completions or typed Lyric expressions.
Evaluate the inserted call when you want to run it.

To create a service, define its functions and list the ones to expose:

```lyric
(defn! echo (x) x)
(spawn_srv! :echo :interface '(echo))

(bind_srv :echo)
(echo "hello") # => "hello"
```

`spawn_srv!` starts a child service and returns when it is ready. Use `srv!`
instead when the current process should become the service; it keeps waiting
for requests. Define exported functions before starting the service.

## Building a Call in the Editor

Run `C-c C-b` (`vrs-browse-functions`) to choose a bound service function in
Emacs. Press Enter to insert a call with argument names as placeholders:

```lyric
(move_window window destination)
```

Use `C-u C-c C-b` to fill arguments before insertion. Choose live values from
[entity completion providers](#entities-and-completions), or enter a Lyric
expression when no choices are available. The result is ordinary source:

```lyric
(move_window '(:os/window :id 7) '(:os/display :index 2 :title "Display 2"))
```

The selected function is not executed. Entered argument expressions are parsed
without evaluation; completion providers run when their choices are requested.
`C-g` cancels without inserting a partial call.

You can also begin with a value. On an expression such as `(get_windows)`, use
`C-c C-v` (`vrs-choose-value`) to choose an entity and retain it as a literal.
Then `C-c C-a` (`vrs-act-on-value`) discovers an action for that entity, fills
remaining arguments, and replaces the entity expression with the call.
`M-x vrs-execute-action` instead runs the selected action and publishes it to
`:cmd` for recording. `vrs-choose-field` selects a field from a record.

The choice can happen in vrsjmp and still return to the editor: run
`M-x vrsjmp-browse-functions`, or evaluate `(vrsjmp_browse_functions)` with
`C-u C-c C-e`. The GUI uses the functions bound in its own service; Enter returns
a call with placeholders, and **Cmd-K → Fill arguments** fills it first.
The editor retains the returned call as source. Keep vrsjmp running for this
path; the Emacs commands work without it. Within vrsjmp itself, the
**Browse Functions** entry collects arguments and executes the selected call.

## Quotation and Templates

A list normally describes a call. Quote (`'`) keeps it as data; `eval` executes
that data as code:

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

Quote a list inserted as an argument so the generated call treats it as data:

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

(unless! false (+ 20 22))               # => 42
(unless! true (error "should not run")) # => nil
```

`& body` collects the remaining expressions. A function evaluates its arguments
before the call; a macro can choose which expressions run. Redefine a macro and
evaluate its call again to try the new version.

`and!` stops at the first false value; `or!` stops at the first true value:

```lyric
(and! (list? window) (get window :id))
(or! (get window :title) "Untitled")
```

Both return the last value evaluated. Like `if`, they treat `nil`, `false`, `0`,
`""`, and `'()` as false.

### Inspecting an Expansion

Quote the call to see the code it produces:

```lyric
(macroexpand_1 '(unless! false (+ 20 22)))
# => (when! (not? false) (+ 20 22))

(macroexpand '(unless! false (+ 20 22)))
# => (if (not? false) (begin (+ 20 22)) nil)
```

`macroexpand_1` expands once. `macroexpand` continues while the result is another
outer macro call; it does not expand calls nested inside that result. Neither
executes the returned code, but both run the macro body.

In Emacs, `C-c C-m` (also `C-c RET`) shows one expansion; `C-u C-c C-m` uses
`macroexpand`. Both can use definitions you evaluated earlier in the session.
You can also evaluate definitions and an inspection together:

```lyric
(begin
  (defn! echo (x) x)
  (macroexpand_1 '(srv! :test :interface '(echo))))
```

Use `C-u C-c C-e` at the end of this block to replace it with the expansion.

### Scope and Helpers

Macros can call ordinary functions. You do not need `for_syntax`; it behaves
like `begin`. Define helpers with `defn!` as usual.

The macro body uses definitions from where the macro was defined; the code it
returns runs in the caller's scope. To evaluate an expression using variables
at the call site while building that code, use `eval_caller`:

```lyric
(defmacro remember (expression)
  (def value (eval_caller expression))
  `(quote ,value))

(defn! example (x)
  (remember! (+ x 1)))

(example 41) # => 42
```

Here `eval_caller` sees the caller's `x`. It is only available while a macro is
expanding. Ordinary `eval` uses variables visible where it is called.

### Generated Names

Use `gensym` for temporary variables introduced by a macro:

```lyric
(defmacro or_else (expression fallback)
  (def temp (gensym "value"))
  `(let ((,temp ,expression))
     (if ,temp ,temp ,fallback)))

(def value 42)
(or_else! false value) # => 42
```

This evaluates `expression` once. Naming the temporary `value` would hide the
caller's `value` in the fallback. `gensym` creates a fresh name starting with
`value__`, avoiding that collision.

## Entities and Completions

An entity is a list with a type tag followed by properties:

```lyric
(def window '(:os/window :id 123 :app "Safari" :title "VRS"))
(get window :title) # => "VRS"
```

Add `interactive` to a function to tell vrsjmp which kind of value to offer
for each argument:

```lyric
(defn! focus_window (window)
  "Focus Window"
  (interactive :os/window)
  (exec "yabai" "-m" "window" (str (get window :id)) "--focus"))
```

Register a function that returns those choices:

```lyric
(set_entity_completions :os/window 'get_windows)
```

The provider takes no arguments and returns a list of entities. Bindings import
providers exported by a service. To inspect a function or its choices:

```lyric
(meta focus_window)
(get_entity_completions :os/window) # => (get_windows)
```

The type annotation guides the picker; it does not reject other values when
you call the function directly.

## Pages and Actions in Vrsjmp

To add a page, define an item function in
[scripts/vrsjmp.ll](../scripts/vrsjmp.ll). It takes the search text and returns
rows with titles and commands:

```lyric
(defn! project_items (query)
  (fuzzy_match query
    '((:title "VRS"
       :subtitle "github.com"
       :on_click (open_url "https://github.com/leoshimo/vrs")))
    (fn (item) (get item :title))))
```

Add an entry to `favorite_items` that opens the page:

```lyric
(make_item "Projects" '(push_page 'project_items "Find a project…"))
```

Enter runs a row's `:on_click` expression. Escape returns to the previous page.
Optional `:actions` use the same row format and appear in the Cmd-K menu.
The menu initially selects the first secondary action when available. An action
with the same `:on_click` form as its row is treated as primary and grouped above
the secondary actions. Typing reveals a search field at the bottom; Escape closes the menu
and restores the page search. Actions can include an optional `:icon` string:
`"open"`, `"link"`, `"copy"`, or `"command"` (the default).
Reload your edits with `(run "./scripts/vrsjmp.ll")`.

For an action that asks for arguments and then runs a function, use:

```lyric
(make_item "Focus a window" '(call_interactively 'focus_window))
```

`call_interactively` shows one selection page per argument, using the registered
completion providers. Selecting the last argument runs the function.

## Choosing Values in Vrsjmp

Choose one element of a list, or one field's value from a record:

```lyric
(vrsjmp_choose '("tea" "coffee" "water"))
(vrsjmp_choose (get_windows))
(vrsjmp_choose_field (active_tab))
```

The call waits for your selection. `vrsjmp_choose_field` shows keys and values;
it accepts keyword/value records and tagged entities. Empty input is an error.
Cancelling the picker raises an error in the waiting call.

## Processes and Messages

`spawn` starts a process. `self` returns the current process's ID; `send` and
`recv` exchange values:

```lyric
(def parent (self))
(spawn (fn () (send parent '(:greeting "hello"))))
(match (recv)
  ((:greeting message) message)) # => "hello"
```

`call` sends a request and waits for its reply. `call_timeout` sets the deadline
in seconds for this process's calls:

```lyric
(call_timeout 10)
(call (find_srv :echo) '(:echo "hello"))
```

## Pub/Sub

Subscribe to a topic to receive its future publications:

```lyric
(subscribe :clock)
(publish :clock '(:tick 1))
(recv) # => (:topic_updated :clock (:tick 1))
```

Earlier publications are not replayed to new subscribers.

## External Commands

`exec` returns a program's exit status, stdout, and stderr. `decode` turns text
into values:

```lyric
(def result (exec "printf" "one\ntwo\n"))
(decode :lines (get result :stdout)) # => ("one" "two")
```

For macOS launch helpers, bind `:os_browser` to use `open_url`, or `:os_apps`
to use `open_app` and `open_file`.

## Scripts and Initialization

`run` executes a file in a new process. Use it to start or reload a service:

```lyric
(run "./scripts/counter.ll")
```

Add service startup calls to [scripts/init.ll](../scripts/init.ll) to run them
when starting VRS with `./serve`.

## Nodes and Peering

Connect another node to discover and call its services by name:

```lyric
(node_name) # => "laptop"
(configure :nodes '("ssh://home-server"))
(ls_srv)
```

Commands run on the node hosting the service.

## Introspection

```lyric
(ls_env)    # Names in the current scope.
(help recv) # Documentation for a function.
(ps)        # Running processes.
```
