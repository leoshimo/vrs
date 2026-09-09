<p align="center">
    <img width="450" src="https://raw.github.com/leoshimo/vrs/main/assets/vrs-venn.png">
</p>

> In the multiverse, you can live up to your ultimate potential. We discovered a
> way to temporarily link your consciousness to another version of yourself,
> accessing all of their memories and skills.
>
> It's called verse jumping.
>
> — Alpha Waymond

## What is this?

[vrs](https://github.com/leoshimo/vrs) is a personal programming runtime - a sandbox for building my “endgame” software platform.

The goal is to create a computing environment that brings me joy.

Each piece - the language, runtime, tools, and more - is designed as part of one holistic experience.

I live on a collection of personal software running on vrs every day, evolving the runtime as I go.

Inspired by systems I love - Emacs, Erlang, Unix, Plan 9, and Hypermedia - combining their powerful ideas into something that feels just right for me.

Built at the [Recurse Center](https://www.recurse.com/).

## Status

🚧 Under heavy construction, in perpetuity 🚧

🐉 Here be dragons 🐉

vrs is a *personal* passion project, focused on play + experimentation.

To this end:

- There are no stability guarantees. The implementation + concepts are volatile.
- Contributions of the intellectual kind are welcome ([reach me here](https://x.com/leoshimo)), but contributions of code are not accepted.
- [This software has sharp edges](https://www.youtube.com/watch?v=sCZJblyT_XM&t=310s). Be warned!

## Structure

- `lyric`: Embedded Lisp Dialect and Virtual Machine
- `vrsd`: A runtime implementation as a system daemon
- `libvrs`: The `vrs` library crate shared by runtime and client implementations
- `vrsctl`: A thin CLI client over `libvrs`
- `emacs`: The editor workflow for evaluating expressions and building calls
- `vrsjmp`: A GUI launch bar client that can also return choices to the editor

[Debugging](docs/guide-debug.org) ·
[Core Concepts](docs/concepts.md) · [Demos](docs/demos.md) ·
[Design Notes](DESIGN.org) · [AI and VRS](docs/ai.md)

## Init scripts and nodes

`vrsd --init PATH` evaluates a script inside the runtime before accepting local
clients. Unlike `fread`, `(run PATH)` evaluates all of a file's top-level
forms with an implicit `begin`, in a fresh process, and waits for that process
to finish. Services created by `spawn_srv!` keep running.

Every daemon has an immutable string node name. It defaults to the machine's
short hostname and can be set explicitly:

```sh
cargo run --bin vrsd -- --node laptop --init ./scripts/init.ll
```

The repository's `scripts/init.ll` can nonblockingly add nodes to the service
registry:

```lisp
(if (eq? (node_name) "home-server")
    (run "./scripts/feedbin.ll")
    (configure :nodes '("ssh://home-server")))
```

Use `./serve` (or `./scripts/serve.sh`) to start the runtime and GUI. It first
installs the current `vrsctl` with `cargo install --locked --path vrsctl --force`, so
Emacs and script shebangs use the matching client. `./serve dev` installs a
debug client for the debug runtime socket. Use `./serve headless` on a node
that should run the daemon and its services without launching the GUI.

Endpoints name their transport explicitly. Release builds use VRS port `8773`;
debug builds use `8774`, keeping a persistent `serve.sh` runtime isolated from
ordinary `cargo run` development. Append `:PORT` to either `tcp://HOST` or
`ssh://HOST` to select another port explicitly. The SSH transport runs
`ssh -W 127.0.0.1:PORT HOST`, so SSH aliases, Bonjour names, and Tailscale
MagicDNS names are resolved by OpenSSH without exposing the VRS listener. The
listener remains bound to localhost.

Keep a service near its data, tools, or hardware while editing it from your
laptop. Send a local file to a connected node:

```sh
vrsctl --node home-server ./scripts/feedbin.ll
```

The source runs there; its functions become available here through `bind_srv`.
Edit and resend it, and existing bindings follow the replacement. `remote!`
does the same for an inline block. The [speaker example](docs/concepts.md#nodes-and-peering)
puts sound on another Mac and calls it from a laptop.

To run two nodes on one machine, give each daemon a distinct local socket,
node name, and listener port:

```sh
cargo run --bin vrsd -- --node alpha --node-port 8773 --socket /tmp/alpha.socket
cargo run --bin vrsd -- --node beta  --node-port 8774 --socket /tmp/beta.socket
```

Both nodes must run compatible builds. Remote execution does not install a
restart policy or guarantee singletons. Sending to a disconnected node fails
immediately. Every `call`, whether local or remote, fails after five seconds if
the service has not replied; calls are never retried automatically. A process
can change its default with `(call_timeout 30)`.

<p align="center">
    <img src="https://raw.github.com/leoshimo/vrs/main/assets/vrs-arch-stack.png">
</p>


---

## A Tour of VRS

### Introduction to Lyric

VRS programs are written in Lyric, a small Lisp with lexical scope, first-class
functions, pattern matching, and macros:

```lyric
(defn! double (x) (+ x x))
(map '(1 2 3) double) # => (2 4 6)
```

The [Lyric guide](docs/guide-lyric.org) introduces the language through small,
runnable examples, from values and bindings to code templates and macros.
See the [Lyric reference](lyric/README.md) for detailed language contracts.

### Process

Each VRS process runs a Lyric fiber in a Tokio task, with its own environment
and mailbox. Async operations such as `recv` and `sleep` suspend the task while
waiting. CPU-bound Lyric code is not preempted and can occupy a worker thread.

Processes communicate through messages. Spawned processes copy their bindings;
closure environments can still share mutable state, so this is not complete
memory isolation.

```lyric
# See list of running processes in runtime
(ps)

# See this process's process_id
(self)

# Spawn a new process
(def echo_proc (spawn (lambda ()
    (def (sender msg) (recv))
    (send sender msg))))
```

### Message Passing

Each process has a dedicated mailbox that it can poll to receive messages:

```lyric
# See messages in mailbox, without blocking or consuming a message
(ls_msgs)

# Poll for new message. This blocks execution until a message is received:
(recv)

# `recv` can poll for messages matching specific patterns
(recv '(:only_poll_for_matching msg))

# A common idiom is a "service loop" - an infinite loop that recv messages and runs some function within the process:
(loop (match (recv)
    ((:event_a ev) (handle_a ev))
    ((:event_b ev) (handle_b ev))
    (_ (error "unexpected message"))))

# Sending messages is done via `(send PID MSG)`.
# Use process id from `(self)`, `(pid PID_NO)`, and `(find_srv SRV_NAME)`
(send (pid 10) :hello)
(send (self) :hello_from_self)

# Message from child back to parent
(def parent_pid (self))
(spawn (lambda ()
    (sleep 10)
    (send parent_pid :hello_from_child)))
```

### Services - Registry, Discovery, Binding

A service is a process registered under a name, with an exported interface.
`spawn_srv!` starts a child service and returns once it can receive messages:

```lyric
(defn! echo (message) message)
(spawn_srv! :echo :interface '(echo))

(find_srv :echo)           # process ID, including its node
(info_srv :echo :interface) # => ((:echo message))
(ls_srv)                  # service names and their registration records

(bind_srv :echo)
(echo "hello")            # => "hello"
```

`bind_srv` installs message-passing stubs in the calling process. `srv!` serves
requests in the current process instead of spawning a child. See
[Services](docs/concepts.md#services) for interface metadata and binding rules.

### PubSub

Topics broadcast messages to subscribers on the same runtime node.

```lyric
# Subscribe to :my_topic
(subscribe :my_topic)

# Publish data to :my_topic
(publish :my_topic '(:hello :world))

# Updates are received via mailbox:
(recv) # => (:topic_updated :my_topic (:hello :world))
```

---

## Examples

### Example: Counter Service

```lyric
#!/usr/bin/env vrsctl

# Internal state in process - count
(def count 0)

# Define an interface to increment count and publish over topic
(defn! increment (n)
  (set count (+ count n))
  (publish :count count))

# Serve a counter service, with `increment` as exported interface:
(spawn_srv! :counter :interface '(increment))
```

### Example: System Appearance Service

[scripts/system_appearance.ll](scripts/system_appearance.ll) wraps macOS
appearance settings with `exec` and exports `toggle_darkmode` as a service.

---

## Tooling

### REPL-driven workflows via `vrsctl`

`vrsctl` is a CLI client for vrs. When invoked without arguments, it launches into an interactive REPL useful for live programming and debugging:

```shell
$ vrsctl

vrs> (+ 20 22)
42

vrs> (defn! echo (x) x)
vrs> (spawn_srv! :echo :interface '(echo))
vrs> (bind_srv :echo)
vrs> (echo "hello")
"hello"
```

`vrsctl` also offers convenient interfaces and tools to support scripting and
debugging - see `vrsctl --help` for an overview of available commands.

Results automatically use multiline formatting when stdout is a terminal,
including the REPL, `--command`, script files/stdin, and subscriptions
(`--subscribe`, `--follow`, `--followclear`). Lists that fit stay on one line;
larger lists put each element on its own line. Keyword/value records such as
those returned by `(ls_srv)` keep small pairs together. The default target width is
90 columns in every output mode; use `--width` to choose another width. A nested value that is too
large to fit after its keyword starts on the next line.

```sh
vrsctl -c '(ls_srv)'                       # readable terminal output
vrsctl --width 60 -c '(ls_srv)'             # choose a target width
vrsctl --format pretty -c '(ls_srv)' | less # readable even through a pipe
vrsctl --format compact -c '(ls_srv)'       # force compact output
vrsctl --raw -c '(pretty (ls_srv) 60)'      # display the formatter's string
```

Redirected stdout stays compact by default. `--format default` selects this
terminal/pipe behavior explicitly. Strings remain quoted and escaped, including
embedded newlines; `--raw` prints only top-level strings verbatim. It does not
change nested strings. `--format editor` echoes source and prefixes every result
line with a comment, so multiline results remain safe in a Lyric transcript.
All output options apply to every input mode; subscriptions take priority over
redirected stdin. Explicit files cannot be combined with commands/subscriptions.

Formatting is also available inside Lyric:

```lyric
(pretty (ls_srv))       # returns a string, default width 90
(pretty (ls_srv) 60)    # positive integer width
(read (pretty '(1 2 3))) # => (1 2 3)
```

`pretty` returns text without printing or changing the value. `(display VALUE)`
keeps its compact behavior. The pretty representation of serializable values
can be parsed back with `read`; quote syntax and string escaping are preserved.
Raw block source strings become ordinary escaped string literals when printed,
preserving their decoded contents. Runtime-only values such as PIDs, references,
and functions retain their existing opaque display notation and do not round-trip.
Width is a target in display columns: atoms are never split, and a long string,
keyword/value pair, or deep nesting can exceed it. Nothing is truncated.

### Emacs Integration

[`emacs/vrs-mode.el`](emacs/vrs-mode.el) provides syntax highlighting,
indentation, and evaluation for `.ll` files using built-in Emacs libraries.
Add the repository's `emacs` directory to `load-path` and load the mode:

```elisp
(add-to-list 'load-path "/path/to/vrs/emacs")
(require 'vrs-mode)
```

Evaluation sends the source to `vrsctl` unchanged, including raw block strings
and reader prefixes. Customize `vrs-vrsctl-command` to set the executable path
or add options such as service bindings.

- `C-c C-e` evaluates the expression at its closing parenthesis or preceding
  point, including its quote prefix.
  Try `(ls_srv)` or `(pretty (ls_srv) 60)` to see readable results in the
  `*VRS Result*` buffer. Top-level strings are displayed as text.
- `C-c C-r` evaluates the region; `C-c C-c` evaluates the buffer. Both currently
  display only the last top-level result, evaluating the source as an implicit
  `begin` in the persistent session.
- `C-u C-c C-e` and `C-u C-c C-r` retain the result as literal source and indent
  it in context, including quotes for lists and symbols. Unrepresentable values
  and evaluation errors leave source intact. `M-x vrs-insert-evaluated-code`
  instead inserts generated code without adding a quote or executing it.
- `C-u C-c C-c` evaluates the buffer expression by expression and displays source
  with commented results: `(+ 1 2)` followed by `# => 3`, then `(+ 3 4)` followed
  by `# => 7`. Use this for a scratch-buffer transcript.
- `C-c C-m` inspects one macro expansion; `C-u C-c C-m` repeats outer expansion.
- `M-x vrs-reset-session` starts a fresh connection and clears its definitions.
- `C-g` aborts a waiting evaluation and clears its session, preserving source.
  Effects already performed are not undone.

Buffers using the same `vrs-vrsctl-command` share a session. Variables, functions,
macros, and service bindings persist between evaluations, including after errors.
Re-evaluate a changed definition to update it.

In Emacs, `C-c C-m` displays one expansion of the form at its closing
parenthesis or preceding point, adding the required quote automatically; a
prefix argument repeats outer expansion. For example, use it on
`(srv! :test :interface '())`, or evaluate
`(macroexpand_1 '(srv! :test :interface '()))`. Without the quote, the service
loop runs before the inspection function can be called. `C-g` cancels a waiting
evaluation and clears its session. Evaluation and macro expansion share a
persistent editor session, so earlier evaluated definitions remain available.
`M-x vrs-reset-session` explicitly starts a fresh connection.

Choose values and build calls using ordinary Emacs completion, including your
configured completion packages:

| Command | Shortcut | Result |
| --- | --- | --- |
| `vrs-choose-value` | `C-c C-v` | Evaluate a list, choose an element, replace the source with its literal value. |
| `vrs-choose-field` | `M-x` | Evaluate a record or tagged entity, choose a field, replace the source with its literal value. |
| `vrs-browse-functions` | `C-c C-b` | Insert a service call with argument names as placeholders. With `C-u`, fill its arguments first. |
| `vrs-act-on-value` | `C-c C-a` | Evaluate an entity, choose an action, fill its remaining arguments, and replace the source with the call. |
| `vrs-execute-action` | `M-x` | Evaluate an entity, choose and execute an action, and display its result while keeping the source. |

Value, field, and action commands use the active region, or the expression at its
closing parenthesis or before point. A region can contain several forms; the last
value is used. The expression runs once. For example, choose from
`'((:os/process :pid 10) (:os/process :pid 20))` with `C-c C-v`; selecting the second
entry leaves `'(:os/process :pid 20)` in the buffer. `vrs-choose-field` can then
retain `20`. Lists and symbols receive the quote needed to evaluate as literal
data; strings retain their escaping. Value and argument choices show the full
Lyric value, with a row number to distinguish duplicates. Field choices show the
key alongside its value. Opaque values such as
runtime references and functions cannot be retained as source.

The function browser searches names, signatures, services, and documentation of
methods **bound in the Emacs session**. Evaluate `(bind_srv :SERVICE)` first, or
add `--bind SERVICE` to `vrs-vrsctl-command`. It does not import every
registered service automatically. Noun–verb discovery also includes session-local
functions declared `interactive`; an action matches when its first argument's
declared type equals the entity's leading tag. It uses the same metadata rules
as vrsjmp. For example, evaluate this harmless definition:

```lisp
(defn! process_pid (process)
  "Read PID"
  (interactive :os/process)
  (get process :pid))
```

On `'(:os/process :pid 20)`, `C-c C-a` offers **Read PID** and constructs
`(process_pid '(:os/process :pid 20))`. On the same entity,
`M-x vrs-execute-action` runs the selected action and displays `20`.
Both commands also work directly on an expression that returns the entity, such
as `(get '((:os/process :pid 20)) 0)`. There is no need to retain a literal
first: the expression is evaluated once before choosing the action. Execution
keeps that expression in place; construction replaces it with the complete call.

Argument filling uses the session's `get_entity_completions` providers. If no
choices are available, enter a Lyric expression such as `"hello"`, `42`, or
`'(a b)`. Entered expressions are parsed without evaluation when constructing
a call. Selecting functions or constructing calls never executes those calls.
Entity expressions and requested completion providers do run. The explicit
execute command publishes the selected call to `:cmd` for command macro recording,
then executes it once, using the same publication and failure handling as vrsjmp.

`C-g`, evaluation errors, and edits to or closure of the source buffer cancel
construction without committing a partial replacement. Buffer changes
detected before execution also prevent dispatch. Cancelling a pending runtime
request resets its session; effects already performed cannot be undone.
These helpers require a daemon built from this version of VRS; rebuild and restart
your chosen runtime after updating. The earlier
`vrs-browse-functions-minibuffer` name remains an alias for `vrs-browse-functions`.
The dedicated [Working in Emacs guide](docs/guide-emacs.org) covers these interactions
in Org format, with all example definitions inline. Paste its setup into
`scratch.ll` to choose values and fields, build or execute actions from literals
and calls, inspect a transcript, and bring a GUI choice back into source.

A choice can also happen in another app and return to the waiting editor.
Vrsjmp's **Browse Functions** opens the service-function list for execution.
Enter calls the selected function after collecting its arguments. When no
completion provider exists, enter a Lyric expression such as `"hello"` or `42`.

`M-x vrsjmp-browse-functions` makes the choice in vrsjmp and brings the call
back into the editor. You can
also use `M-x vrs-insert-evaluated-code` on `(vrsjmp_browse_functions)`. Search by function
or service name; each row shows its service. Press Enter
to insert its form with argument names as placeholders. In the Cmd-K actions menu,
choose **Fill arguments** to select one value per argument, using the same choices
as `call_interactively`. The last selection returns the form without executing it.
Escape out of the picker cancels the evaluation and leaves the original expression intact.

The GUI must be running for automatic opening. Requests remain in the
`:vrsjmp` service if a wakeup is missed, and are checked again on opening or
reconnection. `(show_gui)` asks a running GUI to open through its normal
`root_page`/`get_items` flow.

Customize `vrs-result-width` (default 90) for editor results. Indentation
aligns data and keyword/value lists under their opening parenthesis, uses two
spaces for call bodies, and preserves raw block string contents. The result
buffer uses Lyric syntax highlighting and is read-only.

Run the Emacs tests without a runtime:

```sh
emacs -Q --batch -L emacs -l vrs-mode-tests -f ert-run-tests-batch-and-exit
```

The terminal harness starts a separate test runtime and includes Emacs evaluation
tests when `emacs` is installed:

```sh
cargo build --locked -p vrsctl -p vrsd
cargo test --locked -p vrsctl --test terminal
```

Add `--release` to both commands to exercise optimized binaries. Build `vrsd`
first because it belongs to a separate Cargo package; the tests use the daemon
next to Cargo's `vrsctl` binary, including with a custom target directory.

The terminal harness uses a temporary socket, an ephemeral node port, and an
init fixture that starts a small in-memory service. It checks
startup, command, file, stdin, REPL, and subscription output on pipes and
pseudo-terminals, and keeps the user's runtime and REPL history intact.
