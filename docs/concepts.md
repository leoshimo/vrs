# Core Concepts

## Lyric Forms

Lyric uses the same representation for code and structured data. A form can be
constructed, stored, sent to another process, or evaluated later.

```lyric
(def command '(+ 40 2))
(eval command) # => 42
```

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
cargo run --bin vrsd -- --init ./init.ll
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
