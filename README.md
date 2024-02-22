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

[vrs](https://github.com/leoshimo/vrs) is a WIP personal software runtime,
inspired by Emacs, Erlang, Unix, Plan 9, and Hypermedia systems.

It hopes to combine powerful ideas from those systems into one cohesive project,
so I can build rich personal software ecosystems and superplatforms at the speed
of thought.

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
(get item :subtitle)   # => "My Title"

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

# Helper: Wrapper around AppleScript to get dark mode appearance
(defn osa_get_darkmode ()
  (def (:ok result) (exec "osascript"
                          "-e" "tell application \"System Events\""
                          "-e" "tell appearance preferences"
                          "-e" "return dark mode"
                          "-e" "end tell"
                          "-e" "end tell"))
  (eq? result "true"))

# Helper: Wrapper around AppleScript to set dark mode appearance
(defn osa_set_darkmode (dark)
  (exec "osascript"
        "-e" "on run argv"
        "-e" "tell application \"System Events\""
        "-e" "tell appearance preferences"
        "-e" (if dark "set dark mode to true" "set dark mode to false")
        "-e" "end tell"
        "-e" "end tell"
        "-e" "end run")
  :ok)

# Initialize state of service to current appearance state
(def is_dark (osa_get_darkmode))

# Toggle appearance, flipping `is_dark`
(defn toggle_darkmode ()
  (set is_dark (not is_dark))
  (osa_set_darkmode is_dark))

# Fork this process into a service called :system_appearance with toggle_darkmode as exported interface
(spawn-srv :system_appearance :interface '(toggle_darkmode))
```
