# Demos

## Inline evaluation

In Emacs, write:

```lyric
(get (active_tab) :url)
```

Put point on its closing parenthesis and press `C-u C-c C-e`. The expression
becomes the URL of the actual browser tab:

```lyric
"https://github.com/leoshimo/vrs"
```

Keep the value in a list, pass it to another function, or edit it.

## Use vrsjmp to edit source

```lyric
(open_url (vrsjmp_choose_field (active_tab)))
```

Replace just the inner `vrsjmp_choose_field` expression with `C-u C-c C-e`.
Vrsjmp shows the tab's fields. Choose `:url` and the source becomes:

```lyric
(open_url "https://github.com/leoshimo/vrs")
```

The browser supplied the value, vrsjmp selected it, and the editor kept it as code.

## Build a function call

Run `M-x vrs-browse-functions` in Emacs. Choose a service function to insert a
call with argument placeholders. Use **Fill arguments** to choose live objects
when the function has completions. For example:

```lyric
(focus_window '(:os/window :id 123 :app "Safari" :title "VRS"))
```

Vrsjmp's **Browse Functions** entry runs calls directly.

## A program waits for a GUI choice

Run this from a REPL, script, or editor:

```lyric
(def page
  (vrsjmp_choose
    '((:title "VRS" :url "https://github.com/leoshimo/vrs")
      (:title "Emacs" :url "https://www.gnu.org/software/emacs/"))))
(open_url (get page :url))
```

The program waits while you choose in vrsjmp, then opens the selected page.
The chooser returns the original record.

## Inspect a GUI action

Subscribe in a REPL or editor, then run an action in vrsjmp:

```lyric
(subscribe :cmd)
(def action (get (recv '(:topic_updated :cmd _)) 2))
(pretty action)
```

The result is the command vrsjmp ran. Change its arguments, evaluate it again,
or keep a variation as a new command. The debugging tool is itself something
you assemble from the live system.
