# Remote execution

`remote!` sends a block of source to a named connected VRS node. It runs in a
fresh process there and returns its value. The macro is part of the standard
library installed in every process:

```lisp
(remote! "minato"
  (node_name))
# => "minato"

# The ordinary function accepts code as data:
(eval_remote "minato" '(node_name))

# Include a caller-local value explicitly:
(def count 40)
(eval_remote "minato" `(+ ,count 2))
# => 42
```

The macro expands to `(eval_remote NODE '(begin FORMS...))`. Its node expression
runs once in the caller. The body, including its macro calls, runs on the target.
Locally defined functions and macros are not implicitly copied. A target equal
to the caller's own node name still creates a fresh process.

## Setup

Both nodes must run compatible builds. The node protocol is version 2; older
peers are rejected at the handshake. Existing configured `ssh://` or `tcp://`
links carry remote evaluation as well as service messages. `--node` names a VRS
node, not an SSH endpoint.

For an isolated demonstration, build `vrsd` and `vrsctl` on each machine:

```sh
cargo build -p vrsd -p vrsctl
```

In a terminal on Minato, from its checkout:

```sh
./target/debug/vrsd --node minato --node-port 28774 --socket /tmp/minato-demo.socket
```

In a terminal on Shibuya, from its checkout:

```sh
./target/debug/vrsd --node shibuya --node-port 28773 --socket /tmp/shibuya-demo.socket
```

In another Shibuya terminal:

```sh
./target/debug/vrsctl --socket /tmp/shibuya-demo.socket \
  -c "(configure :nodes '(\"ssh://minato:28774\"))"

./target/debug/vrsctl --socket /tmp/shibuya-demo.socket \
  -c '(remote! "minato" (node_name))'
```

Configuration connects asynchronously. If the read-only `node_name` probe says
the node is not connected yet, repeat the probe after connection establishment.
No remote evaluation is automatically replayed. These daemons use separate
sockets and explicit ports so the demonstration can run alongside normal VRS.

## Edit here, run there, call from another client

The repository includes `scripts/demo-remote.ll`. Its entire service is:

```lisp
(defn! host_probe ()
  (list :node (node_name) :version 1))

(spawn_srv! :host_probe :interface '(host_probe))
```

Send Shibuya's file to Minato:

```sh
./target/debug/vrsctl --socket /tmp/shibuya-demo.socket \
  --node minato ./scripts/demo-remote.ll
```

Open another client of Shibuya's daemon and leave it open:

```sh
./target/debug/vrsctl --socket /tmp/shibuya-demo.socket
```

```lisp
(bind_srv :host_probe)
(host_probe)
# => (:node "minato" :version 1)
```

Edit the local file to return `:version 2`, then resend it with the same shell
command. In the client that already called `bind_srv`:

```lisp
(host_probe)
# => (:node "minato" :version 2)
```

`spawn_srv!` replaces the old service process. Existing stubs resolve the service
name on each call, so they follow the replacement. In-memory state is discarded;
replacement is not transactional or guaranteed to be interruption-free. Rebind
if the exported interface or argument metadata changes.

An inline deployment uses the same mechanism:

```lisp
(def deployed
  (remote! "minato"
    (defn! ping () :pong)
    (spawn_srv! :remote_ping :interface '(ping))))

(bind_srv :remote_ping)
(ping)
# => :pong
(call deployed '(:ping))
# => :pong
```

## File and editor locality

`vrsctl --node minato FILE` reads FILE on the client machine and sends its source.
The file need not exist on Minato. The client preserves the original file name
and line/column provenance, including for `dbg!` observations.

Within Lyric, `read_script` reads a file on the node executing that expression
and returns a `begin` form without evaluating it:

```lisp
# Read on the caller's node and send the resulting code:
(eval_remote "minato" (read_script "./scripts/demo-remote.ll"))

# Read and execute a file that already exists on Minato:
(remote! "minato" (run "./scripts/demo-remote.ll"))
```

File reads and `exec` inside the submitted code use the destination's filesystem,
working directory, tools, and environment. Sending a script does not synchronize
other files it references or persist the script on the destination. Update the
deployed checkout/init separately if a change should survive a daemon restart.

Remote editor sessions preserve definitions and macros between requests:

```sh
./target/debug/vrsctl --socket /tmp/shibuya-demo.socket --node minato --session
```

Each stdin line is a JSON request, with the same formatting and optional source
origin fields as local sessions:

```json
{"source":"(def x 41)","file":"/local/editor-buffer.ll","line":1,"column":1}
{"source":"(+ x 1)"}
```

In Emacs, a buffer-local client command selects the target while retaining the
normal expression/region/buffer evaluation keys:

```elisp
(setq-local vrs-vrsctl-command
            "vrsctl --socket /tmp/shibuya-demo.socket --node minato")
```

The client protocol is forwarded intact, including subscriptions and source
requests. `vrsctl --node minato dbg` can inspect that node's debug history.
Closing or cancelling the client closes its remote evaluator. Services spawned
by the evaluator remain running. Disconnects close remote sessions; reconnecting
starts a new session, without restoring definitions or replaying commands.

## Registration visibility and waits

Both `eval_remote` results and remote client responses carry a versioned snapshot
of the destination's own service registry, taken after evaluation. The initiating
node applies and acknowledges that snapshot before delivering the response.
Snapshots preserve unchanged entries and ignore older announcements, so a
refresh does not republish every service or resurrect a replaced registration.
Revision state is reset when a node link is replaced.

`spawn_srv!` already waits for its child's local registration before returning.
That registration is therefore included in the completion checkpoint even
though the child has a separate process environment. A second client of the
initiating daemon can immediately bind the service. Unrelated third nodes still
learn registrations asynchronously. A crash or concurrent replacement can still
change availability after the checkpoint; ordinary service-name selection applies.

For independent startup dependencies, or an ordinary child that registers later:

```lisp
(wait_srv :feedbin)
(wait_srv :remote_ping :pid deployed)
(wait_srv :remote_ping :pid deployed :timeout 5)
```

The wait returns the selected PID. An expected PID prevents an older instance
from satisfying the wait. Without `:timeout`, the wait lasts until matched or
cancelled; timeout values are nonnegative integer seconds. `:timeout 0` performs
an immediate check. The wait observes registration, not health, and does not pin
name-based bindings to a PID. A direct `(call PID MESSAGE)` addresses that instance.

Ordinary `spawn` does not wait for a child's future work. If that child registers
after the remote evaluation finishes, the completion checkpoint cannot include
it. Use a readiness message or an explicit wait in that case.

`eval_remote` transfers data values and process IDs, not closures or executable
bytecode. Nontransferable results produce an error after any preceding effects.
A disconnect while waiting produces an outcome-unknown error; code may already
have run. Cancellation is best effort and does not roll back effects or kill
services already spawned. Remote sessions have bounded message queues; a stalled
consumer is disconnected rather than blocking all traffic on the node link.

## Validation

Automated tests cover isolation and quotation, real PID returns, immediate
cross-client binding after a fork, replacement through existing stubs, exact-PID
waits, stale snapshots and unchanged-name selection, persistent source-aware
remote sessions and subscriptions, cancellation cleanup, CLI file/editor input,
and disconnect/restart behavior.

An isolated live test on Shibuya and Minato also verified local-only file
deployment, versions 1 and 2 through one existing binding, PID waiting, nested
remote evaluation, remote `uname`, retained editor definitions, and the local
file's source identity in Minato's `dbg!` history. The test daemons were stopped
after verification; the normal running services were not redeployed.
