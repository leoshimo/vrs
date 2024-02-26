[![Rust](https://github.com/leoshimo/vrs/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/leoshimo/vrs/actions/workflows/rust.yml)

<p align="center">
    <img width="360" src="https://raw.github.com/leoshimo/vrs/main/assets/vrs.png">
</p>

🚧 Under Heavy Construction

> In the multiverse, you can live up to your ultimate potential. We discovered a
> way to temporarily link your consciousness to another version of yourself,
> accessing all of their memories and skills.
>
> It's called verse jumping.
>
> — Alpha Waymond

## What is this?

[vrs](https://github.com/leoshimo/vrs) is a personal software runtime - an
opinionated take on my "endgame" software platform.

Its key inspirations are Emacs, Erlang, Unix, Plan 9, and Hypermedia
applications. By combining powerful ideas from those systems into one cohesive
project, it hopes to accelerate development of my personal software ecosystem.

Its key principles are: 

- simplicity
- pragmatic
- joy

## Status

vrs is a sandbox project, focused on play and experimentation in a pure-fun,
pure-utility environment. While I live-on vrs everyday, the platform is very
volatile in both concepts and implementation. Be warned!

## Structure

- `libvrs`: The `vrs` library crate shared by runtime and client implementations
- `vrsd`: A runtime implementation as a system daemon
- `lyric`: Embedded Lisp Dialect and Virtual Machine
- `vrsctl`: A thin CLI client over `libvrs`
- `vrsjmp`: A GUI launch bar client

---

## A Tour of VRS

### Introduction to Lyric

The runtime runs software written in Lyric lang:

```lyric
# Use `def` to define new bindings
# e.g. "hello lyric!" string to symbol `msg`
(def msg "hello lyric!")

# Update bindings with `set`
(set msg "goodbye lyric!")

# Basic Primitives - integers, lists, keywords, and more
42                                # integers
:my_keyword                       # keywords start with colon (:)
true                              # booleans are `true` or `false`
(list msg var_number var_keyword) # create new lists with `list` function
'("a" "b" "c")                    # quote expression with '

# Function declarationes use `defn`
# Lyric is expression-oriented - last form is returned as value to caller
(defn double (x)
    (+ x x))
    
# Call functions by using bound symbol names within parens, followed by arguments
(double 10) # => 20

# List Operations
(def l '(1 2 3))
(def first (get l 0))       # get 0th item in `l`
(contains l 3)              # check if `l` contains `3`

# Association Lists
(def item '(:title "My Title" :subtitle "My Subtitle"))
(get item :title)      # => "My Title"
(get item :subtitle)   # => "My Subtitle"

# Functions (Lambdas) are first class
(defn apply (x fn)
    (fn x))
(apply 41 (lambda (x) (+ x 1)))        # => 41
(map '(1 2 3) (lambda (x) (+ x x))     # => '(2, 4, 6)

# Conditionals with `if` - equality with `eq?`
(if (eq? msg "Hello")
    "msg was hello"
    "msg was not hello")

# and flip conditions with `not`
(if (not false)
    "it was not true")

# Catch error with `try`. Introspect result with `err?` or `ok?`
(if (err? (try (not_a_function)))
    "failed to call not_a_function")

# Pattern match with `match`. `_` is a wildcard pattern.
(def result '(:ok "Successful data"))
(match result
    ((:ok msg) msg)
    ((:err err) (:err err))
    (_ '(:err "Unrecognized result")))

# Destructuring bindings can be used to pattern match against forms:
(def result '(:ok "Success"))
(def (:ok status) result)      # matches :ok, binds status to string "Success"

# As a Lisp, Lyric has `eval` and `read`:
(eval (read "(+ 40 2)")) # => 42

# and there are more builtins and symbols in environment, introspectable via `ls-env`
(ls-env)
```

TODO: Examples for fibers, coroutines, yielding, infinite iterators, macros

### Process

In VRS, software runs as *processes* running Lyric lang.

These processes are implemented as [green threads](https://en.wikipedia.org/wiki/Green_thread),
and are lightweight compared to OS processes. Processes are scheduled on
multiple cores in an IO-aware manner.

Each process has a single thread of execution. CPU-bound and IO-bound work is
transparent at program level, but the runtime schedules work such that a
processes waiting for IO or running CPU-intensive work do not block cores.

Millions of processes can run on a single machine, without a single process
halting the system altogether.

With lightweight processes and a sequential programming model to handle IO-bound
and CPU-bound work transparently, the intention is to simplify typical
event-based or callback-based idioms used for building software.

For example, annual jobs can be represented as a infinite looping program that sleeps for a year:

```lyric
(loop (sleep (duration :years 1))
      (do_a_thing))
```

And user flows can be represented sequentially, without blocking the "main thread":

```lyric
(def query (prompt "Enter search term: ")) # block on user response
(def items (search-items query))           # network-bound query
(def selection (select items))             # block on user selection
```

Processes run in isolated environments from one another - symbols bound in one
process cannot be seen by another process.  The only method for communicating
between process is via *message passing*, covered below.

```lyric
# See list of running processes in runtime
(ps)

# See this process's process-id
(self)

# Spawn a new process
(def echo_proc (spawn (lambda ()
    (def (sender msg) (recv))
    (send sender msg))))
```

### Message Passing

Processes are isolated - and communicate through message-passing.

Each process has a dedicated mailbox that it can poll to receive messages:

```lyric
# See messages in mailbox, without blocking or consuming a message
(ls-msgs)

# Poll for new message. This blocks execution until a message is received:
(recv)

# `recv` can poll for messages matching specific patterns
(recv '(:only_poll_for_matching msg))

# A common idiom is an infinite loop that recv messages and dispatches some action internal to service:
(loop (match (recv)
    ((:event_a ev) (handle_a ev))
    ((:event_b ev) (handle_b ev))
    (_ (error "unexpected message"))))

# Sending messages is done via `(send PID MSG)`, often with `(self)` or `(pid PID_NO)`:
(send (pid 10) :hello)
(send (self) :hello_from_self)

# or from spawned children, back to parent
(def parent_pid (self))
(spawn (lambda ()
    (sleep 10)
    (send parent_pid :hello_from_child)))
```

### Services - Registry, Discovery, Linking

- `register`
- `srv`
- `spawn-srv`
- `bind-srv`
- `ls-srv`
- `find-srv`

### PubSub

- `subscribe`
- `publish`

### Example: Counter Service

```lyric
#!/usr/bin/env vrsctl

# Internal state in process - count
(def count 0)

# Define an interface to increment count and publish over topic
(defn increment (n)
  (set count (+ count n))
  (publish :count count))

# Serve a counter service, with `increment` as exported interface:
(srv :counter :interface '(increment))
```

### Example: System Appearance Service

```lyic
#!/usr/bin/env vrsctl
# macOS System Appearance Integration
#

# Get system appearance state
(defn is_darkmode ()
  (def (:ok result) (exec "osascript"
                          "-e" "tell application \"System Events\""
                          "-e" "tell appearance preferences"
                          "-e" "return dark mode"
                          "-e" "end tell"
                          "-e" "end tell"))
  (eq? result "true"))

# Set system appearance state
(defn set_darkmode (dark)
  (exec "osascript"
        "-e" "on run argv"
        "-e" "tell application \"System Events\""
        "-e" "tell appearance preferences"
        "-e" (if dark "set dark mode to true" "set dark mode to false")
        "-e" "end tell"
        "-e" "end tell"
        "-e" "end run")
  :ok)

# Toggle current state
(defn toggle_darkmode ()
  (set_darkmode (not (is_darkmode))))

# Fork into service exporting `toggle_darkmode` as service
(spawn-srv :system_appearance :interface '(toggle_darkmode))
```

---

## Tooling

### REPL-driven workflows via `vrsctl`

`vrsctl` is a CLI client for vrs. When invoked without arguments, it launches into an interactive REPL useful for live programming and debugging:

```shell
$ vrsctl

# Experiment with lyric:
vrs> (def url "https://github.com/leoshimo/vrs")
"https://github.com/leoshimo/vrs"
vrs> (open_url url)
(:ok "")

# Introspect runtime state:
vrs> (ls-srv)
((:name :launcher :pid <pid 28> :interface ((:get_items) (:add_item title cmd)))
 (:name :system_appearance :pid <pid 5> :interface ((:toggle_darkmode))))
 
# Bind and talk to services:
vrs> (bind-srv :launcher)
((:get_items) (:add_item title cmd))
vrs> (add_item "Hello" '(open_url "http://example.com"))
:ok
```

`vrsctl` also offers convenient interfaces and tools to support scripting and
debugging - see `vrsctl --help` for an overview of available commands.

### Emacs Integration

There is an major-mode available for Emacs - `lyric-mode`.

It provides syntax highlighting and bindings useful for bottom-up, interactive,
editor-centric software development.

The package is currently not available via package repositories - but is
available in my [dotfiles repository](https://github.com/leoshimo/dots/blob/527bd86095f7c082e6fd6a7658698c8745c65be0/emacs/.emacs.d/init.org#lyric--vrs).
