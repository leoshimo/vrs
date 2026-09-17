# A Tour of VRS

VRS is a personal programming environment for building and composing
software across your applications and devices. Its design combines a
practical, exploratory style of programming with relentlessly uniform
abstractions.

VRS is built around *services* written in [Lyric](manual.md#lyric), its own scripting language. Services make their functions available to one another
and to *clients*: the REPL, shell scripts, graphical applications, and the
editor. Each client can run code, introspect live state,
and use the full capabilities of the runtime.

Like a Unix shell, VRS is built for hacking software together from
command-line programs, local databases, and UI automation—whatever gets
the job done.

Programs, data, and user interface markup share the same representation.
An expression that runs at the REPL can be included directly in markup as
an action. User interactions, in turn, can be recorded as code—to edit and replay.

The environment extends across devices. Connect a laptop and a home
server as peers, and programs can use capabilities from both through
ordinary function calls.

Development happens iteratively, against the running system. The editor
can evaluate code, inspect results, and bring live values into source code.
You can update running services without restarting the programs that use them.

## How to follow along

With [VRS running](manual.md#start-and-evaluate), enter expressions in the
`vrsctl` REPL or evaluate a saved file with `vrsctl filename.ll`.

```console
$ vrsctl
vrs> (format "Hello, {}!" "computer")
"Hello, computer!"
```


## Build a todo service

A small script can add a capability to the whole VRS environment.

Here is a simple todo service:

<!-- example: todo-service -->
```vrs
# todos.ll
(def todos '())

(defn! get_todos () todos)

(defn! add_todo (title notes)
  (def todo `(:title ,title :notes ,notes))
  (set todos (push todos todo))
  todo)

(defn! complete_todo (todo)
  (set todos (filter todos (fn (item) (not? (eq? item todo))))))

# Start the service with these functions.
(spawn_srv! :todos :interface '(get_todos add_todo complete_todo))
```

Evaluating this file starts the service:

```sh
vrsctl todos.ll
```

Once the service is running, its functions can be called from the REPL:

<!-- example: use-todos -->
```vrs
# Bind the service to make its functions callable from this REPL.
(bind_srv :todos)

(get_todos)
# => ()

(add_todo "Buy coffee" "")
# => (:title "Buy coffee" :notes "")

(get_todos)
# => ((:title "Buy coffee" :notes ""))
```

The todo functions can be combined with capabilities from existing services.
For example, the [browser service](../scripts/os_browser.ll) provides Safari's
current page through `active_tab`:

<!-- example: compose-browser-todo -->
```vrs
# Make Safari's browser functions callable here.
(bind_srv :os_browser)

(active_tab)
# => (:title "VRS" :url "https://github.com/leoshimo/vrs")
```

A new function can combine `active_tab` with `add_todo`, using the page's
info for the todo's fields:

<!-- example: todo-from-tab -->
```vrs
# todos.ll — replace spawn_srv! with this block, then reevaluate.
(bind_srv :os_browser)

(defn! add_todo_from_tab ()
  (def tab (active_tab))
  (add_todo (get tab :title) (get tab :url)))

# Update the service to include the new function.
(spawn_srv! :todos
  :interface '(get_todos add_todo complete_todo add_todo_from_tab))
```

Reevaluating `todos.ll` makes the new function available to other programs:

<!-- example: capture-todo -->
```vrs
(bind_srv :todos) # Discover the new function.
(add_todo_from_tab)
# => (:title "VRS" :notes "https://github.com/leoshimo/vrs")
```


### Connect services through events

Services can work together through events too. With VRS's built-in
pubsub, one service can publish a value for other services to respond to.

Let’s publish an event whenever a todo is completed:

<!-- example: completing-with-events -->
```vrs
# todos.ll — replace complete_todo and reevaluate the file.
(defn! complete_todo (todo)
  (set todos (filter todos (fn (item) (not? (eq? item todo)))))
  (publish :todo_completed todo)) # Publish the event.
```

A separate service can subscribe to that event and run
[unicornleap](https://github.com/kevinliddle/unicornleap), sending a unicorn
across the screen when todos are completed:

<!-- example: celebration-service -->
```vrs
# celebration.ll — evaluate this file to start the subscriber.
(defn! celebrate (todo)
  (exec "unicornleap"))

(spawn_srv! :celebration
  :topics '((:todo_completed celebrate))) # Call celebrate on todo_completed.
```

Let’s try it at the REPL:

<!-- example: complete-todo -->
```vrs
(bind_srv :todos)
(def todo (add_todo "Finish this example" ""))
(complete_todo todo)
# A unicorn leaps.
```

Publishing events lets programs work together without directly calling one
another. A completed todo could update a daily journal and an e-ink
display; a newly saved article could trigger an offline download.
Subscribers can add these behaviors independently, without changes to the
services that publish the events.


## Build a user interface

A function that works at the REPL can become an action in user interfaces
with very little code.

Start with markup that gives the user two choices: creating a todo from the
typed text, or creating one from the current Safari tab.

<!-- example: todo-ui -->
```vrs
# todo-ui.ll
(bind_srv :todos)

# Build two user interface rows, each with a title and an action.
(defn! todo_items (query)
  # The first action uses the search text; the second uses the Safari tab.
  `((:title ,(format "Add todo: {}" query)
     :on_click (add_todo ,query ""))
    (:title "Add todo with Safari tab"
     :on_click (add_todo_from_tab))))
```

Each `:on_click` contains the expression that runs when the item is selected.

Serve the markup through `:vrsjmp`, a service powering command-bar user
interface clients:

<!-- example: ui-service -->
```vrs
# Append to todo-ui.ll, then evaluate the file.

# Describe the initial page shown by the client.
(defn! root_page ()
  '(:push_page :title "Todos" :prompt "Add a todo…"
    :get_items todo_items :args ()))

# Return todo choices for the current search text.
(defn! get_items (callback args query)
  (todo_items query))

# Execute the selected item's action.
(defn! on_click (item)
  (eval (get item :on_click))
  :close)

# Expose these functions through the :vrsjmp service.
(spawn_srv! :vrsjmp :interface '(root_page get_items on_click))
```

The service returns the user interface as ordinary Lyric data. Each item
includes what to display and the expression to execute on click:

<!-- example: ui-interaction -->
```vrs
(bind_srv :vrsjmp)
(def page (root_page))

# Fetch todo choices for the search query "Try VRS".
(def items
  (get_items (get page :get_items)
             (get page :args)
             "Try VRS"))
# Each item includes its label and the exact expression to run.
# => ((:title "Add todo: Try VRS"
#      :on_click (add_todo "Try VRS" ""))
#     (:title "Add todo with Safari tab"
#      :on_click (add_todo_from_tab)))
```

The [vrsjmp desktop app](../vrsjmp/README.md) and the [vrsjmp-terminal script](../scripts/vrsjmp-terminal)
are general-purpose clients for this markup. One is a desktop app; the other runs in the terminal.

Try our todo user interface in the terminal and add a few todos:

<!-- example: shell-client -->
```sh
./scripts/vrsjmp-terminal
```

### Show existing todos

Let’s update the interface to show existing todos, each with an action to mark it complete.

<!-- example: live-todos -->
```vrs
# todo-ui.ll — replace todo_items and reevaluate the file.
(defn! todo_items (query)
  # Keep the two actions for adding todos.
  (def capture_items
    `((:title ,(format "Add todo: {}" query)
       :on_click (add_todo ,query ""))
      (:title "Add todo with Safari tab"
       :on_click (add_todo_from_tab))))

  # Find existing todos matching the search text.
  (def matches (fuzzy_match query (get_todos)))

  # Give each match an action to complete it.
  (def completion_items
    (map matches
      (fn (todo)
        `(:title ,(format "Complete: {}" (get todo :title))
          :subtitle ,(get todo :notes)
          :on_click (complete_todo ',todo)))))

  # Return both capture actions and matching todo items.
  (+ capture_items completion_items))
```

The interface now supports adding a todo, finding it, and marking it complete.
For example, add *Buy coffee*, search for *coffee*, then select
*Complete: Buy coffee*.

The new capability works in both `:vrsjmp` clients without needing to recompile
the desktop app or edit the terminal program. The markup describes both the
user interface and its available actions—a pattern borrowed from hypermedia
on the web—allowing clients to adapt to new applications without
application-specific code.


### User interactions are code

Selecting an item executes the `:on_click` expression in its markup—code
that other programs can run as written, or inspect and transform as data.

Let’s use pubsub to watch those expressions as the interface is used.
Update `on_click` to publish each expression to `:tour_cmd` before
evaluating it:

<!-- example: publishing-clicks -->
```vrs
# todo-ui.ll — replace on_click and reevaluate the file, including spawn_srv!.
(defn! on_click (item)
  (def command (get item :on_click))
  (publish :tour_cmd command) # Publish the command.
  (eval command)
  :close)
```

Watch the published expressions by following the `tour_cmd` topic:

```sh
vrsctl --subscribe tour_cmd --follow
```

Adding a *Buy coffee* todo through the interface will now show this in the terminal:

```
(add_todo "Buy coffee" "")
```

Your interactions give you executable code you can edit and run again.

This symmetry between user interactions, code, and data is powerful.
Other programs can [record and replay](../scripts/cmd_macro.ll) user interactions.
The editor can use those same expressions and live values to edit programs,
[as we’ll see later](#build-against-the-running-system).

A sequence of user interactions can become a program—and data for other
programs to work with.


## Make functions `interactive`

So far, what can be done with a todo has been implicit in the code. We’ve
written the user interface to pair values from `get_todos` with `complete_todo`,
using our knowledge of which functions work with which values.

`interactive` makes that knowledge explicit through the runtime’s metadata
system. This enables programmatic discovery of *which functions apply to a
value*—such as a todo item—and *which values a function can operate on*.

### Discover actions for a todo

An *entity* is an ordinary value tagged with its kind. Let’s apply this to
our todo service by:

- Tagging our todo data with `:todo`.
- Marking the `todo` argument of `complete_todo` and `copy_todo` with
  `(interactive :todo)`.

<!-- example: tagged-todos -->
```vrs
# todos.ll — update these definitions and the final spawn_srv!, then reevaluate.
(defn! add_todo (title notes)
  # The leading :todo tag identifies todo data.
  (def todo `(:todo :title ,title :notes ,notes))
  (set todos (push todos todo))
  todo)

(defn! complete_todo (todo)
  (interactive :todo) # Mark this function as working with :todo values.
  (set todos (filter todos (fn (item) (not? (eq? item todo)))))
  (publish :todo_completed todo))

(defn! copy_todo (todo)
  (interactive :todo)
  (exec "pbcopy" :stdin (get todo :title)))

# Make the functions and their metadata available through the service.
(spawn_srv! :todos
  :interface '(get_todos add_todo complete_todo add_todo_from_tab copy_todo))
```

Given a todo, we can discover which functions operate on it:

<!-- example: todo-actions -->
```vrs
(bind_srv :todos)
(def todo (add_todo "Buy coffee" ""))
# => (:todo :title "Buy coffee" :notes "")

(interactive_functions todo)
# => (complete_todo copy_todo)
```

The result names `complete_todo` and `copy_todo`—two functions that accept
our original value as an argument: `(complete_todo todo)` or `(copy_todo todo)`.


### Find entities for a function

The `interactive` metadata on `complete_todo` declares that it operates on
`:todo` entities. But how can a program find the `:todo` entities available
in the running environment?

An *entity source* is a function that supplies available entities of a
particular type.

Let’s register `get_todos` as a source of `:todo` entities:

<!-- example: todo-argument-provider -->
```vrs
# todos.ll — add before spawn_srv!, then reevaluate the file.
(register_entity_source :todo 'get_todos)
```

Other programs can now use the `entities` function to ask for the type of
data they want—`:todo`, for example—without hard-coding how to fetch it:

<!-- example: todo-arguments -->
```vrs
(bind_srv :todos)

# Calls the registered source, get_todos.
(entities :todo)
# => ((:todo :title "Buy coffee" :notes ""))
```

We can pair entity queries with a function’s `interactive` metadata.
`focus_window`, for example, declares `(interactive :os/window)`. With that
declaration and an entity source, `vrsjmp` can query `(entities :os/window)`
and build a user interface for calling the function with one of those
windows. An editor can use the same information to construct calls with
live values and evaluate them—[as we’ll see later](#build-against-the-running-system).

This discovery happens at runtime. A program doesn’t need to know in
advance that a todo can be completed or copied; it can discover the
functions that operate on that value. Conversely, a program given
`complete_todo` doesn’t need to know how to fetch its inputs; it can read the
function’s `interactive` metadata and ask the runtime for available `:todo`
entities.


## Build capabilities across devices

In VRS, services on different devices can work together through ordinary
function calls. Our todo service can run on a home server without a single
edit to its code or to the programs and user interfaces that use it.

For example, with a home server named `mac-mini` [connected as a peer](manual.md#connected-devices),
we could deploy the same `todos.ll` there:

<!-- example: deploy-todos -->
```sh
# Stop the running todo service on the laptop.
vrsctl -c '(kill (find_srv :todos))'

# Start the todo service on mac-mini.
vrsctl --node mac-mini ./todos.ll
```

Programs on other connected devices can discover, bind to, and call the
service’s functions exactly as before:

<!-- example: call-remote-todos -->
```vrs
(bind_srv :todos) # Bind functions exposed by :todos on mac-mini.

(add_todo "VRS" "https://github.com/leoshimo/vrs")
(get_todos)
# => ((:todo :title "VRS" :notes "https://github.com/leoshimo/vrs"))
```

A program can also evaluate code directly on a specific device using
`remote!`:

<!-- example: remote-expression -->
```vrs
(remote! "mac-mini"
  (node_name))
# => "mac-mini"
```

VRS makes it simple to bring data and capabilities from multiple devices
into one programming environment. While your laptop sleeps, an always-on
home server could periodically update a [search index of articles](../scripts/feedbin.ll).
A small computer on your desk could drive a connected [pixel display](../scripts/ditoo.ll).

The data and capabilities exposed by those services are a function call
away from any program in the environment, wherever it runs.


## Build against the running system

VRS is a *live programming environment*: you develop the system from within
the same environment in which it runs.

### Develop interactively in the editor

The editor is just another VRS client, with full access to the runtime: you
can query live data, publish events, and update running services—even on
another device. Results can be inspected and brought directly back into
source code.

VRS’s editor integration lets us work directly with our running todo service
from Emacs. Open a scratch file with the integration loaded:

```sh
emacs -Q -L emacs -l vrs-mode scratch.ll
```

Then type these expressions into the editor:

<!-- example: inspect-live-todos -->
```vrs
(bind_srv :todos)
(first (get_todos))
```

Press `C-c C-c` (Ctrl+C twice) to evaluate the file in the running VRS
environment and see the results.

The editor can also evaluate individual expressions. Place the cursor after
the closing parenthesis of `(get_todos)`—before the final parenthesis of
`(first (get_todos))`—and press `C-c C-e` to see the list that `first` receives.

You can replace an expression with the value it returns. Press `C-u C-c C-e`
at the same position to evaluate the expression and retain its result in
source code:

<!-- example: retain-todo-result -->
```vrs
(first (get_todos))

# After C-u C-c C-e:
(first '((:todo :title "VRS" :notes "https://github.com/leoshimo/vrs")))
```

The editor can use the same introspection functions we’ve seen so far. It
can browse the functions exposed by the `:todos` service, fetch available
todos with `(entities :todo)`, and use our earlier `interactive` metadata
to find functions such as `complete_todo` to try on those values.

See the [editor section in the manual](manual.md#editor) for more details.

### Trace execution with the debugger

VRS’s built-in debugger lets us follow an expression’s execution. Open its
viewer:

```sh
vrsctl dbg --web
```

Wrap code in [`dbg!`](manual.md#debugging) and evaluate it in the editor or
REPL:

<!-- example: debug-todos -->
```vrs
(dbg!
  (add_todo "Try the debugger" "")
  (first (get_todos)))
```

The debugger shows each call’s arguments, result, elapsed time, and source
location. We can inspect the todo returned by `add_todo`, the list returned
by `get_todos`, and the item selected by `first`. `dbg!` evaluates the code
normally and returns its result.

Together, the editor and debugger let you develop through direct
interaction with the running system.


## Toward a personal software ecosystem

VRS uses its programming language as the common interface between programs,
user interfaces, and devices—including the tools you use to develop the
system itself. Programs, data, and UI markup share the same representation. Your interactions can produce code that you can build on.
Using the software and shaping it happen in the same live environment.

VRS aims to bring the practicality and freedom of scripting to a whole
personal software environment. You can try an idea against the running
system, see what happens, and develop iteratively and incrementally. A little
code can solve an immediate problem and leave the environment more capable
for the next hack.

A live, personal software ecosystem, grown one small script at a time—an
environment to make your own.


## Further reading

- [Manual](manual.md)
- [Design of VRS](design.md)

### Examples

- [Browsers](../scripts/os_browser.ll)
- [Apple Notes](../scripts/os_notes.ll)
- [Feedbin](../scripts/feedbin.ll)
- [Window management](../scripts/os_window.ll)
- [YouTube downloads](../scripts/youtube.ll)
- [Command macros](../scripts/cmd_macro.ll)
- [Tailscale](../scripts/tailscale.ll)
- [Ditoo Pixel Display](../scripts/ditoo.ll)
- [More examples](https://github.com/leoshimo/vrs/tree/main/scripts)
