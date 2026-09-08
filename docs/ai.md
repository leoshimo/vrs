# AI and VRS

A vrsjmp button, a REPL command, an editor, and an agent can all run the same call:

```lyric
(open_url "https://github.com/leoshimo/vrs")
```

The action is ordinary code. Inspect it, read `(help open_url)`, change its
arguments, or use it in a larger program. Agents have the same functions,
processes, messages, and timing primitives available to them.

## Natural-language commands

Ask for a program, inspect it, then run it:

```lyric
(def reminder (codegen "Remind me to stretch in ten minutes"))
(pretty reminder)
(spawn (fn () (eval reminder)))
```

The generated program can `sleep` before calling `notify`. It keeps running
after the request that created it finishes.

[Implementation](../scripts/nl_shell.ll)

## Interface generation

```lyric
(interfacegen "A five-minute timer button")
```

An example result:

```lyric
((:title "5 minutes"
  :on_click (begin
    (sleep 300)
    (notify "Timer finished" "Five minutes are up"))))
```

The [vrsjmp demo](../scripts/vrsjmp_interfacegen_demo.ll) displays these rows and
runs each button's code in a background process. The menu is data you can edit.

## Scheduling

```lyric
(schedule_the_day "tomorrow")
```

Uses the current calendar and your preferences to generate and run scheduling
code. [Implementation](../scripts/nl_scheduler.ll)

## Generative UI (WIP)

A focus session ends. Offer actions for the page you started with:

```lyric
(def page (active_tab))
(spawn (fn ()
  (sleep 1500)
  (present
    `((:title "Continue reading"
       :on_click (open_url ,(get page :url)))
      (:title "Save for later"
       :on_click (save_page ',page))))))
```

An agent can choose when to present an interface and compose its buttons from
the functions available in the running environment. New functions become new
building blocks. The calls behind the buttons remain readable and executable
from other clients.

## generated! (WIP)

Keep the generator beside its editable result:

```lyric
(def timer_menu
  (generated!
    :from (interfacegen "A five-minute timer button")
    :value
    '((:title "5 minutes"
       :on_click (begin
         (sleep 300)
         (notify "Timer finished" "Five minutes are up"))))))
```

Evaluate to use the saved value. Regenerate explicitly to review a new version
alongside your edits.
