# VRS: What comes after the Unix pipe?

I want the software I use every day to remain something I can work on. A useful
program should be a place to start: inspect it, borrow a piece, change what it
does, or connect it to another program. The same should be true of the
environment those programs inhabit.

VRS is my attempt to build that environment. It combines the practicality of
shell scripting with the exploratory feel of Emacs, across the applications
and devices I actually use. Services expose functions. Clients run expressions
and present results. The language, user interface actions, and recordings share
symbolic expressions, so an interaction can become source for another program.
The editor works inside the same running system.

This is the experience-led version of the design. The [decision-led version](design.md)
gives each mechanism its own section; the [tour](tour.md) shows them working together.
Both describe the current environment. Longer proposals live in [TODO](../TODO.org).


<a id="clay-and-duct-tape"></a>

## Clay and duct tape

The goal is to create a computing environment that brings me joy. I live on a
collection of personal software running on VRS, evolving the runtime as I go.
There is no requirement that every experiment become a polished application.
Sometimes a small script is the right amount of software.

Building that script can still involve an absurd amount of surrounding work.
An application has useful data but no convenient API. Another application
could use it, but only through a bespoke integration. A graphical interface
needs a different representation of the same action. Running something on
another machine adds another set of conventions.

I want to compose the software I already have and write the parts that are
missing. Lyric provides the glue, bringing existing programs into a shared
environment where their capabilities can be combined, scripted, and used
across devices.

For this kind of work, I want clay and duct tape. Software can feel like having
to CNC-mill every trivial connection. Gary Bernhardt's [Unix Chainsaw](https://www.youtube.com/watch?v=sCZJblyT_XM) gets at
the attitude: half-assed, but the right half of the ass. Use the program that
already works. Spend the effort on the part that matters.

VRS tries to make that attitude practical across a much larger part of the
computer. The language, runtime, service model, clients, and editing experience
are designed together. A small shared vocabulary can remove repeated work
between them. The ambition is broad; the pieces can remain small.

That is why simplicity matters to me. It leaves more of the software within
reach of its author. Understanding and changing a useful program should remain
ordinary activities after it starts being useful. Personal programming is as
much about continued authorship as about getting the first version to run.


<a id="make-glue-first-class"></a>

## Make glue first class

Unix makes programs written independently useful together. Its common
conventions let one program's output become another program's input. What
comes after the Unix pipe, when the material includes application data,
interactive actions, and capabilities on other devices?

VRS uses values and expressions as that material. A *service* is a running
program with a name and exported functions. It can implement behavior in
Lyric or hack it together from command-line programs, local databases, and
UI automation. Other programs use the resulting functions.

The [Safari service](../scripts/os_browser.ll) is an example of the practical side. It uses AppleScript
to obtain the active page, and SQLite to read tabs synced from other devices.
There isn't one blessed route into the application. There are useful ways in,
and a service that presents their results consistently.

Once available, those results can become inputs to other software. With the
browser and the tour's todo service bound:

```vrs
(def tab (active_tab))
(add_todo "Try VRS"
  (format "{}\n{}" (get tab :title) (get tab :url)))
```

This is an ordinary program using two services. The browser-specific work
belongs to the browser service. A todo list, an automation, or a graphical
interface can use the same access without each implementing a Safari extension.

[Apple Notes](../scripts/os_notes.ll), [Antinote](../scripts/antinote.ll), and [Feedbin](../scripts/feedbin.ll) use different combinations of databases,
application URLs, AppleScript, and dedicated CLIs. The programs underneath can
be written in whatever language suits the job. Lyric makes their capabilities
available in a shared programming environment.

The useful promise is that an integration has more than one consumer. In the
ideal case, M services and N clients need M adapters and N runtime clients,
rather than M×N application-specific connections. The adapter still has to be
written and maintained. The next program gets to use it.


<a id="the-application-uses-code"></a>

## The application uses code

The same expression can describe an action in a user interface:

```vrs
'(:title "Add todo: Buy coffee"
  :on_click (add_todo "Buy coffee" ""))
```

There is no new command language hidden in this item. Its action is the call
itself, with its arguments included. A program can extract and evaluate it;
another program can construct an item using the same list operations.

The effect is programmed normally. Its call is also data that another program
can inspect, collect, or transform. An expression built from that data can run
as code again.

The editor participates in this too: it can insert calls into source, while
other programs can construct actions for the user interface. I don't want
another layer of app-intent declarations or action schemas translating these
operations back into calls. Such layers can become weak approximations of
programming languages. The page and item formats still define a contract for
clients, but the action itself is already expressed in the language.

This representation is powerful for both users and programs. A person can
edit an action into a script, and another program can
turn it into a recording or capture its argument values for retry. The function
being called needs no special recording or retry interface.

This is where “everything is S-expr” and *code mode* come together. Symbolic
expressions describe data, programs, interface markup, and actions. Service
discovery supplies callable functions. An operation can move from a REPL into
a script or a user interface without needing another representation of what
it does.

It doesn't mean every runtime object is printable source. Functions execute
as bytecode, and values such as process IDs and references have their own
representation. The important continuity is between the things a programmer
reads, constructs, invokes, and keeps.


<a id="use-a-feature-get-a-program-example"></a>

### Use a feature, get a program example

The shared command path can publish selected expressions through pubsub.
After choosing *Add todo: Buy coffee*, a subscriber can receive:

```
(add_todo "Buy coffee" "")
```

That expression can be edited, replayed, or pasted into a larger program. The
[command-macro service](../scripts/cmd_macro.ll) records a sequence by collecting such expressions into
a `begin` form. It uses the same evaluator as a hand-written program.

Using an application becomes a way to explore how to program it. A familiar
interaction supplies a function call with real arguments. This is something I
value in Emacs: the program remains accessible through the commands that make
it work. VRS extends that relationship across clients and services.

Recording is explicit and captures the selected source form. It does not prove
that an operation succeeded, and replay runs against current state. There is
no hidden time machine behind the expression.


<a id="keep-the-inputs-of-a-failed-attempt"></a>

### Keep the inputs of a failed attempt

The `:vrsjmp` service uses code as data to prepare a retry. An action such as
`(save_page (active_tab))` initially contains a query. The shared execution
code evaluates that argument and uses ordinary list operations to construct
a call containing the result:

```
(save_page '(:title "VRS" :url "https://github.com/leoshimo/vrs"))
```

The captured page is now part of the program. Retry sends this expression back
to `eval`, using that page even if the active tab has changed. The action itself
is still an ordinary call to `save_page`. If the service implementation needed
fixing, the named function can resolve through its existing binding to the replacement.
The same expression is also a useful case to inspect in the editor.

The interface for Retry and Dismiss uses ordinary items and actions. Neither
the desktop nor the terminal client needs another error-recovery protocol.
This behavior took shared code to implement; the common representation made
it applicable across service calls.

It is still a manual retry of the whole call. Only the outer arguments are
captured; queries inside the function run again. Earlier effects can be
repeated, and the record is lost when the service restarts. The [manual](manual.md#failed-actions) gives
the precise limits. Durable execution is a different problem.


<a id="let-the-clients-learn-what-is-available"></a>

## Let the clients learn what is available

A *hypermedia* interface describes available interactions as part of the data
it returns. In VRS, a page names the function that supplies its items, and the
items include executable actions. The client can ask what is available rather
than having every operation built into it.

The desktop app `vrsjmp` and the [terminal script](../scripts/vrsjmp-terminal) are two clients of the
`:vrsjmp` service. One renders graphical entries; the other presents a terminal
list. They send selections back to the service for execution. The todo example
adds search and completion to both by changing the service's Lyric code.

This is the benefit of a general-purpose client: it doesn't need to know what
a todo is. The same client can present operations on notes, windows, or a new
service. New actions use an existing interaction vocabulary. A new kind of
widget still needs client support.

Recording, source editing, general-purpose clients, and retry are connected
consequences of including ordinary executable code directly in the markup.
An interaction can become source to edit or data another program transforms.
The clients present the resulting actions through the markup they already
understand.

The inspiration from hypermedia is useful on its own terms. A program supplies
its available behavior; the client presents it. There is no need to turn the
whole explanation into an analogy with web pages.


<a id="the-same-declarations-work-in-both-directions"></a>

### The same declarations work in both directions

Some interactions can be obtained from the functions themselves. A function
declares the type of value it accepts for interactive use:

```vrs
(defn! copy_title (note)
  "Copy note title"
  (interactive :note)
  (exec "pbcopy" :stdin (get note :title)))

(entity_functions '(:note :title "A small idea"))
# => (copy_title) when it is the only matching function in scope.
```

Given a note, a program can discover `copy_title`. Given a function instead,
it can inspect the argument types and ask their registered providers for
current values. That is how a command can prompt for a window or a todo without
a separate implementation of the prompt for every function.

The [Embark](https://github.com/oantolin/embark) and Emacs `interactive` influences are quite direct. VRS makes
these relationships available through runtime queries. The editor uses them
to construct calls in source; the shared user interface service uses them to
construct prompts and action menus. Functions remain functions, callable
without either user interface.

Entities are tagged values. They do not need a central object store, and the
metadata guides interaction rather than enforcing runtime types. A database
row, a service result, or decoded shell output can supply the value.


<a id="new-interfaces-can-be-generated-too"></a>

### New interfaces can be generated too

`interfacegen` explores a consequence of using ordinary data and expressions
for interfaces. Asked for a five-minute timer button, it can produce:

```
((:title "5 minutes"
  :on_click (begin (sleep 300)
                   (notify "Timer finished" "Five minutes are up"))))
```

The [example](../scripts/vrsjmp_interfacegen_demo.ll) executes button actions in background processes. The generated
interface can use existing clients because it describes their usual items and
actions. Adding a service gives the generator more functions to compose; it
doesn't require deploying a new frontend for each generated application.

The generated code is still available to inspect and edit. Generating it and
executing it are separate operations.


<a id="work-inside-the-running-computer"></a>

## Work inside the running computer

The editor is another client. It can call functions, inspect actual results,
and retain those results in source. With the browser service bound:

```vrs
(active_tab) # C-u C-c C-e retains its result as source.
# For example, the expression becomes:
# '(:title "VRS" :url "https://github.com/leoshimo/vrs")
```

An observed value can become a fixture or a starting point for another program.
A function that offers a choice can supply an argument. A function that
constructs code can supply an edit. These operations use the same environment
as the applications being developed.

This is more than a shorter edit-run cycle. The computer can answer questions
while the program is being constructed. I don't have to simulate every
intermediate result in my head before trying the next expression.


<a id="why-the-language-belongs-to-the-environment"></a>

### Why the language belongs to the environment

Owning a Lisp makes it possible to shape the programming model around these
interactions. The language already has code as data. Its functions also have
metadata slots for documentation and interactive argument information. The
same values can be used by the evaluator, service bindings, editor, and clients.

Other languages can support annotations and registries. The reason to own
Lyric is the freedom to extend the common model rather than repeatedly building
a smaller programming language around another one. Macros can construct code;
native bindings can expose host capabilities; execution can yield to runtime
operations. These parts can be designed together.

Repeated conventions can become part of the language instead of leaving the
programmer to act as a human compiler. This is the appeal of
[bottom-up programming](https://www.paulgraham.com/hundred.html): teach the environment how to express the work,
then use that vocabulary to build more of it.

Maintaining a language is a real cost. Lyric doesn't inherit all the libraries
and machinery of a mature ecosystem. Practical composition lets external
programs do much of that work while the common language stays focused on the
environment. See [Lyric](manual.md#lyric) for the language itself.


<a id="inspection-is-part-of-the-environment-too"></a>

### Inspection is part of the environment too

The debugger extends the access already used by the REPL and editor. `dbg!`
records calls, arguments, results, and source locations while returning the
ordinary result:

```vrs
(dbg! (map '(2 3) (fn (x) (+ x x))))
(get (dbg_history) :records)
```

The terminal and web viewers use the same recorded values that another program
can query. Observations can stay beside the source as expressions worth running
again. This history is bounded and in memory; it is not stepping or time-travel
debugging. Code in another process needs its own observation block.

Reevaluating service startup replaces a running service. Compatible bindings
follow the new instance, and existing clients can present new actions in the
format they already understand. The service's state and old stack aren't
automatically migrated. Redefining an export in a separate REPL alone is not
the same operation; the [editor chapter](manual.md#editor) makes that distinction explicit.


<a id="ask-for-real-data-including-in-prompts"></a>

### Ask for real data, including in prompts

The same access is useful when constructing a prompt. A program can fetch the
active page, current tasks, or calendar entries and put those values into its
request. For example, with the relevant functions available:

```vrs
(def tab (active_tab))
(def proposed
  (codegen
    (format "Write a notification reminding me to revisit this page: {}"
            (pretty tab))))
(pretty proposed)
```

The computer supplies the page instead of requiring someone to describe it
from memory. [Natural-language commands](../scripts/nl_shell.ll) and [calendar scheduling](../scripts/nl_scheduler.ll) explore this
pattern. Agents can use the same function discovery, expressions, and process
primitives as other programs. The architecture doesn't need a second model
of every application for an agent to work with it.


<a id="a-program-can-wait"></a>

## A program can wait

A process runs a sequence of Lyric expressions with an environment and mailbox.
It can suspend while waiting for a message, I/O, or a timer. A service is a
process handling requests; a recurring job can be a process that sleeps:

```vrs
(spawn (fn ()
  (loop
    (publish :index_result (exec "feedbinctl" "index"))
    (sleep 900))))
```

The indexer can use other services and publish results to subscribers. It can
be inspected and stopped through ordinary runtime functions. It needs neither
a separate scheduling language nor an operating-system thread reserved for
the time it spends asleep.

This is the appeal of lightweight processes and message passing from Erlang:
concurrent work can still be written as sequences. A service can also receive
pubsub events in its message loop. Completing a todo can trigger a separate
program, such as the unicorn animation in the tour, without adding that behavior
to the todo service itself.

Current boundaries matter. CPU-bound code is not preempted; closures can share
mutable bindings across processes. A sleeping worker doesn't answer requests
at the same time, so responsive controls belong in a separate process. Waiting
retains memory, and a runtime restart loses it. Pubsub is node-local; an event
handler can still call a service on another device.


<a id="the-rest-of-the-devices-are-part-of-the-environment"></a>

## The rest of the devices are part of the environment

In the tour, the todo service moves to a home server. It still asks the laptop's
Safari service for the current page. The user interface keeps making the same
todo calls. Those relationships survive the change in where the service runs.

A file can be sent to a connected node for execution. A `remote!` block can
define and start a service there. The registry makes its functions discoverable,
and existing bindings follow a replacement service. Remote development uses
the same programming environment as local development.

The [Ditoo service](../scripts/ditoo.ll) makes the consequence tangible: only the host attached to
the Bluetooth display needs access to the hardware. A build script on another
machine can call its display functions. Another program can connect them to an
event without implementing Bluetooth again.

Plan 9 is an important influence here. I want the collection of devices to
feel like one computer I can program. Network failures, dependencies, and
service lifetimes remain real. Their [contracts](manual.md#connected-devices) belong in the environment too.


<a id="keep-the-whole-thing-within-reach"></a>

## Keep the whole thing within reach

The recurring qualities are practical, direct, and interactive. Services use
whatever works. Expressions stay close to the operations they describe. The
editor, debugger, and applications participate in a running system that can
answer questions and accept changes.

The useful test is whether the next piece can participate. Can another client
use the capability? Can an interaction become a program? Can a failure produce
an input worth retaining? The second consumer reveals more about the design
than another list of primitives.

A personal software ecosystem can grow one small program at a time. The parts
can be useful, occasionally scrappy, and still available to change. Building
the larger environment should retain the practicality—and the pleasure—of
hacking on a small program.


<a id="influences-and-further-directions"></a>

## Influences and further directions

Unix, Acme, and Plan 9 inform how independent programs participate in an
environment. Emacs informs the continuity between use, inspection, and editing.
Erlang informs concurrent execution. Hypermedia, Embark, and Hyperbole inform
how data can describe interactions. The [linked reading list](design.md#further-reading) gives the sources
and additional influences without making them prerequisites for this essay.

Richer documents, durable work, a relationship store, preemption, and deeper
execution control are proposals in [TODO](../TODO.org). The working entity system doesn't
depend on those proposals being completed.
