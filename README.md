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
# Use `def` to define new bindings - e.g. "hello lyric!" string to symbol `msg`
(def msg "hello lyric!")

# Update bindings with `set`
(set msg "goodbye lyric!")

# Basic Primitives - integers, lists, keywords, and more
(def var_number 42)                              # integers
(def var_keyword :my_keyword)                    # keywords start with colon (:)
(def var_bool true)                              # booleans are `true` or `false`
(def var_list (list msg var_number var_keyword)) # create new lists with `list`
(dev var_qlist '("a" "b" "c"))                   # quote expression with '

# Function declaration use `defn`
# Lyric is expression-oriented - last form is returned as value to caller
(defn double (x)
    (+ x x))
    
# Call functions by using bound symbol names within parens, followed by arguments
(double 10) # => 20

# List Operations
(def l '(1 2 3))
(def first (get l 1))       # get 0th item in `l`
(contains l 3)              # check if `l` contains `3`
(map l (lambda (x) (+ x x)) # => '(2, 4, 6) # map over list

# Conditionals with `if` - equality with `eq?`
(if (eq? msg "Hello")
    "msg was hello"
    "msg was not hello")

# Catch error with `try`, then check body's result with `err?` or `ok?`
(if (err? (try (jibberish)))
    "failed to call jibberish")

# Pattern match with `match`:
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

TODO: Examples for lambdas, coroutines, yielding, infinite iterators, macros

### Process

In VRS, software runs as *processes* running the Lyric programming language.

Each process has a single thread of execution. These processes are distinct from
OS processes, with a lighter footprint, only using CPU cycles if work is being
done.

- `ps`
- `spawn`

### Message Passing

- `ls-msgs`
- `send`
- `recv`
- `pid`

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

# Fork this process into a service called :system_appearance with toggle_drkmode as exported interface
(spawn-srv :system_appearance :interface '(toggle_darkmode))
```
