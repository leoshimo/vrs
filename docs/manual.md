# VRS Manual

## Overview

<a id="how-vrs-runs"></a>

VRS runs as a daemon, `vrsd`. Clients such as `vrsctl` connect to this running
environment to evaluate Lyric expressions. Code runs in [processes](#processes-and-messages),
which can exchange [messages](#message-passing) and offer functions as named
[services](#services).

| Component | Role |
| --- | --- |
| `Lyric` | VRS's scripting language. |
| `vrsd` | The runtime daemon. |
| `vrsctl` | CLI client. |
| `vrsjmp` | GUI client. |

<a id="start-and-evaluate"></a>

### Running VRS

You can start the daemon with:

```sh
vrsd
```

In another terminal, run `vrsctl` to open a REPL. The daemon stays running
between client connections. Services started by a client can continue after
that client disconnects.

#### Startup configuration

Use an init file to configure the environment at startup:

```sh
vrsd --init ./init.ll
```

The init file is a Lyric program. It can start services, configure peering, or
run other scripts. For example:

```vrs
# init.ll
(defn! hello () "Hello from VRS")
(spawn_srv! :greeting :interface '(hello))
```

## Lyric

Lyric is VRS's scripting language, used to configure the runtime, write
services, and call into the running environment. Programs are made of
expressions:

```vrs
(format "Hello, {}!" "VRS") # => "Hello, VRS!"
(+ 2 (+ 20 20))            # => 42
```

### Values

Values include numbers, strings, booleans, keywords, lists, and `nil`:

```vrs
(list 42 "hello" true :ready nil)
# => (42 "hello" true :ready nil)
```

Keywords such as `:ready` are values used as labels or field names.

### Strings

Strings use double quotes and escapes such as `\n` and `\"`.

| String helper | Example | Result |
| --- | --- | --- |
| Concatenate text | `(str "item " 3)` | `"item 3"` |
| Substitute values | `(format "{} items" 3)` | `"3 items"` |
| Join with a separator | `(join ", " "a" "b")` | `"a, b"` |
| Split on a separator | `(split "," "a,b")` | `("a" "b")` |

#### Multiline strings

Triple quotes allow multiline text and literal quotation marks. An indented
multiline string drops the opening newline and common indentation:

```vrs
"""
  She said "hello".
  Then she left.
  """
# => "She said \"hello\".\nThen she left.\n"
```

<a id="lists-and-keyword-fields"></a>

### Lists and records

Quote (`'`) returns an expression without evaluating it. The `list` function
builds a list from its arguments:

```vrs
'(1 2 3)         # => (1 2 3)
(list 1 (+ 1 1)) # => (1 2)
```

`get` accesses elements by zero-based index. `push` returns a new list with an
element appended:

```vrs
(get '(1 2 3) 0)  # => 1
(first '(1 2 3))  # => 1
(push '(1 2 3) 4) # => (1 2 3 4)
```

| List helper | Example | Result |
| --- | --- | --- |
| Length | `(len '(1 2 3))` | `3` |
| Last element | `(last '(1 2 3))` | `3` |
| Index from the end | `(get '(1 2 3) -1)` | `3` |
| Combine lists | `(concat '(1 2) '(3))` | `(1 2 3)` |
| Elements from an index onward | `(slice '(1 2 3) 1)` | `(2 3)` |

#### Records

Records are lists containing keywords and their values. Given a keyword, `get`
returns the value following it, or `nil` when the field is absent.

```vrs
(get '(:title "Notebook" :pages 80) :title)  # => "Notebook"
(get '(:title "Notebook" :pages 80) :author) # => nil
```

<a id="values-and-bindings"></a>

### Bindings

A symbol such as `title` names a binding. `def` creates or replaces a binding
in the current scope. `set` updates an existing binding.

```vrs
(def title "First draft")
(set title "Second draft")
title # => "Second draft"
```

The `symbol` and `keyword` functions construct name values:

| Conversion | Example | Result |
| --- | --- | --- |
| String or keyword to symbol | `(symbol "title")` | `title` |
| String or symbol to keyword | `(keyword 'title)` | `:title` |

Constructing a symbol does not define a binding. Quote a symbol, as in
`'title`, to use its name as data instead of looking up its value.

`let` introduces local bindings.

```vrs
(def amount 40)
(let ((amount 2) (original amount))
  (+ original amount)) # => 42
amount                # => 40
```

`begin` groups expressions, evaluates them in order, and returns the final
value. It does not introduce a scope.

```vrs
(begin
  (set amount (+ amount 1))
  (+ amount 1)) # => 42
```

#### Destructuring

A binding can use a pattern to take a value apart. Symbols bind matched values;
literals must match; `_` ignores a value.

Here the leading `:point` must match, and the next two elements become `x` and
`y`:

```vrs
(def (:point x y) '(:point 20 22))
(+ x y) # => 42
```

You can ignore fields or destructure nested lists:

```vrs
(def (:point x _) '(:point 20 22))
x # => 20

(def (:ok (a b)) '(:ok (20 22)))
(+ a b) # => 42
```

A mismatched pattern raises an error:

```vrs
(def (:point x y) '(:size 20 22)) # Error: the :point tag does not match.
```

### Functions

Define a named function with `defn!`. Lyric is expression-oriented: a function
returns the value of its last expression.

```vrs
(defn! double (x)
  (+ x x))
(double 21) # => 42
```

#### Anonymous functions and closures

`fn` creates an anonymous function. Functions are values that can be passed as
arguments or returned by other functions.

```vrs
(defn! with_value (x f)
  (f x))
(with_value 21 double)          # => 42
(with_value 41 (fn (x) (+ x 1))) # => 42
```

Functions retain access to their enclosing bindings. Each call to
`make_counter` below creates a counter whose value is retained by the returned
function:

```vrs
(defn! make_counter (count)
  (fn ()
    (set count (+ count 1))
    count))
(def next (make_counter 0))
(next) # => 1
(next) # => 2
```

#### Functions over collections

`map` and `filter` take a list followed by a function. `apply` calls a function
with the elements of a list as its arguments.

```vrs
(map '(1 2 3) (fn (x) (+ x x))) # => (2 4 6)
(filter '(0 1 0 2) (fn (x) (not? (eq? x 0))))
# => (1 2)
(apply + '(10 20 12)) # => 42
```

<a id="control-flow-and-patterns"></a>

### Control flow

`if` evaluates a condition and only the selected branch:

```vrs
(if (eq? 2 2) "equal" "different") # => "equal"
(if false "yes")                  # => nil
```

Falsy values are `nil`, `false`, `0`, `""`, and the empty list. Truthy values
include `true`, nonzero integers, and nonempty strings and lists.

These predicates return booleans:

| Predicate | Example | Result |
| --- | --- | --- |
| Equality | `(eq? 2 2)` | `true` |
| Membership | `(contains? '(1 2 3) 2)` | `true` |
| Empty string or list | `(empty? '())` | `true` |
| Value type | `(list? '(1 2))`, `(keyword? :title)`, `(symbol? 'title)` | `true` |
| Lyric function | `(lambda? (fn (x) x))` | `true` |
| Negation | `(not? false)` | `true` |

`cond` selects the first clause with a true condition:

```vrs
(cond ((eq? 3 1) :one)
      ((eq? 3 2) :two)
      (true :many)) # => :many
```

`when!` evaluates its body when the condition is true. `and!` and `or!` stop
when their result is determined and return the last value evaluated.

```vrs
(when! true (+ 20 22))       # => 42
(and! true "ready")         # => "ready"
(or! nil "Untitled")        # => "Untitled"
```

`loop` repeats its body.

#### Pattern matching

`match` compares a value with patterns in order and evaluates the first matching
clause. It uses the same patterns as destructuring bindings.

```vrs
(def result '(:ok (20 22)))
(match result
  ((:ok (a b)) (+ a b))
  ((:err reason) reason)
  (_ "unknown result")) # => 42
```

### Errors

`error` raises an error and stops the current evaluation unless it is caught:

```vrs
(error "Missing item")
```

`try` turns a raised error into an error value. A successful result passes
through unchanged. Use `err?` or `ok?` to check the result:

```vrs
(def result (try (error "Missing item")))
(err? result)   # => true
(try (+ 20 22)) # => 42
```

<a id="quotation-and-code-templates"></a>

### Code as data

Code can be stored and manipulated as data, then executed with `eval`.

#### Quoting and constructing expressions

Quote keeps an expression unevaluated:

```vrs
(def code '(+ 20 22))
(first code) # => +
(eval code)  # => 42
```

Backtick starts a *quasiquote* template. Comma inserts a computed value;
comma-at inserts the elements of a list.

```vrs
(def base 20)
(def extra '(10 12))
(def code `(+ ,base ,@extra))
code        # => (+ 20 10 12)
(eval code) # => 42
```

When the generated call takes a list or symbol as data, include the quote that
call will need:

```vrs
(def item '(:title "Notebook" :pages 80))
(def code `(get ',item :title))
code # => (get '(:title "Notebook" :pages 80) :title)
(eval code) # => "Notebook"
```

Quasiquotes can nest. The outer template starts at depth 1. Each nested
quasiquote adds a level; each comma (`unquote`) removes one. An unquote
evaluates its expression only when encountered at depth 1. At greater depths,
it remains in the template for later evaluation.

Inside two nested quasiquotes, `,x` therefore remains a substitution for the
inner template. `,,x`, shorthand for `(unquote (unquote x))`, crosses both
levels and substitutes `x` while building the outer template. Ordinary list
parentheses do not change quotation depth. An unquote outside a quasiquote
raises an error.

#### Reading expressions

`read` parses source text into a value. Use `eval` to execute it:

```vrs
(def code (read "(+ 20 22)"))
code        # => (+ 20 22)
(eval code) # => 42
```

<a id="readable-values"></a>

### Printing values

`display` returns a value's textual representation. `pretty` formats it as an
s-expression, optionally using a target line width:

```vrs
(display '(1 2 3)) # => "(1 2 3)"
(pretty '(:title "Notebook" :pages 80 :tags (work ideas)) 30)
# => "(:title \"Notebook\"\n :pages 80\n :tags (work ideas))"
```

### Macros

A macro receives unevaluated expressions and returns code to run in the caller's
scope. Define it with `defmacro` and add `!` to its name when calling it.

```vrs
(defmacro unless (condition & body)
  `(when! (not? ,condition) ,@body))

(unless! false (+ 20 22)) # => 42
```

`& body` collects the remaining expressions. Unlike a function, this macro can
decide whether those expressions are evaluated.

`macroexpand_1` shows one expansion. `macroexpand` continues while the result
is another outer macro call.

```vrs
(macroexpand_1 '(unless! false (+ 20 22)))
# => (when! (not? false) (+ 20 22))
(macroexpand '(unless! false (+ 20 22)))
# => (if (not? false) (begin (+ 20 22)) nil)
```

Macros expand when execution reaches the call, including inside functions and
loops. Redefining a macro changes subsequent calls, including those in existing
functions. Its body can use ordinary functions; the code it returns runs in
the caller's scope.

#### Avoiding name collisions

When a macro introduces a temporary binding, its name can hide one supplied by
the caller. Use `gensym` to generate a name that will not collide:

```vrs
(defmacro or_else (value fallback)
  (def temp (gensym "temp"))
  `(let ((,temp ,value))
     (if ,temp ,temp ,fallback)))

(let ((temp 42))
  (or_else! false temp)) # => 42
```

This evaluates `value` once. A fixed name `temp` in the generated `let` would
hide the caller's `temp` in the fallback expression, returning `false` instead.

### Evaluation and scope

When evaluating code stored as data, the environment determines which bindings
its symbols refer to.

| Form | Environment | Used for… |
| --- | --- | --- |
| `(eval expression)` | Current lexical environment. | Running a constructed expression with the bindings visible here. |
| `(eval_callsite expression)` | Call-site environment of the active macro expansion. | Inspecting caller-defined values while constructing a macro's output, as the service macros do with exported functions. |
| `(eval_global expression)` | Current process's top-level environment. | Evaluating or defining names outside local scopes, as `bind_srv` does when installing service bindings. |

`eval_global` is useful when a helper must define something that remains
accessible outside its own local scope.

For example, a local binding changes what `eval` sees, while `eval_global`
still sees the top-level binding:

```vrs
(def x 10)
(let ((x 42))
  (list :eval (eval 'x)
        :eval_global (eval_global 'x)))
# => (:eval 42 :eval_global 10)
```

A macro body uses the lexical environment where the macro was defined.
`eval_callsite` instead evaluates an expression in the lexical environment of
the call site where the macro is being expanded:

```vrs
(defmacro inspect_here (expression)
  `(quote ,(eval expression)))
(defmacro inspect_at_call (expression)
  `(quote ,(eval_callsite expression)))

(let ((x 42))
  (list :inspect_here (inspect_here! x)
        :inspect_at_call (inspect_at_call! x)))
# => (:inspect_here 10 :inspect_at_call 42)
```

Most macros return code that will use the caller's bindings when executed;
they do not need `eval_callsite` for that. Use it when the macro must inspect a
caller's value during expansion to decide what code to generate. Calling it
outside an active macro expansion raises an error.

A `def` evaluated through `eval_callsite` creates a binding in that call-site
environment. Its visibility follows that scope; `eval_global` targets the
process's top level.

#### Generating a dispatcher

Service macros inspect functions to generate request handlers. Here is a small
version: `dispatch!` reads a function's argument names from its metadata, then
builds a pattern and a call using those names.

```vrs
(defmacro dispatch (message name)
  (def function (eval_callsite name))
  (def args (map (get (meta function) :args)
                (fn (arg) (get arg :name))))
  `(match ,message
     ((,(keyword name) ,@args) (,name ,@args))))

(let ()
  (defn! greet (name) (str "Hello, " name))
  (dispatch! '(:greet "Ada") greet)) # => "Hello, Ada"
```

The macro expands to this expression inside the `let`:

```vrs
(match '(:greet "Ada")
  ((:greet name) (greet name)))
```

`greet` is defined at the call site. Inserting its name into generated code
would not require `eval_callsite`; reading its argument metadata during
expansion does.

<a id="documentation-and-metadata"></a>

### Help and introspection

A function's leading string is its docstring. `help` returns that documentation;
`meta` returns metadata, including argument names.

```vrs
(defn! double (x)
  "Add a number to itself."
  (+ x x))
(help double)            # => "Add a number to itself."
(get (meta double) :args) # => ((:name x))
```

`with_meta` returns a function with supplied metadata:

```vrs
(def labelled (with_meta double '(:category :math)))
(get (meta labelled) :category) # => :math
(labelled 21)                  # => 42
```

These queries describe the running environment. The runtime sections explain
services and entities in more detail.

| Question | Expression | What it returns |
| --- | --- | --- |
| What names are available here? | `(ls_env)` | Names of bindings visible in the current scope. |
| What does a function do? | `(help double)` | The function's documentation. |
| What arguments does it take? | `(get (meta double) :args)` | Argument names and any declared argument metadata. |
| What services are available? | `(ls_srv)` | Registered service names. |
| What functions does a service export? | `(info_srv :counter :interface)` | Exported function signatures. |
| Which functions operate on this type of data? | `(interactive_functions note)` | Bound interactive function names whose first argument matches the value's tag. |
| What values of this type are available? | `(entities :note)` | Tagged values supplied by registered entity sources. |

## Runtime

The runtime runs Lyric code in concurrent processes. Processes can communicate
through messages, expose functions as services, and subscribe to events.

<a id="processes-and-messages"></a>

### Processes

Each process has its own namespace of bindings, an ID, and a mailbox. A spawned
process starts with the bindings available to its function. Expressions within
a process run sequentially; other processes can run concurrently.

`spawn` starts a process from a function and returns its ID. The child can
continue running independently of its parent:

```vrs
(def worker (spawn (fn () (sleep 60))))
(kill worker)
```

| Expression | Purpose |
| --- | --- |
| `(self)` | Return the current process's ID. |
| `(ps)` | List running processes. |
| `(kill worker)` | Stop a process. |
| `(sleep 60)` | Wait for 60 seconds. |
| `(ls_env)` | List binding names visible in the current scope. |

### Message passing

`send` delivers a value to another process's mailbox. `recv` waits for a
message:

```vrs
(def parent (self))
(spawn (fn () (send parent '(:greeting "hello"))))
(match (recv)
  ((:greeting message) message)) # => "hello"
```

`recv` can take a quoted pattern to wait for a matching message, leaving other
messages in the mailbox. `ls_msgs` lists queued messages without consuming them:

```vrs
(send (self) '(:status "busy"))
(send (self) '(:greeting "hello"))
(recv '(:greeting _)) # => (:greeting "hello")
(ls_msgs)            # => ((:status "busy"))
```

### Services

A service is a named process with an interface of exported functions. Those
functions run in the service process and can access its state across calls.

Define the state and functions, then use `spawn_srv!` to start the service.
`:interface` lists the functions to export.

```vrs
(def count 0)
(defn! add_count (amount)
  (set count (+ count amount)))
(defn! get_count () count)
(spawn_srv! :counter :interface '(add_count get_count))
```

#### Finding and calling services

`bind_srv` installs bindings in the current process for a service's exported
functions. Calling one sends a request to the service, where the function runs.

The bindings include the functions' documentation and metadata, available
through `help` and `meta`.

```vrs
(bind_srv :counter)
(add_count 2)
(get_count) # => 2
```

Use `ls_srv` to list services, `find_srv` to find a service's process ID, and
`info_srv` to inspect its interface:

```vrs
(info_srv :counter :interface)
# => ((:add_count amount) (:get_count))
```

Service calls wait up to five seconds by default. You can change the timeout
for calls made by the current process:

```vrs
(call_timeout 30)
```

A timeout ends the caller's wait. The service may still complete the operation.

#### Updating a service

After changing the functions, reevaluate `spawn_srv!` to replace the service
under its existing name. Existing bindings resolve calls to the replacement.
Run `bind_srv` again to pick up new function names or changed signatures.

<a id="events-and-subscriptions"></a>

### Pubsub

VRS has built-in pubsub. A *topic* connects publishers to subscribers. Use
`subscribe` to register the current process, and `publish` to send a value:

```vrs
(subscribe :clock)
(publish :clock '(:tick 1))
(recv) # => (:topic_updated :clock (:tick 1))
```

Published values arrive in subscribers' mailboxes as `:topic_updated` messages.
Publishing does not wait for subscribers to handle them. Events reach
subscribers on connected nodes.

#### Subscribing a service

Services can handle events using `:topics`. Each entry pairs a topic with a
function that receives its published value:

```vrs
(def latest nil)
(defn! remember (value) (set latest value))
(defn! last_event () latest)
(spawn_srv! :latest_event
  :interface '(last_event)
  :topics '((:clock remember)))
(publish :clock '(:tick 2))
```

`remember` is called when a value is published on `:clock`. After it handles
this event, a bound `last_event` call returns `(:tick 2)`.

<a id="entities-and-interactive-functions"></a>

### Interactive functions and entities

An `interactive` declaration records the types of values a function accepts
as arguments. Here, `note_title` accepts a value tagged `:note`:

```vrs
(defn! note_title (note)
  (interactive :note)
  (get note :title))
```

An *entity* is a value identified by a leading type tag. Given such a value,
`interactive_functions` finds bound functions whose first argument declares
that type:

```vrs
(def note '(:note :title "Notebook"))
(interactive_functions note) # => (note_title)
(note_title note)            # => "Notebook"
```

The query returns function names. Each can be called with the original value.

#### Entity sources

An entity source is a function registered to supply values of a particular
type. Register the function's name with `register_entity_source`:

```vrs
(defn! get_notes ()
  '((:note :title "Notebook") (:note :title "Reading")))
(register_entity_source :note 'get_notes)
(entity_sources :note) # => (get_notes)
(entities :note) # Calls get_notes.
# => ((:note :title "Notebook") (:note :title "Reading"))
```

`entity_sources` lists the registered functions. The `entities` function calls
them to fetch available values.

A client can read a function's `interactive` metadata and use `entities` to
offer arguments of the declared type. The editor uses this information to
construct calls with values from the running environment.

When a service exports an entity source, binding the service makes that source
available to these queries along with its exported function metadata.

Registrations made with `register_entity_source` in the current process
override sources imported for that type. `(register_entity_source :note nil)`
removes the local override, restoring the imported sources.

### Files

`fdump` writes a Lyric value to a file. `fread` reads it back:

```vrs
(fdump "notes.ll" '((:note :title "Notebook")))
(fread "notes.ll") # => ((:note :title "Notebook"))
```

`fdump` replaces the file's contents. Paths are resolved on the device running
the code, including when it is evaluated from an editor on another device.

#### Running scripts

`run` evaluates a script in a new process, waits for it to finish, and returns
its final result:

```vrs
(run "./service.ll")
```

### External programs

Use `exec` to run external programs:

```vrs
(def result (exec "printf" "one\ntwo\n"))
result # => (:exit 0 :stdout "one\ntwo\n" :stderr "")
```

Decode the output with `decode`, which supports `:lines`, `:json`, and `:tsv`:

```vrs
(decode :lines (get result :stdout)) # => ("one" "two")
```

Pass `:stdin TEXT` to supply input to the program:

```vrs
(exec "cat" :stdin "hello") # => (:exit 0 :stdout "hello" :stderr "")
```

<a id="connected-devices"></a>

### Multiple devices

Each running `vrsd` is a node. Nodes can be connected to discover services,
call functions, and deliver pubsub events across devices.

#### Peering

On the second device, start a daemon with a node name:

```sh
vrsd --node home
```

From the first device's REPL, configure the connection using an SSH hostname
or alias that can reach the second device:

```vrs
(configure :nodes '("ssh://home"))
```

Here the SSH alias and VRS node name are both `home`. The runtimes communicate
through SSH. Direct TCP connections are also supported, as shown below.

#### Remote code and services

`remote!` evaluates expressions on a named node:

```vrs
(remote! "home" (node_name)) # => "home"
```

Code evaluated there can start a service. Other nodes can bind and call it
using the service's name:

```vrs
(remote! "home"
  (defn! host_name () (node_name))
  (spawn_srv! :host :interface '(host_name)))
(bind_srv :host)
(host_name) # => "home"
```

Once this `remote!` expression returns, `:host` is registered and visible to the
calling node, so the following `bind_srv` can use it immediately.

To evaluate a local script on a node, use `vrsctl --node home ./service.ll`.

#### Two runtimes on one machine

Give each daemon a node name, local client socket, and peer port. Start these
commands in separate terminals:

```sh
vrsd --node one --socket /tmp/vrs-one.sock --node-port 8873
```

```sh
vrsd --node two --socket /tmp/vrs-two.sock --node-port 8874
```

Connect a REPL to the first daemon:

```sh
vrsctl --socket /tmp/vrs-one.sock
```

Then connect it to the second node:

```vrs
(configure :nodes '("tcp://127.0.0.1:8874"))
```

`--socket` selects the Unix socket used by local clients; `--node-port` selects
the TCP port used by peers. For SSH peering with a custom port, use an address
such as `ssh://home:8874`, where `8874` is the remote VRS peer port.

### Advanced runtime operations

For most services, `spawn_srv!` and `bind_srv` are sufficient to start a service
and call its functions. For direct control over registration and request
handling, you can use the same process registration and message-passing
operations they build on: `register` names a process, `send` delivers a message,
and `call` sends a request and waits for a reply.

#### Serving in the current process

`srv!` runs a service in the current process. `spawn_srv!` forks a child process
to serve it and returns the child's ID once the service is registered. Both
accept the same interface and topic declarations.

For a script whose main job is serving requests, finish with `srv!`:

```vrs
(defn! greet (name) (str "Hello, " name))
(srv! :greeting :interface '(greet))
```

#### Naming and addressing processes

Use `register` when writing a process with its own message loop. It makes the
process discoverable by name; the process remains responsible for handling
its messages.

| Function | Purpose |
| --- | --- |
| `(register :worker)` | Register the current process as `:worker`. |
| `(find_srv :worker)` | Look up its process ID. |
| `(pid 42)` | Construct a process ID for process 42 on the current node. |
| `(ref)` | Create a unique value, useful for matching a reply to a request. |
| `(call pid request)` | Send a request to a process and wait for its reply. |

To call a service directly, supply its process ID and a request naming an
exported function, followed by its arguments:

```vrs
(call (find_srv :counter) '(:get_count))
```

#### Waiting for a service

`wait_srv` waits until a service name can be resolved and returns its process
ID. Use it when a dependency starts independently:

```vrs
(wait_srv :counter :timeout 10)
(bind_srv :counter)
```

This waits for registration. It does not call the service to check its health.

## vrsctl

### REPL

Run `vrsctl` without arguments for a REPL:

```sh
vrsctl
```

Each REPL instance runs in a process with its own namespace. Definitions and
service bindings remain available for that session.

### Shell scripting

Use `-c` to evaluate an expression and print its result:

```sh
vrsctl -c '(+ 20 22)' # => 42
```

A file or standard input can contain several expressions. The result of the
final expression is printed:

```sh
vrsctl ./service.ll
printf '(+ 20 22)\n' | vrsctl
```

Use `--bind` to bind a service before evaluating code, or `--node` to evaluate
on a peered node:

```sh
vrsctl --bind counter -c '(get_count)'
vrsctl --node home -c '(node_name)'
```

#### Formatting output

`--raw` prints string results without quotes for use in shell pipelines:

```sh
vrsctl --raw -c '(str "Hello, " "VRS")'
# Hello, VRS
```

Use `--format pretty` to format values across multiple lines:

```sh
vrsctl --format pretty --width 30 -c "'(:title \"Notebook\" :pages 80 :tags (work ideas))"
# (:title "Notebook"
#  :pages 80
#  :tags (work ideas))
```

#### Following a topic

Use `--subscribe` and `--follow` to print values as they are published:

```sh
vrsctl --subscribe clock --follow
```

Run `vrsctl --help` for all command-line options.

## Hypermedia

🚧 Under construction 🚧

See [Build a user interface](tour.md#build-a-user-interface) in the tour and the
[`:vrsjmp` service](../scripts/vrsjmp.ll).

## Editor

The Emacs major mode `vrs-mode` lets you evaluate code against the running
environment, inspect results, and bring values back into source code. You can
work one expression at a time while keeping definitions available between
evaluations.

### Setup

Load `vrs-mode` in Emacs, using the path to your checkout:

```elisp
(add-to-list 'load-path "/path/to/vrs/emacs")
(require 'vrs-mode)

;; If vrsctl is not on Emacs's executable path:
;; (setq vrs-vrsctl-command "/path/to/vrsctl")
```

### The editor session

The editor uses `vrsctl` to keep a process running for evaluations. Its
definitions and service bindings are shared by buffers using the same
`vrs-vrsctl-command`. Run `M-x vrs-reset-session` to start a fresh process with
a new namespace.

### Evaluate expressions and retain values

Enter these expressions in a file:

```vrs
(def note '(:note :title "Read paper"))
(get note :title)
```

Press `C-c C-c` to evaluate the buffer. The result buffer shows `Read paper`.

To evaluate only `(get note :title)`, place the cursor after its closing
parenthesis and press `C-c C-e`. The result is displayed in the same way.

With the prefix `C-u C-c C-e`, the result replaces the expression in the source
code:

```vrs
# Before: (get note :title)
"Read paper"
```

An active region can be evaluated with `C-c C-r`, or replaced by its result
with `C-u C-c C-r`. Retained lists and symbols are quoted so they remain data
when the surrounding program is evaluated.

### Introspecting the running environment

As a connected client, the editor can query registered services and function
metadata. `C-c C-s` browses services and their exported functions, binding a
selected service in the editor session. `C-c C-b` browses service functions
already bound in that session.

Choosing a function inserts a call with argument names as placeholders. Use a
`C-u` prefix to choose values for its arguments instead.

The earlier `interactive` metadata also supports starting from a value. Evaluate
this definition:

```vrs
(defn! note_title (note)
  (interactive :note)
  (get note :title))
```

Then put the cursor after this entire expression and press `C-c C-a`:

```vrs
'(:note :title "Read paper")
```

Choose `note_title`. The editor replaces the value with a call:

```vrs
(note_title '(:note :title "Read paper"))
```

This constructs code to inspect, edit, or evaluate. To choose and execute an
action on a value instead, use `M-x vrs-execute-action`.

<a id="command-reference"></a>

### Commands

| Key or command | Operation |
| --- | --- |
| `C-c C-c` | Evaluate the buffer. |
| `C-c C-e` | Evaluate the expression before the cursor. |
| `C-c C-r` | Evaluate the region. |
| `C-u C-c C-e` / `C-u C-c C-r` | Replace the expression or region with its result. |
| `C-u C-c C-c` | Evaluate the buffer and show a transcript of results. |
| `C-c C-m` | Expand the preceding macro call once; with `C-u`, expand successive outer macro calls. |
| `C-c C-s` | Browse services and insert an exported call; with `C-u`, choose argument values. |
| `C-c C-b` | Browse bound service functions; with `C-u`, choose argument values. |
| `C-c C-v` | Evaluate a list expression and replace it with a chosen element. |
| `C-c C-a` | Choose an action for a value and construct its call. |
| `M-x vrs-choose-field` | Choose a field from a value and insert it. |
| `M-x vrs-execute-action` | Choose and run an action for a value. |
| `M-x vrs-reset-session` | Start a fresh evaluation session. |

<a id="debugging"></a>

## Debugger

`dbg!` evaluates its body normally and returns its result while recording
function calls, arguments, results, errors, elapsed time, and source locations.
It can be used in any running program, including code evaluated from the editor.

Open a viewer:

```sh
vrsctl dbg       # Terminal viewer.
vrsctl dbg --web # Browser viewer.
```

Then wrap the code you want to inspect in `dbg!`:

```vrs
(dbg!
  (+ 2 (+ 20 20))) # => 42
```

Filter by error status, source file, expression, or elapsed time:

```sh
vrsctl dbg --filter 'status:error'
vrsctl dbg --filter 'file:notes.ll time:>100ms'
vrsctl dbg --filter 'expr:"(+ x x)"'
```

The web viewer accepts the same filter syntax. Multiple terms must all match.
`dbg_history` makes the recorded trace data available to programs.
