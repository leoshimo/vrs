# Emacs walkthrough and manual checks

Use this as a standalone editor demo, or try individual interactions while
following the [runtime demos](../docs/demos.md). Start with a call, keep one of
its returned values, then build or execute another call using that value. The
same steps work with service data from your running computer.

## Paste the setup into scratch.ll

Use your running VRS and an Emacs buffer in `vrs-mode`. Load the current
`emacs/vrs-mode.el` and `emacs/vrs-choose.el` with `M-x load-file` if needed;
the daemon and `vrsctl` must also include the chooser helpers from this version.

Paste this block into `scratch.ll`, select it, and evaluate with `C-c C-r`.
It defines a service whose actions only append to an in-memory log. The same
definitions are committed in [chooser-demo.ll](chooser-demo.ll).

```lyric
(def demo_history '())

(defn! demo_items ()
  '((:demo/item :title "Blue notebook" :id 1 :note "first" :tags (alpha beta))
    (:demo/item :title "Green notebook" :id 2 :note "line\n\"quoted\"" :tags (gamma delta))))

(defn! demo_places ()
  '((:demo/place :title "Desk" :id 10)
    (:demo/place :title "Backpack" :id 20)))

(defn! demo_move (item place)
  "Move demo item"
  (interactive :demo/item :demo/place)
  (set demo_history (push demo_history (list :item item :place place)))
  :moved)

(defn! demo_note (item text)
  "Note demo item"
  (interactive :demo/item :demo/text)
  (set demo_history (push demo_history (list :item item :note text)))
  :noted)

(defn! demo_log () demo_history)

(set_entity_completions :demo/item 'demo_items)
(set_entity_completions :demo/place 'demo_places)
(spawn_srv! :chooser_demo :interface '(demo_items demo_places demo_move demo_note demo_log))
(bind_srv :chooser_demo)
```

The final `:ok` comes from `bind_srv`. Definitions and bindings remain available
for later evaluations. Re-evaluating this setup starts a new demo service with
an empty log.

For each expression below, put point immediately after its closing parenthesis
unless a region is specified. `TAB` shows choices when your completion setup
does not show them automatically. Use `C-c C-e` to inspect `(demo_items)` now.

## Command reference

| Interaction | Command | Effect on source |
| --- | --- | --- |
| Evaluate an expression | `C-c C-e` | Keep source; display its result. |
| Evaluate a region / buffer | `C-c C-r` / `C-c C-c` | Keep source; display the last result. |
| Evaluate a buffer with a transcript | `C-u C-c C-c` | Keep source; show expressions with commented results. |
| Retain an evaluated result | `C-u C-c C-e` / `C-u C-c C-r` | Replace source with its printed result; add a quote yourself for literal lists or symbols. |
| Choose a list element | `C-c C-v` | Replace source with the chosen literal, including needed quotes. |
| Choose a record field | `M-x vrs-choose-field` | Replace source with the field's literal value. |
| Insert a function call | `C-c C-b` | Insert argument names as placeholders. |
| Insert a filled function call | `C-u C-c C-b` | Prompt for arguments and insert the call; do not execute it. |
| Build an action on an entity | `C-c C-a` | Replace the entity expression with a filled call; do not execute it. |
| Execute an action on an entity | `M-x vrs-execute-action` | Keep source; run the chosen action and display its result. |
| Build a call in the GUI | `M-x vrsjmp-browse-functions` | Insert the call returned by vrsjmp. |

Value, field, and action commands accept an active region too. If it contains
several expressions, they use the last value. An entity expression runs once
before the action prompt; the selected action only runs when explicitly executed.

## 1. Choose a value

Insert `(demo_items)` and press `C-c C-v`. Each row shows a full Lyric value;
choose the one containing `"Green notebook"` and `:id 2`.
The expression becomes:

```lyric
'(:demo/item :title "Green notebook" :id 2 :note "line\n\"quoted\"" :tags (gamma delta))
```

Emacs displays the value directly, without extracting a title. `:title` is just
one of the fields in this fixture. The original fixture called both items
**Same** to check that equal titles did not confuse selection; no special name
or title field is required. Row numbers distinguish even identical values.

The tag, nested symbols, string escaping, and outer quote should survive.
Evaluate the retained value with `C-c C-e` to confirm it is usable source.

## 2. Choose fields and regions

Insert `(get (demo_items) 1)` and run `M-x vrs-choose-field`.
Choose `:note`: the source should become `"line\n\"quoted\""`.
Repeat on a fresh copy and choose `:tags`: expect `'(gamma delta)`.
This retains the field's current value; it does not generate a `get` expression.

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

On another blank line, run `C-u C-c C-b`. Choose `demo_move`, then the value
containing **Green notebook**, then the value containing **Desk**. Expect a call
with both complete, quoted entities. Evaluate `(demo_log)` on a separate line:
it should still be `()`.

Repeat with `demo_note`. After choosing an item, type `"hello"` for the text
argument, including quotes. The call should be inserted with `"hello"`, and the
log should still be empty. Entering a call such as `(demo_items)` as an argument
also only inserts that expression; it does not evaluate it during construction.

## 4. Start with an entity and construct an action

Insert `(get (demo_items) 1)` and press `C-c C-a`.
Choose **Move demo item**, then **Desk**. Expect:

```lyric
(demo_move '(:demo/item :title "Green notebook" :id 2 :note "line\n\"quoted\"" :tags (gamma delta))
           '(:demo/place :title "Desk" :id 10))
```

`(demo_log)` should still be empty. You now have a complete call you can edit,
save, or evaluate explicitly with `C-c C-e`. The same interaction works on the
quoted entity retained in step 1. The leading `:demo/item` matches the action's
first declared argument type; the title does not decide which actions apply.

## 5. Execute an action on a literal or a call

Paste this literal and run `M-x vrs-execute-action`:

```lyric
'(:demo/item :title "Green notebook" :id 2 :tags (gamma delta))
```

Choose **Move demo item**, then **Desk**. The literal stays in the buffer,
`*VRS Result*` shows `:moved`, and `(demo_log)` contains one new entry.

Now paste this call and run the same command directly, without first retaining
its result with a chooser:

```lyric
(get (demo_items) 1)
```

Choose **Move demo item**, then **Backpack**. The call runs once to obtain the
entity. Its source stays in place, the chosen action runs once, and `(demo_log)`
has one more entry. This is also how to act directly on live service results.

To inspect the concrete call published for macro recording, evaluate
`(subscribe :cmd)` with `C-c C-e`, perform one more `vrs-execute-action`, then
evaluate:

```lyric
(recv '(:topic_updated :cmd _))
```

The event contains the action call with the chosen entity and destination.

## 6. Show an evaluation transcript

In another scratch buffer in `vrs-mode`, paste only:

```lyric
(+ 1 2)
(+ 3 4)
```

`C-c C-c` currently displays `7`, the buffer's final value. `C-u C-c C-c`
evaluates the expressions separately and shows this in `*VRS Result*`:

```lyric
(+ 1 2)
# => 3
(+ 3 4)
# => 7
```

The source buffer stays unchanged. The prefix command evaluates the buffer
again; it does not recover output from the previous evaluation.

## 7. Make a choice in vrsjmp and keep it in Emacs

With vrsjmp running on your runtime, insert
`(vrsjmp_choose '("tea" "coffee"))` and press `C-u C-c C-e`.
Choose **coffee** in the GUI. The waiting editor expression becomes `"coffee"`.

You can also run `M-x vrsjmp-browse-functions` to bring a call built in the GUI
back into the editor. Enter selects placeholders; its **Fill arguments** action
chooses values before returning the call. The GUI lists functions bound in its
own service, so it need not show the same functions as the Emacs session.
`C-c C-b` and `C-u C-c C-b` use Emacs completion for those two interactions.

## Additional checks

- To check items with equal titles, choose the second item from
  `'((:demo/item :title "Same" :id 1) (:demo/item :title "Same" :id 2))`
  with `C-c C-v`. The retained value must have `:id 2`.
- Start `C-u C-c C-b`, choose a function and one argument, then press `C-g`.
  There should be no partial insertion and no new entry in `(demo_log)`.
- On `(missing_function)`, press `C-c C-v`. Expect an evaluation error and
  unchanged source. A subsequent `(demo_items)` should still work.
- On `(recv)`, press `C-c C-v`, then `C-g` while it waits. This resets the
  session. Evaluate `(bind_srv :chooser_demo)` to restore the bindings; the
  demo service and its history remain alive.

`M-x vrs-reset-session` also starts a fresh session. See the
[Emacs reference](../README.md#emacs-integration) for macro expansion and connection settings,
and for the shared Lyric implementation behind these commands.

The old name `vrs-browse-functions-minibuffer` remains an alias for
`vrs-browse-functions`. `(pick_photo)` and mobile REPL access remain TODOs.
