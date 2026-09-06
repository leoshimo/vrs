# Core Concepts

## Lyric Forms

Lyric uses the same representation for code and structured data. A form can be
constructed, stored, sent to another process, or evaluated later.

```lyric
(def command '(+ 40 2))
(eval command) # => 42
```

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
(spawn_srv :echo :interface '(echo) :overwrite)

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
