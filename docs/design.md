# VRS: Design decisions

> **Reorganization WIP**

VRS is my experiment in building a personal software environment. I want the
practicality of shell scripting, the feeling of working inside Emacs, and access
to the programs and devices I actually use. The unit of design is that whole
experience: the language, the runtime, the user interfaces, and the means of
changing them.

The central choice is to make *symbolic expressions* the common representation
for programs, data, and user interface actions. A service makes functions
available to the environment. Clients evaluate expressions and present results;
an action in a graphical application can be the expression a script would run.
Services can call one another across devices. The editor and debugger work
against this same running system.

A few consequences are worth keeping in mind:

-   Use an application, inspect its action, and keep it as a program.
-   Add a capability once; use it from scripts, other services, and existing clients.
-   Let function metadata supply interactions for unfamiliar data.
-   Change the software while it is running, including on another machine.

The [tour](tour.md) demonstrates these ideas incrementally. This document explains the
choices behind them. An [alternative essay](design-experience.md) approaches the same design through
the experience of building with it. Exact behavior belongs in the [manual](manual.md);
unfinished proposals live in [TODO](../TODO.org).


<a id="background-and-motivation"></a>

## Background and motivation

The goal is to create a computing environment that brings me joy. I live on a
collection of personal software running on VRS, evolving the runtime as I go.
Some programs are useful once. Others become part of how I use my computer.
Both should be easy to inspect, combine, and change.

I want to compose the software I already have and write the parts that are
missing. Lyric provides the glue, bringing existing programs into a shared
environment where their capabilities can be combined, scripted, and used
across devices.

Ordinary software can do the things in the tour. The interesting question is
what it takes to build them. An application adapter, a graphical frontend, a
background job, and a remote deployment often bring their own conventions.
Connecting them can become a project larger than the thing I wanted to make.

For plenty of personal software, I want clay and duct tape. I don't want to
CNC-mill a bracket every time two useful pieces need connecting. Gary
Bernhardt's [Unix Chainsaw](https://www.youtube.com/watch?v=sCZJblyT_XM) captures something I like about this approach:
half-assed, but the right half of the ass. The point is choosing where effort
buys something, and where an existing program already does the job.

The tour's “whatever gets the job done” points to a culture of hacking and
interoperability: making existing software serve purposes its authors did
not anticipate. A command-line tool, local database, or UI automation may
provide the connection when an application offers no suitable API. There is
a connection to adversarial interoperability here—especially when a vendor
actively resists that connection—though not every improvised integration is
adversarial. The goal is to keep the software open to its user's intentions,
without making that user wait for a vendor to build the integration.

VRS deliberately takes on more scope than a library. The language, execution
model, service interfaces, clients, and editing experience are designed
together. A local convenience is less interesting than a convention that
removes work from several of these places at once.

That is also the connection between simplicity and agency. Fewer conventions
mean fewer things standing between a useful program and a change to it. I want
the software I use to remain mine to work on. This is a design goal for personal
computing, not a claim that everyone should build their own applications.


<a id="what-comes-after-the-unix-pipe"></a>

## What comes after the Unix pipe?

Unix makes independently written programs useful together through a small set
of conventions. VRS asks what a similarly practical medium could look like
when the pieces include application data, graphical interactions, long-running
programs, and several devices.

The pipe connects streams. In VRS, the common material is values and expressions
that operate on them. Consider a call from the tour:

```vrs
(add_todo "Buy coffee" "")
```

The same expression can be included in an interface item:

```vrs
'(:title "Add todo: Buy coffee"
  :on_click (add_todo "Buy coffee" ""))
```

The application uses code. The action is written as an ordinary function call,
included directly in the markup. A user can extract that call and use it in a
program. Another program can inspect it, collect it with other calls, or replace
an argument expression with the value it returned.

This is the practical point of *code as data* and *data as code*. The call can be
manipulated with ordinary list operations, and the resulting expression can go
back to the same evaluator. The function that performs the action is written
normally; recording and retry work on the expression that calls it. There is no
second description of the operation to translate back into a program.

Action schemas, app-intent declarations, and miniature automation languages
can become weak approximations of programming languages. I don't want to
describe a call in one representation only to reconstruct it in another.
Lyric already expresses the operation, its arguments, and how it composes
with other calls. User interfaces, programs, and the editor can work with
that same expression. Clients still need agreed markup fields and service
functions; the action itself needs no second encoding.

This is the useful meaning of *code mode* here. Service operations are
functions. Client actions are expressions. Discovery makes functions available
in a namespace. A CLI or database still needs an adapter, but the rest of the
environment works with the adapter's ordinary functions and values.

Uniformity has several dimensions: representation, execution, communication,
and interaction. It is not a demand that every application look alike. A
terminal list and a graphical application can present the same action quite
differently. They agree on what it means to invoke it.


<a id="why-write-our-own-lisp"></a>

### Why write our own Lisp?

Lisp provides a small, manipulable representation of programs. Lists and
quotation are enough to construct an action, inspect it, combine several
actions, and pass the result to the evaluator. Code generation uses the same
operations as ordinary data construction.

Owning the language also lets the needs of the environment become language
features. Lyric's callable values have metadata slots. A function can describe
its arguments, documentation, and interactive use in a form the service system
and clients can inspect. It remains a function, callable by another program.

In another language, some of this could be implemented with annotations,
wrappers, or registries. The reason to own Lyric is the freedom to change the
common model when those conventions start becoming a second, less capable
programming language. The shell is similarly shaped around the work it makes
easy. I want that freedom across more of the environment.

Too much programming feels like acting as a human compiler for the same
institutionalized patterns. If a convention keeps recurring, I want to teach
the environment to express it. Paul Graham's account of
[bottom-up programming](https://www.paulgraham.com/hundred.html) captures the connection: a vocabulary built for one
program becomes a language for writing others.

The cost is maintaining a language and its implementation. Lyric is small and
still changing; it doesn't inherit a mature language's entire ecosystem. It
can instead compose programs written in other languages and expose native
functions from Rust. The [language chapter](manual.md#lyric) covers its actual semantics.


<a id="what-everything-is-s-expr-means"></a>

### What “everything is S-expr” means

Source code, interface descriptions, recorded actions, and ordinary structured
data share symbolic expressions. This keeps those things inspectable and
editable by the same programs. Functions themselves execute as bytecode;
process IDs, references, and other runtime-only values are not all source that
can be printed and read back. The shared representation is a useful design
constraint, not a claim that every implementation detail is a list.


<a id="services-make-composition-practical"></a>

## Services make composition practical

A *service* is a running process with a name and exported functions. It can
implement behavior in Lyric, run another program, query a local database, or
automate an application's interface. The rest of VRS calls its functions.

The [browser service](../scripts/os_browser.ll) uses AppleScript for Safari's active page and SQLite
for tabs synced from other devices. These are different ways into the same
application. Both become ordinary values returned by functions. The todo
example combines the page title and URL with its own data without a Safari
extension or a separate browser integration in each client.

Other services make different choices. [Feedbin](../scripts/feedbin.ll) uses a dedicated CLI.
[Apple Notes](../scripts/os_notes.ll) combines database reads, application URLs, and AppleScript.
[Ditoo](../scripts/ditoo.ll) runs a Bluetooth display through another small program. Lyric provides
the glue; the programs it composes can be written in whatever language suits
the work.

This is where “integrate once” has a concrete meaning. The browser adapter is
implemented in one service. A script, a todo service, and a user interface can
use it without each learning how Safari stores or exposes its tabs. New
consumers still need their own application logic, but they can reuse the access.

For M capability providers and N clients, the aim is M adapters and N clients
rather than M×N bespoke connections. This describes the integration work the
design tries to share. Parsing, authentication, domain behavior, and rendering
don't disappear. Make glue first class; don't pretend glue has ceased to exist.


<a id="the-same-interface-across-devices"></a>

### The same interface across devices

Services belong near their data or hardware. Their names and function calls
remain useful elsewhere. In the tour's example, the same todo service can run
on a home server while programs on other devices keep calling `add_todo` and
`get_todos`. Changing where it runs does not require a new interface for its
callers.

Remote evaluation makes developing the service part of the same environment.
A local file can be sent to another node, or a `remote!` expression can define
and start a service there. Discovery and binding then make its functions
available to other programs.

This recalls what I like about [Plan 9](https://9p.io/sys/doc/): the devices should feel
like parts of one computer. VRS uses functions and messages for that connection.
Remote calls can still fail, dependencies still belong somewhere, and replacing
a service does not preserve its in-memory state automatically. Those are real
properties to expose, not details a common syntax can erase.


<a id="processes-make-waiting-ordinary"></a>

## Processes make waiting ordinary

A *process* runs a Lyric program with an environment and a mailbox. A service
is one use of a process. So is a recurring indexer:

```vrs
(spawn (fn ()
  (loop
    (publish :index_result (exec "feedbinctl" "index"))
    (sleep 900))))
```

The code describes what happens in order. It can call a service, wait for a
message, run an external command, or sleep without handing the rest of its
control flow to a collection of callbacks. Lightweight execution and message
passing are the parts of [Erlang](https://www.erlang.org/doc/system/conc_prog.html) I want to bring into this environment.

A waiting fiber does not reserve an operating-system thread. Its state remains
in memory until it can continue. CPU-bound Lyric code is currently not
preempted. Processes copy their bindings when spawned, but closures can retain
shared mutable environments; this is not complete memory isolation.

Pubsub delivers publications to subscribers' mailboxes. Services can handle
those events alongside their function calls in one message loop. A todo
service publishes that a task was completed; a separate service can respond
by making a unicorn leap or updating a display. The publisher need not know
which reactions have been added.

The same runtime can observe and control the indexer. There is no separate job
language: it is a process with calls, events, and ordinary state. A sleeping
worker does not also answer requests; a separate service can expose controls
when the worker must remain busy or asleep. Waiting is cheap, not durable:
runtime termination loses that state. Pubsub reaches subscribers on connected
nodes; a subscriber can also call a remote service.


<a id="hypermedia-keeps-applications-out-of-clients"></a>

## Hypermedia keeps applications out of clients

VRS's user interface markup describes what can be displayed and what can be
done. A page names the function that supplies its items. Each item can include
an executable action and further actions on the same value. In this sense the
interface is *self-describing*: a client can obtain the available operations
from the running program.

Putting application logic on a server doesn't give a client this property by
itself. A client can delegate every operation and still have a hard-coded list
of operations. Hypermedia puts the available interactions in the representation.
A web browser understands links and forms; the page supplies their destinations
and actions. [Fielding describes this as presenting information and controls
together](https://roy.gbiv.com/untangled/2008/rest-apis-must-be-hypertext-driven),
so a person or program can choose what to do next.

The `:vrsjmp` service supplies such interfaces. The desktop app `vrsjmp` and
the terminal client understand the shared markup and the `root_page`,
`get_items`, and `on_click` service functions. A todo item supplies both its
presentation and an action such as `(complete_todo '(:todo ...))`. Neither
client needs a built-in `complete_todo` operation. They present the items and
send selections back to the service to handle.

In the tour, adding search and completion to the service changes both user
interfaces. The desktop binary doesn't change; the shell script doesn't need
another branch for todos. This is the part of hypermedia that interests me:
the running application supplies its available behavior to general-purpose
clients.

VRS represents the action as an ordinary executable Lyric expression. The
client doesn't need to interpret its application logic; the service evaluates
it. That choice connects the user interface to the rest of the environment:
the same expression is code to run, data to inspect or transform, and source
to insert into a program. Recording, replay, source editing, and retry can all
work with it.

General-purpose doesn't mean logicless. These clients implement a particular
interaction vocabulary, including presentation, selection, and navigation.
New actions within that vocabulary work with existing clients; a new kind of
widget still needs client support. A voice client or richer document editor
would need its own presentation and input support. Uniform access leaves room
for different interfaces.


<a id="an-interaction-is-a-program-example"></a>

### An interaction is a program example

The selected action is already source. The shared command path can publish it
through pubsub, and another program can retain it:

```vrs
# With command publication enabled by the application.
(subscribe :cmd)
(def action (get (recv '(:topic_updated :cmd _)) 2))
(pretty action)
```

An action such as `(add_todo "Buy coffee" "")` can be pasted into a program,
edited, or combined with other recorded expressions in a `begin` form. That is
how the [command-macro service](../scripts/cmd_macro.ll) builds a recording.

Using an application becomes a way to explore how to program it. This is an
Emacs feeling: a familiar interaction has a discoverable command behind it.
In VRS, the resulting expression can move between clients and programs without
being translated into a separate automation language.

Publication is deliberate; not every function call is logged. A recording
records the selected expression, not proof that every effect succeeded.
Replaying evaluates it against the current environment.


<a id="captured-calls-and-failure-handling"></a>

### Captured calls and failure handling

Argument capture is a small program transforming another program. The shared
`:vrsjmp` service recognizes direct command calls, evaluates their outer
arguments, and constructs a new call with those values filled in. For an action
of this form:

```
(save_page (active_tab))
```

the captured call can look like:

```
(save_page '(:title "VRS" :url "https://github.com/leoshimo/vrs"))
```

Retry passes the captured expression to `eval`. It uses the original page,
even after the active tab changes. The function name still resolves through its
binding, so a corrected service can be called with the retained inputs. The
expression also gives the programmer a concrete case to investigate.

Retry and Dismiss use the existing item format and action expressions. Recovery
needed shared execution logic, but no special recovery protocol in each client
or bespoke UI for each service. The [manual](manual.md#failed-actions) describes its limits: this is a
manual retry of the whole captured call, not a checkpoint or a rollback.

This representation is powerful for both users and programs. Other programs
can [record and replay](../scripts/cmd_macro.ll) user interactions, or preserve
a failed action as code—ready to retry with its original inputs. The editor
can use those same expressions and live values to edit programs. A sequence
of user interactions can become a program—and data for other programs to
work with.

Recording, source editing, general-purpose clients, and retry are connected
consequences of including ordinary executable code directly in the markup.
A person can use an interaction as source; a program can inspect, collect,
or transform the same expression, or construct an action for a user interface.
The action's function is written normally. Shared code handles recording and
retry, and clients present the resulting actions through the markup they
already understand.


<a id="generating-an-interface-is-generating-data-and-code"></a>

### Generating an interface is generating data and code

An interface generator has the same output format as a hand-written page. For
example, `interfacegen` can produce a timer action:

```vrs
(interfacegen "A five-minute timer button")
```

One possible result is:

```
((:title "5 minutes"
  :on_click (begin (sleep 300)
                   (notify "Timer finished" "Five minutes are up"))))
```

The [timer example](../scripts/vrsjmp_interfacegen_demo.ll) runs button actions in background processes. The generator
doesn't have to build and deploy a new frontend. It describes an interaction
the existing clients know how to present, using functions the environment
already supplies. New service capabilities become material for these generated
interfaces through the same representation.


<a id="metadata-turns-functions-into-interactions"></a>

## Metadata turns functions into interactions

A function and its arguments already describe an interaction. `complete_todo`
operates on todos; `get_todos` supplies them. A hand-written user interface
bakes in the programmer’s knowledge of that relationship. VRS makes it explicit
and programmable, so the editor and other programs can construct interactions
from it.

A *tagged entity* is an ordinary value with a leading type tag. Functions can
declare the types of arguments they accept for interactive use:

```vrs
(defn! copy_title (note)
  "Copy note title"
  (interactive :note)
  (exec "pbcopy" :stdin (get note :title)))

(interactive_functions '(:note :title "A small idea"))
# => (copy_title) when it is the only matching function in scope.
```

`interactive` makes it possible to discover which functions work with a value.
Registering entity sources supplies the other half: which functions return
current values of a given kind. Together, these let a client start with a value
and offer operations, or start with an operation and offer values for its
arguments.

The [tour](tour.md) shows this with todos. Window services use it for focusing a window
or moving it to another display; note services can offer ways to open a note.
The editor can construct a call in source, while the `:vrsjmp` service uses
the metadata to build prompts and action menus. The desktop app presents those
interactions. These programs can discover new operations and available values
as functions and entity sources are registered. The associations remain in the
runtime, available to each consumer through the same queries.

[Embark](https://github.com/oantolin/embark) and Emacs's `interactive` declaration are direct inspirations.
The important part is that a declaration on an ordinary function is available
to ordinary programs. It isn't a GUI-only annotation or a second command API.
It guides interaction; it does not enforce the argument's runtime type.

This mechanism doesn't require an object store. Tagged values can come from a
database, a service's memory, or decoded command output. Persistence, inferred
relationships, and shared views are additional questions, kept in [TODO](../TODO.org).


<a id="live-programming"></a>

## Live programming

Building happens against the running environment. The REPL, editor, and
debugger can ask it questions while the program is being written. An actual
browser result is better material than an imagined approximation of one.

In the editor, a result can become source:

```vrs
(active_tab) # Evaluate and retain the result with C-u C-c C-e.
# The expression becomes a quoted value, for example:
# '(:title "VRS" :url "https://github.com/leoshimo/vrs")
```

That value can become a fixture, a starting list, or an argument to another
expression. The editor can also call functions that offer choices or construct
code. It participates in the environment instead of merely editing files
that will eventually describe it.

The environment being developed also supplies capabilities for developing it.
An editor can query service interfaces, use `interactive_functions` to find
operations on a value, and use `entities` to retrieve actual data. These are
the same queries available to other programs; development tools can build on
the capabilities added for everyday use.


<a id="inspection-is-available-to-programs"></a>

### Inspection is available to programs

`dbg!` records execution while returning the expression's normal value:

```vrs
(dbg! (map '(2 3) (fn (x) (+ x x))))
(get (dbg_history) :records)
```

The history contains calls, arguments, results, and source locations. Terminal
and web viewers present it; another program can query it. The same principle
applies to debugging as to application interfaces: inspection should be
available through the programming environment itself.

Observations are bounded, in-memory records. Code inside another process or
remote handler needs its own observation block. Stepping, interception, and
durable replay remain proposals rather than properties of this recorder.


<a id="change-the-running-program"></a>

### Change the running program

Reevaluate a function to change it in the current environment. Reevaluating
service startup with `spawn_srv!` replaces the named service; compatible
bindings follow the replacement. Updated actions can be used by clients that
already understand their format, including across devices.

Service replacement does not migrate its old stack or preserve its state
automatically. Exported functions are retained by the service process;
redefining one in a different REPL alone doesn't replace that export. Some
helpers can share captured environments, so explicit service replacement is
the reliable operation demonstrated in the tour.

This is the exploratory style I value in Emacs and the shell: try something,
look inside, change the program around what actually happens. The scope is now
the collection of applications, services, and devices around the editor too.


<a id="ask-the-computer-including-when-constructing-a-prompt"></a>

### Ask the computer, including when constructing a prompt

Prompting can use real values instead of asking a person to transcribe them.
For example, with the browser and code-generation services available:

```vrs
(def tab (active_tab))
(def request
  (format "Write a notification that reminds me to revisit this page: {}"
          (pretty tab)))
(def proposed (codegen request))
(pretty proposed) # Inspect the generated program before evaluating it.
```

The useful property comes before the model: a program can query the environment
to assemble its inputs. Agents can discover and call the same functions, build
expressions, and wait using ordinary processes. They don't need a separate
representation of every capability.

[Natural-language commands](../scripts/nl_shell.ll) and [calendar scheduling](../scripts/nl_scheduler.ll) explore this approach.
A generated program can still be wrong, and constructing source is distinct
from executing it. The design keeps that source available for inspection and
change.

A reminder illustrates the same distinction. With `codegen` available, the
program can be inspected before being started as a background process:

```vrs
(def reminder (codegen "Remind me to stretch in ten minutes"))
(pretty reminder)
(spawn (fn () (eval reminder)))
```

The generated program can wait and notify through ordinary runtime functions.
The scheduling experiment uses `(schedule_the_day "tomorrow")` to assemble
calendar context and preferences. These are applications of the environment,
not extra execution models added for AI.


<a id="the-experience-these-decisions-are-for"></a>

## The experience these decisions are for

*Directness* is the short distance between an expression and its effect.
*Interactivity* is working against the running system. *Pragmatism* is using
whatever works underneath a shared interface. These qualities reinforce one
another: the adapter supplies real values, the language makes them usable,
and the running environment lets the next step be tried immediately.

A useful test is the second consumer. Does the next program reuse the
capability, or need another special integration? Can an interaction become
source? Can a failure provide a case to inspect? Those questions keep the design
attached to the experience it is meant to improve.

The ambition is a whole personal software environment that retains the
practicality—and the pleasure—of hacking on a small program.


<a id="further-reading"></a>

## Further reading

The influences are discussed where they affect a decision. These links are
additional routes into the same questions, rather than prerequisites:

-   [Ritchie's Unix retrospective](https://www.nokia.com/bell-labs/about/dennis-m-ritchie/retro.html) and [Pike's Acme paper](https://9p.io/sys/doc/acme/acme.html): shared conventions between programs.
-   [Plumbing](https://9p.io/sys/doc/plumb.html) and [Hyperbole](https://mail.gnu.org/archive/html/hyperbole-users/2019-01/msg00037.html): data that suggests useful interactions.
-   [Phoenix LiveView](https://github.com/phoenixframework/phoenix_live_view), [Elm](https://guide.elm-lang.org/architecture/), and [htmx](https://htmx.org): keeping behavior and presentation connected.
-   [Semantic Compression](https://caseymuratori.com/blog_0015): shaping a language around the work.
-   [Colin Woodbury on Lisp](https://www.fosskers.ca/en/blog/rounds-of-lisp) and [Stop Writing Dead Programs](https://www.youtube.com/watch?v=8Ab3ArE8W3s): working inside a running program.
-   [Bret Victor, The Future of Programming](https://www.youtube.com/watch?v=8pTEmbeENF4) and [Andy Matuschak on premature scaling](https://notes.andymatuschak.org/zKKB5ENRahwftH96H7mijiu): protecting the ability to explore.

The detailed directions for richer documents, persistence, supervision,
preemption, and execution control remain in [TODO](../TODO.org).
