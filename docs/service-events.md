# Service topic handlers

A service can receive published events alongside ordinary function calls:

```lisp
(defn! celebrate (task)
  (exec "unicornleap"))

(spawn_srv! :celebration
  :interface '()
  :topics '((:todo_completed celebrate)))
```

Another process publishes a completed task:

```lisp
(publish :todo_completed '(:id 7 :title "Write the tour" :done true))
```

`celebrate` receives exactly one argument: `(:id 7 :title "Write the tour" :done true)`.
The topic and `:topic_updated` envelope are removed by the service loop. List
payloads are passed whole, never spread into arguments or evaluated as code.

`:topics` evaluates to a list of `(TOPIC HANDLER)` pairs. Topics must be unique
keywords; handlers must name functions with exactly one argument. Define them
before starting the service. Like `:interface`, the declaration is evaluated
once when expanding the macro, and invalid declarations fail before spawning.
The generated loop calls each handler by name, using the same function lookup
as ordinary service methods. The set of topics stays fixed.
Topic handlers are not exported through `bind_srv` unless also in `:interface`.
An empty interface is useful for a service that only reacts to events.

Both `srv!` and `spawn_srv!` accept `:topics`. `srv!` uses the current process;
`spawn_srv!` starts a child. Subscriptions are established before registration
and before the readiness message, so a publication after `spawn_srv!` returns
can reach the service. All event handlers and exported functions execute in
that service's single loop and share its state. A slow handler delays other
messages; handlers are not invoked reentrantly while another handler waits.

A handler's return value is ignored. A raised or returned error is reported
in the daemon log with the service, topic, process ID, and error, and the loop
continues. Changes made before an error are not rolled back.
Unknown topics and unrelated messages are ignored. The subscription forwarding
task stops when the service process finishes, fails, or is killed, including
when the topic is idle. Reloading a service gives its replacement new
subscriptions; general RAII handles and dynamic `on_topic` remain deferred.

Pub/sub is currently node-local, best-effort delivery: earlier publications
are not replayed, and lagging subscriptions can lose events. `publish` waits
for the broker, not for handlers to finish. Events and direct service calls
travel through different paths, so a later call can arrive before an earlier
publication. Use an explicit acknowledgment when completion matters. Reloading
subscribers is not an atomic, lossless handover.

## Runnable completion example

[scripts/demo-service-events.ll](../scripts/demo-service-events.ll) creates a
fictional todo service and a separate celebration service. It does not access
personal todos. From the repository root, with a runtime built from this version:

```lisp
(run "./scripts/demo-service-events.ll")
(bind_srv :demo_todos)
(get_demo_todos)
(complete_demo_todo 1)
```

Completing the task publishes `:demo_todo_completed`; the subscriber runs
`(exec "unicornleap")`. Install [unicornleap](https://github.com/kevinliddle/unicornleap) on the daemon's
PATH first, or substitute another command. Completing an
already completed or unknown task returns `nil` and publishes nothing. Re-run
the script to reset its private demonstration state.

## Command recording

[scripts/cmd_macro.ll](../scripts/cmd_macro.ll) uses the same mechanism:

```lisp
(spawn_srv! :cmd_macro
  :interface '(get_macros clear_macros start_macro_record end_macro_record macro_is_recording)
  :topics '((:cmd record_command)))
```

`start_macro_record` initializes service state. `record_command` appends events
while recording is active and skips start/stop commands. `end_macro_record`
saves the commands handled so far. There is no separate recorder process or
manual `:get_recording` protocol. This is a recorder of delivered events, not
an acknowledged audit log: start/stop calls do not establish a delivery barrier
for publications still in transit.
