# AI and VRS

Notes on AI in VRS belong here. This document collects existing experiments and
future directions; [Core Concepts](concepts.md) covers using the runtime.

## The same environment

An agent can work through the same functions as a person using Emacs or a Lyric
program. It can discover services, bind their functions, inspect arguments and
docstrings, query live state, and compose calls into a program. The service
adapter is shared: exposing a function need not also require writing a separate
agent tool for it.

For example, `(help active_tab)` describes the function, `(active_tab)` retrieves
the current browser page, and `(get (active_tab) :url)` extracts its URL. These
are the same expressions a person can evaluate and keep in their source file.
An agent with runtime access can use them too.

The interesting step beyond calling functions is participating in the running
program: start a process, wait for a message, inspect a result, or replace a
service implementation. Redefinition has a scope: editing the agent's or editor's
own session does not change a separate service. Replacing a named service lets
future calls reach its new implementation; it does not migrate its old state.

## Existing experiments

- [Natural-language shell](../scripts/nl_shell.ll): builds a prompt from `help`
  for a selected set of functions, reads the model's reply as Lyric, and can
  evaluate it in a spawned process. It publishes the generated form on `:code`.
- [Scheduling](../scripts/nl_scheduler.ll): adds actual calendar events and
  handwritten preferences to a request for a program that schedules the day.
- [Interface generation](../scripts/interfacegen.ll): asks for a list of titles
  and action expressions. The [GUI demo](../scripts/vrsjmp_interfacegen_demo.ll)
  turns those into a timer menu whose buttons run Lyric code.

These scripts demonstrate individual pieces. They do not yet form an agent
that discovers the whole environment, repairs itself, and asks for input.

## Ask a question by making an interface

A program can encounter a decision it cannot make and wait for a value. The
current `request_input` mechanism already connects a waiting process to a
vrsjmp page. The proposed value chooser makes that useful for arbitrary lists
and records; see the [chooser design](../DESIGN.org).

Consider an agent filing captured pages. Two projects both seem plausible for
one article. It could present the article title and those two projects, with
actions returning the corresponding project values. The answer becomes the
result of the waiting expression, and the program continues.

The page can be specific to the decision: actual candidate projects, relevant
excerpts, or a proposed change. An agent can construct that page as data using
ordinary code. For the current list GUI, new kinds of behavior still need
callbacks available in the vrsjmp service; an arbitrary local callback is not
automatically installed there. A generic chooser would cover many questions
without adding a callback for each one.

## Change the rule while the operation waits

The [running-program demo](../DESIGN.org) files a batch of captures. When its
policy cannot choose a destination, it waits. The user can answer once, or edit
and reload the policy service, then tell the worker to retry the decision.
Completed items stay completed; the waiting worker continues with the new rule.

An agent could help by inspecting the troublesome record, proposing a policy
change in Emacs, and trying it against sample captures. The user can inspect
the same values and calls. Accepting the change improves the program for later
items as well as resolving the current question.

This uses an explicit pause in ordinary code. General recovery from a failed
call, interruption handlers, and editing suspended stack frames remain separate
runtime work. A process waiting in memory is also different from a durable job
that can survive a daemon restart.

## Assemble the debugging tool from the live system

Start with a small observation that already works. In an editor session:

```lyric
(subscribe :cmd)
(recv)
```

Run an action in vrsjmp. The waiting expression receives a publication containing
the action form. Inspect that form, change its arguments, and keep a useful
variation in source. This observes commands vrsjmp publishes, not every function
call in the runtime.

For the capture worker, add an application topic containing each item and its
proposed destination. A second process can collect those values. Combine that
collection with the proposed chooser to inspect an item, compare two policy
functions on it, and retain the example as a test case. The observation process,
comparison functions, and UI can be written as ordinary Lyric for this problem.

An agent could assemble the same temporary instrument when investigating a bug.
The person debugging can inspect or alter it because it uses the same runtime
primitives. TracePoint-style hooks would extend this to calls that do not already
publish events; they are tracked in [TODO.org](../TODO.org).

## Keep the generator with the generated source

The `generated!` / `generate!` idea is a proposed source block containing both
the expression that produced some code and the accepted code itself. Neither
name nor syntax is settled, and neither macro exists yet.

For example, retain an `interfacegen` request for a timer menu beside the actual
menu definition. Reading the file shows both the intention and the functions
the buttons will call. Regeneration is an explicit editing action: show the
diff, accept it, then evaluate the accepted definition normally. Loading the
file should not silently call a model again.

The generator could also be a visual editor or a browser field selector. AI is
one possible producer of source. Open questions include how manual edits are
preserved, what live inputs to retain for regeneration, and how to indicate
that generated code no longer matches its generator.

## Further directions

- A GUI client exposing native UI functions as a service would let programs
  and agents address that client through ordinary calls. This needs a bridge
  for incoming requests, client lifetime, and calls that overlap an input wait.
- Service-driven progress and partial results would let a long-running program
  keep its question or results page useful while work continues.
- Source locations on definitions would let an agent or function browser find
  the same file and line that a person edits in Emacs.

These are shared runtime and tooling projects, tracked in [TODO.org](../TODO.org).
