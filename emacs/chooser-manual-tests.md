# Manual tests for Emacs choosers

These checks use a separate runtime and [an in-memory demo service](chooser-demo.ll).
The fixture has two items with the same title, typed destination choices, a text
argument without a provider, and a log of executed actions. It does not call
personal apps or write application data.

## Start the test environment

From the repository root, run this in a terminal. Leave the terminal open for
cleanup afterwards:

```sh
cargo build --locked -p vrsctl -p vrsd
chooser_tmp=$(mktemp -d /tmp/vrs-chooser.XXXXXX)
./target/debug/vrsd --node chooser-test --node-port 0 \
  --socket "$chooser_tmp/runtime.sock" --init emacs/chooser-demo.ll \
  >"$chooser_tmp/daemon.log" 2>&1 &
chooser_pid=$!
export VRS_CHOOSER_COMMAND="'$PWD/target/debug/vrsctl' --socket '$chooser_tmp/runtime.sock' --bind chooser_demo"
emacs -Q -L emacs -l vrs-mode \
  --eval '(setq vrs-vrsctl-command (getenv "VRS_CHOOSER_COMMAND"))' \
  --eval '(switch-to-buffer "*VRS chooser test*")' \
  --eval '(vrs-mode)'
```

This starts a fresh Emacs using the current source. To test your configured
completion packages later, load both `emacs/vrs-mode.el` and `emacs/vrs-choose.el`
with `M-x load-file` in your usual Emacs. Set `vrs-vrsctl-command` in the test
buffer to the command printed by `printf '%s\n' "$VRS_CHOOSER_COMMAND"` in the
terminal. Use a new scratch buffer in `vrs-mode`.

For each expression below, put point immediately after its closing parenthesis.
`C-c C-e` evaluates without replacing source. `C-c C-v` chooses a value;
`C-c C-a` constructs an action call. Press `TAB` in the minibuffer to see choices
when your completion setup does not show them automatically.

## 1. Choose a value without losing its identity

Insert `(demo_items)` and press `C-c C-v`. Both rows are called **Same**, with
different numbers and source details. Choose the row whose `:id` is `2`.
The expression becomes:

```lyric
'(:demo/item :title "Same" :id 2 :note "line\n\"quoted\"" :tags (gamma delta))
```

The tag, nested symbols, string escaping, and outer quote should survive.
Evaluate the retained value with `C-c C-e` to confirm it is usable source.

## 2. Choose fields and regions

Insert `(get (demo_items) 1)` and run `M-x vrs-choose-field`.
Choose `:note`: the source should become `"line\n\"quoted\""`.
Repeat on a fresh copy and choose `:tags`: expect `'(gamma delta)`.

Then select this whole region and press `C-c C-v`:

```lyric
(def chosen_items (demo_items))
chosen_items
```

Choose either item. The whole region should be replaced by that one literal.

## 3. Browse functions and fill arguments

On a blank line, run `C-c C-b` (`M-x vrs-browse-functions`). Search for
`demo_move`; its signature, service, and **Move demo item** documentation appear.
Select it. Expect `(demo_move item place)` with placeholders.

On another blank line, run `C-u C-c C-b`. Choose `demo_move`, then the item with
`:id 2`, then **Here**. Expect a call with both complete, quoted entities.
Evaluate `(demo_log)` on a separate line: it should still be `()`.

Repeat with `demo_note`. After choosing an item, type `"hello"` for the text
argument, including quotes. The call should be inserted with `"hello"`, and the
log should still be empty. Entering a call such as `(demo_items)` as an argument
also only inserts that expression; it does not evaluate it during construction.

## 4. Start with an entity and construct an action

Insert `(get (demo_items) 1)` and press `C-c C-a`.
Choose **Move demo item**, then **Here**. Expect:

```lyric
(demo_move '(:demo/item :title "Same" :id 2 :note "line\n\"quoted\"" :tags (gamma delta))
           '(:demo/place :title "Here" :id 10))
```

`(demo_log)` should still be empty. You now have a complete call you can edit,
save, or evaluate explicitly with `C-c C-e`.

## 5. Execute explicitly and inspect the published command

Evaluate `(subscribe :cmd)` with `C-c C-e`. On a fresh
`(get (demo_items) 1)`, run `M-x vrs-execute-action`, choose **Move demo item**,
then **Here**.

The original expression stays in the buffer, `*VRS Result*` shows `:moved`, and
`(demo_log)` now contains one entry if you have not executed earlier calls.
Evaluate this to see the exact call published for macro recording:

```lyric
(recv '(:topic_updated :cmd _))
```

## 6. Cancel and recover

- Start `C-u C-c C-b`, choose a function and one argument, then press `C-g`.
  There should be no partial insertion and no new entry in `(demo_log)`.
- On `(missing_function)`, press `C-c C-v`. Expect an evaluation error and
  unchanged source. A subsequent `(demo_items)` should still work.
- On `(recv)`, press `C-c C-v`, wait briefly, then press `C-g`. The pending
  request is cancelled and its session resets. `(demo_items)` should work again:
  the command-line `--bind chooser_demo` restores the binding automatically.

## 7. Optionally make a choice in vrsjmp and retain it in Emacs

This last check needs a buffer connected to a runtime with vrsjmp running; the
isolated test runtime above intentionally has no GUI service. Use matching VRS
code for that runtime and client.

Insert `(vrsjmp_choose '("tea" "coffee"))` and press `C-u C-c C-e`.
Choose **coffee** in the GUI. The waiting editor expression should become
`"coffee"`. You can also run `M-x vrsjmp-browse-functions` to bring a call built
in the GUI back into the editor. `C-c C-b` always uses Emacs completion.

## Cleanup

Close the test Emacs. In the original terminal, stop only the fixture runtime:

```sh
kill "$chooser_pid"
wait "$chooser_pid"
rm -r "$chooser_tmp"
```

The old name `vrs-browse-functions-minibuffer` remains an alias for
`vrs-browse-functions`. `vrs-act-on-value` constructs a call;
`vrs-execute-action` runs an action. `(pick_photo)` and mobile REPL access remain
TODOs, with no implementation to test yet.
