# Source-embedded debug tools

A source file can hold a small tool beside the computation it helps you understand.
Evaluate the marked code to produce observations; open a viewer to inspect them.
The tool stays useful the next time you return to that file. The source expresses
what to observe, while ordinary VRS data and clients supply ways to inspect it.

`dbg!` is the first instance of this idea. It wraps a block and records the calls
that execute within it, including their actual arguments, results, errors, and
source locations. It is general purpose: use it in a short expression, a script,
a function, or a service handler.

```lyric
(defn! twice (x)
  (+ x x))

(dbg!
  (def numbers '(2 3 5))
  (map numbers twice))
# => (4 6 10)
```

In Emacs `vrs-mode`, evaluate the definition and block with `C-c C-e`, or the
whole buffer with `C-c C-c`. The ordinary result appears in `*VRS Result*`.
Open the observations separately:

```sh
vrsctl dbg
vrsctl dbg --web
```

The CLI prints existing history and follows new observations until Ctrl-C. The
web command opens a temporary loopback server and browser tab; Ctrl-C stops that
viewer. `--no-open` prints its URL without launching a browser. Either viewer
can attach before or after evaluation. Closing a viewer does not stop recording.

For a complete example without Emacs:

```sh
vrsctl scripts/demo-dbg.ll
vrsctl dbg --once
vrsctl dbg --once --all --details
vrsctl dbg --web
```

Rebuild both `vrsd` and `vrsctl`, and restart your runtime, when installing this
feature. To select a different runtime, use `--socket PATH` with each command.

## Execution contract

`(dbg! BODY...)` behaves like `(begin BODY...)`: each expression runs once,
in order, in the original lexical scope. The last expression supplies the result;
an empty block returns `nil`. Definitions remain visible outside the wrapper.
The wrapper does not catch errors, change return values, or retry work.

Recording is active during execution of the block on its current fiber. Calls
inside previously defined functions are included, as are `map` and `apply`
callbacks. Nested wrappers belong to the same outer run. Every invocation has a
separate call ID and parent ID, so repetitions and concurrent runs remain distinct.

VRS attaches a recorder to its processes, so scripts and REPL evaluations work
just like editor evaluations. An embedded Lyric host without a recorder executes
the same body normally and collects no observations. Evaluating a quoted
`dbg!` form or inspecting its macro expansion does not execute the generated block.

`C-g` belongs to the Emacs evaluation command. It closes that evaluation's client
connection and resets its session. The runtime cancels the fiber; `dbg!` does not
turn cancellation into a normal return. Expressions after the interruption do
not run. Completed observations remain, and unfinished calls are marked
`cancelled`. Effects that happened before cancellation remain effects.

The existing `(dbg VALUE...)` function remains a compatibility print helper: it
writes Rust debug output to the runtime's stdout and returns `:ok`. It is distinct
from the `dbg` macro invoked as `dbg!`. With `scripts/serve.sh`, stdout goes to
`vrsd-MODE.log`. A deliberate `log!` macro is a follow-up TODO.

## Reading the recording

Both viewers support the same inspection tasks:

| Task | CLI | Browser |
| --- | --- | --- |
| Follow observations | `vrsctl dbg` | Live view |
| Inspect current history | `--once` | Pause view |
| Inspect nested calls | `--all` | Expand a call, or Expand nested calls |
| Inspect actual values | `--details` | Click source, or Show values |
| Find a file | `--file demo-dbg.ll` | File filter |
| Find an expression | `--expr '(+ x x)'` | Expression filter |
| See history at a location | `--at demo-dbg.ll:6:3` | Click a location, or Source location filter |
| Inspect one invocation | `--run ID` or `--call ID` | Run/Call filters and links in details |
| Export structured records | `--once --json` | Export JSON |

Run and call filters accept ID prefixes. A call filter includes that call's
children. Filters retain ancestors for context; those context rows need not match
the filter themselves. `--at` accepts `FILE:LINE` or `FILE:LINE:COLUMN`, with a
full path or a path suffix. Other file and expression filters use substrings.

The CLI initially shows wrappers and their immediate calls. `--all` includes the
nested calls; an explicit filter also reveals matches at any nesting level.
Short records occupy one source-transcript line:

```text
(map numbers twice)  # => (4 6 10)  [demo-dbg.ll:10:3 · 0.38ms · dk_-0ex8]
```

Long expressions use Lyric's pretty printer, and larger results continue in
`# =>` comment lines. `--width COLUMNS` controls that choice. Call IDs are compact
in the transcript; `--details` supplies full IDs, full paths, parent IDs, process
identity, and actual arguments. Slow calls produce a pending line and then a
completion with the same ID. `--json` includes all selected nested records and
prints a whole snapshot per update, so scripts can reconcile by cursor and ID.

The browser presents compact source rows with results and locations alongside
them. Nested calls and value details expand inline. Grouping by source location
shows repeated invocations together; grouping by run keeps each evaluation
together. Source location filters provide the corresponding history workflow in
the CLI. V1 has no depth limit flag or TUI.

## Which calls are shown?

A source-level call is observed when its callee and arguments have evaluated and
the VM is about to invoke it. Its lifetime includes native async suspension and
native code that executes more Lyric bytecode. If computing an argument fails,
the enclosing scope records the error; the outer call has not been invoked.

The recording includes nested calls, even small arithmetic calls. Viewers control
how much is visible; recording policy does not silently discard those calls.
`map` itself is one call. Each invocation of its callback is a child, and the
callback's source-level calls are children beneath that. Internal list assembly
and macro-transformer execution are not extra call rows. Special forms such as
`def` and `if` are not function calls; the `dbg!` wrapper supplies a scope row.

The transcript uses the original source expression, not a reconstructed call
with arguments substituted. Details show evaluated arguments separately. A
function argument is represented by its function source or a concise function
signature, never its captured environment or bytecode. Native callbacks show a
function definition with a `callback` label; their parent identifies the call
that invoked them. Generated syntax has an origin and a `generated` label rather
than a fabricated literal location. Anonymous dynamically constructed code may
have only a generated expression and no file location.

Observation does not automatically propagate into spawned processes or across
remote service boundaries. A local request call can still show its arguments,
result, and waiting time; add a `dbg!` block in a remote handler if you want to
observe its execution separately. This tool observes execution; it does not
pause stacks, step instructions, repair frames, or replay calls.

## Implementation and limits

`dbg` is a thin macro over the private `__debug_scope` special form. The parser
retains source provenance with syntax, without changing symbol identity or the
ordinary serialized `Form` representation. Macro list operations preserve that
provenance. The compiler places source descriptors on call instructions and
function bytecode. A small VM continuation owns each observed call until it
returns, errors, or is dropped during cancellation.

`eval_source` carries source text plus a file/buffer identity and one-based line
and column through the existing evaluation protocol. CLI files, `-c`, the REPL,
Emacs requests, `run`, and daemon init scripts use that path. Emacs sends the
region's actual origin even for unsaved buffers. A record retains its source
expression as it was evaluated; it does not reread an edited file afterward.

Records are ordinary property lists on the existing `:dbg` pub/sub topic:

```lyric
(subscribe :dbg)
(recv)
```

`(dbg_history)` returns a property list containing `:records`, `:cursor`,
`:dropped`, and `:evicted`. Each record includes `:id`, `:run`, `:parent`, `:kind`,
`:site`, `:arguments`, `:result`, `:status`, `:elapsed_us`, `:started_ms`,
`:process`, and `:sequence`. Integers that exceed Lyric's 32-bit range travel as
decimal strings. Scripts can use the existing `get`, `filter`, and `map` operations
to build another viewer. `vrsctl -s dbg -f` also exposes raw live updates.

Each runtime has one in-memory recording of up to 512 call/scope records.
Recording uses a nonblocking publication path; a slow viewer cannot hold up the
computation. Viewers reconcile with snapshots to recover dropped pub/sub
notifications. If the recording lock is busy, an observation may be dropped;
the counter explicitly reports this. Eviction or dropped observations can leave
missing parents or completions. History is cleared by restarting the runtime.

Each argument/result preview traverses at most 16 levels and stores up to 1,024
bytes, with an explicit truncation marker. At most 16 arguments are retained.
Source text/form fields are clipped after 4,096 bytes and paths after 1,024 bytes,
using an ellipsis. The viewer contains snapshots, not live object references.
All observed call sites still incur recording overhead; avoid large hot loops
when that overhead would distort the behavior being investigated.

Optional `RUST_LOG=vrs::observations=debug` logging prints accepted observation
publications from the pub/sub broker, outside the synchronous VM hook. V1 keeps
bounded history in the runtime; a TODO tracks moving that responsibility to an
ordinary service or retained pub/sub primitive if the design stays simple.

Future directions include a TUI, `log!`, and alternate web visualizations of
result data, including HTML rendering. The current web viewer displays source
and values as text and serves only a temporary, read-only loopback interface.
