# A Tour of VRS

VRS is a personal programming environment for building and composing
software across your applications and devices. Its design combines a
practical, exploratory style of programming with relentlessly uniform
abstractions.

VRS is built around *services* that make their functions available to other
programs and user interfaces. They are written in [Lyric](manual.md#lyric), VRS's scripting language.
Like a Unix shell, it's built for hacking software together from
command-line programs, local databases, and UI automation—whatever gets
the job done.

The REPL, shell scripts, graphical applications, and even your [editor](manual.md#editor)
are *clients* of VRS. Each can run code, introspect live state, and use the full
capabilities of the runtime.

The environment extends across devices. Connect a laptop and a home
server as peers, and programs can use capabilities from both through
ordinary function calls.

Programs, data, and user interface markup share the same representation.
An expression that runs at the REPL can be included directly in user interface
markup as an action. User interactions, in turn, can be recorded as code—then
edited, combined, and replayed.

Development happens iteratively, against the running system. The editor
can call functions, inspect their results, and insert those values into
source code. It can also deploy services and update running ones. Programs
using those services pick up the changes without restarting.

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
# Import the todo service's functions into this REPL's namespace.
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
# Import Safari's browser functions.
(bind_srv :os_browser)

(active_tab)
# => (:title "VRS" :url "https://github.com/leoshimo/vrs")
```

A new function combines `active_tab` with `add_todo`, using the page's title
as the todo's title and its URL as the notes:

<!-- example: todo-from-tab -->
```vrs
# todos.ll — replace the final spawn_srv! with this block, then reevaluate.
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
(bind_srv :todos) # Discover the newly added function.
(add_todo_from_tab)
# => (:title "VRS" :notes "https://github.com/leoshimo/vrs")
```


## Build a user interface

A function that works at the REPL can become an action in a user interface
with very little code.

Start with markup that gives the user two choices: create a todo from the
typed text, or create one from the current Safari tab.

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

Serve the markup through `:vrsjmp`, the service that user interface clients
use to request pages and run selected actions:

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
includes what to display and the expression to execute:

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

The `vrsjmp` desktop app and the [vrsjmp-terminal shell script](../scripts/vrsjmp-terminal)
are general-purpose clients for this markup. One presents a graphical
interface; the other presents it in the terminal. Selecting an item sends it
back to the service, which evaluates its `:on_click` expression.

Try our todo user interface in the terminal:

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

  (+ capture_items completion_items))
```

The interface now supports adding a todo, finding it, and marking it complete.
For example, add *Buy coffee*, search for *coffee*, then select
*Complete: Buy coffee*.

This new capability works in both `:vrsjmp` clients without recompiling the
desktop app or editing the terminal script.


### User interactions are code

Selecting an item executes the `:on_click` expression in its markup—code
that other programs can inspect and manipulate as data.

Let’s use VRS’s built-in *pubsub* to see this in practice. Update `on_click`
to publish each expression to `:tour_cmd` before evaluating it:

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
[Another program](../scripts/cmd_macro.ll) can record and replay those expressions. The editor can
use those same expressions and live values to edit programs,
[as we’ll see later](#build-against-the-running-system).

A sequence of user interactions can become a program—and data for other
programs to work with.


## Build interactions from functions and data

So far, we've written the function calls behind each action explicitly.
VRS's metadata system lets us discover which functions to call and
what arguments to supply. Start with a todo and discover the actions
available for it. Or start with an action and find the todos it can operate on.

### Discover actions for a todo

Give each todo a leading `:todo` tag, then associate two operations with it: complete
the todo or copy its title.

<!-- example: tagged-todos -->
```vrs
# todos.ll — replace add_todo.
(defn! add_todo (title notes)
  (def todo `(:todo :title ,title :notes ,notes)) # The leading tag identifies todo data.
  (set todos (push todos todo))
  todo)
```

Functions use `interactive` metadata to describe the data they can act on:

<!-- example: todo-operations -->
```vrs
# todos.ll — replace complete_todo and add copy_todo.
(defn! complete_todo (todo)
  "Complete todo"
  (interactive :todo) # Offer this function as an action for :todo data.
  (set todos (filter todos (fn (item) (not? (eq? item todo))))))

(defn! copy_todo (todo)
  "Copy todo title"
  (interactive :todo)
  (exec "pbcopy" :stdin (get todo :title)))
```

Update the service to make these functions and their metadata available:

<!-- example: todo-metadata-service -->
```vrs
# todos.ll — replace the final spawn_srv!, then reevaluate the file.
(spawn_srv! :todos
  :interface '(get_todos add_todo complete_todo add_todo_from_tab copy_todo))
```

Given a todo, discover which functions can act on it:

<!-- example: todo-actions -->
```vrs
(bind_srv :todos)
(def todo (add_todo "Buy coffee" ""))
# => (:todo :title "Buy coffee" :notes "")

(entity_functions todo)
# => (complete_todo copy_todo)
```

The function metadata explains those choices and supplies their labels:

<!-- example: inspect-todo-metadata -->
```vrs
(get (meta complete_todo) :args)
# => ((:type :todo :name todo))

(get (meta copy_todo) :doc)
# => "Copy todo title"
```

Select one of those functions and combine it with the value:

<!-- example: invoke-todo-action -->
```vrs
(def action 'complete_todo) # Select Complete todo.
(def invocation `(,action ',todo))
# => (complete_todo '(:todo :title "Buy coffee" :notes ""))

(eval invocation)
```


### Find arguments for a function

To offer todos as arguments, clients need a way to fetch them. Register
`get_todos` as the source of available `:todo` values, then update the service:

<!-- example: todo-argument-provider -->
```vrs
# todos.ll — add before the final spawn_srv!, then reevaluate the file.
(set_entity_completions :todo 'get_todos)
```

Use `complete_todo`'s argument metadata to ask for available todos:

<!-- example: todo-arguments -->
```vrs
(bind_srv :todos) # Import the argument provider.
(add_todo "Buy tea" "")
(def argument (first (get (meta complete_todo) :args)))
# => (:type :todo :name todo)

(argument_entities (get argument :type))
# => ((:todo :title "Buy tea" :notes ""))
```

Choose one of those values and call the function:

<!-- example: invoke-with-todo -->
```vrs
(def todo (first (argument_entities (get argument :type))))
(complete_todo todo) # Choose the first candidate and call the function.
```

Using this metadata, clients can start with a function and interactively
prompt for its arguments—or start with selected data and discover applicable
actions. For `focus_window`, a client can list available windows; for a
window in that list, it can find functions such as `move_window`.

The `:vrsjmp` service uses these queries to build interactions from the
functions available at runtime. The editor can use them to construct calls
in source. Each client uses the same mechanism for unfamiliar entity types.

Entity discovery is one application of VRS's metadata system. The docstrings
above can also supply action labels, and parameter names can supply
placeholders when building a call.


## Connect more of your software

A service can give the rest of VRS a programmable way into an application.
Hack it together from whatever gets the job done: command-line programs,
local databases, UI automation, or a small program written for the task.

Existing services use several ways into applications and hardware:

-   [Safari](../scripts/os_browser.ll) uses AppleScript for the active page and SQLite for tabs synced
    from other devices. Both are available through the browser service.
-   [Apple Notes](../scripts/os_notes.ll) reads titles from SQLite, opens notes through application
    URLs, and creates them with AppleScript.
-   [Antinote](../scripts/antinote.ll) reads its SQLite database and opens notes through URLs. Its
    tagged notes also work with the action and argument menus above.
-   [Feedbin](../scripts/feedbin.ll) runs `feedbinctl` to search articles and save pages, with recurring
    indexing hosted in the runtime too.
-   [Notifications](../scripts/os_notify.ll) uses `osascript` on macOS and `notify-send` on Linux
    behind the same `notify` function.
-   [Keyboard brightness](../scripts/os_keyboard.ll) reaches a private macOS framework through
    `osascript` to read and change the backlight.
-   [Ditoo](../scripts/ditoo.ll) calls a dedicated CLI to drive a Bluetooth pixel display.


## Respond to events: make a unicorn leap

A service can publish and respond to events in the environment. This
makes it possible to attach new behavior to programs that are already
running.

Let's make a unicorn leap across the screen whenever a todo is completed:

<!-- example: completing-with-events -->
```vrs
# todos.ll — replace complete_todo and reevaluate the file.
(defn! complete_todo (todo)
  "Complete todo"
  (interactive :todo)
  (if (contains? todos todo)
    (begin
      (set todos (filter todos (fn (item) (not? (eq? item todo)))))
      (publish :todo_completed todo))))
```

A separate service responds by running [unicornleap](https://github.com/kevinliddle/unicornleap), which animates a
unicorn across the desktop:

<!-- example: celebration-service -->
```vrs
(defn! celebrate (todo)
  (exec "unicornleap"))

(spawn_srv! :celebration
  :topics '((:todo_completed celebrate))) # Call celebrate with each event's value.
```

Complete a todo:

<!-- example: complete-todo -->
```vrs
(bind_srv :todos)
(def todo (add_todo "Finish this example" ""))
(complete_todo todo)
# A unicorn leaps.
```

The todo service publishes what happened; the subscriber decides what to do with it.
Other subscribers could refresh an e-ink todo display, append completed
tasks to a daily journal, or update a pixel display on the desk.


## Run recurring work

VRS runs programs as lightweight *processes*. A service is a process that
handles function calls and events; a recurring job is another process
using the same runtime. This indexer works, waits fifteen minutes, and repeats:

<!-- example: index-loop -->
```vrs
(def indexer
  (spawn (fn ()
    (loop
      (publish :index_result (exec "feedbinctl" "index"))
      (sleep 900)))))
```

Its results can be followed through pubsub, displayed in a user interface,
or used by another program. The job can call services and use the same
debugging facilities as any other VRS program. There is no separate job
language, scheduler configuration, or observation system to learn.

The [Feedbin service](../scripts/feedbin.ll) uses this pattern with error reporting and a name
so reloading replaces its previous indexer.


## Build capabilities across devices

The todo service can run on a home server without a single edit to the
scripts, services, or user interfaces that use it. For example, with a
device named `mac-mini` [connected as a VRS peer](manual.md#connected-devices), send it the same `todos.ll`:

<!-- example: deploy-todos -->
```sh
# Stop the example service on the laptop.
vrsctl -c '(kill (find_srv :todos))'
# Send the local source file to mac-mini and start the service there.
vrsctl --node mac-mini ./todos.ll
```

The laptop discovers the remote service and calls it in the usual way:

<!-- example: call-remote-todos -->
```vrs
(bind_srv :todos) # Bind to the service now running on mac-mini.
(add_todo_from_tab)
(get_todos)
# => ((:todo :title "VRS" :notes "https://github.com/leoshimo/vrs"))
```

The todo service runs on the remote device and calls the laptop's
`:os_browser` service to capture its Safari page.

Source can also be evaluated on another device directly from the REPL or
editor. The `remote!` block runs on the named device:

<!-- example: remote-expression -->
```vrs
(remote! "mac-mini"
  (node_name))
# => "mac-mini"
```

For another example, the [Ditoo service](../scripts/ditoo.ll) exposes a Bluetooth pixel display
attached to a home Mac. A program on another device can put a message on it:

<!-- example: desk-display -->
```vrs
(call_timeout 120) # Allow time for the Bluetooth connection and upload.
(bind_srv :ditoo)
(ditoo_text "OFF DUTY")
```

A build script on another device could display *BUILD PASSED* on the desk.


## Build against the running system

The editor can use live data and metadata to help write a program. Open a
scratch buffer with the editor integration loaded:

```sh
emacs -Q -L emacs -l vrs-mode scratch.ll
```

Start by inspecting the running todo service:

<!-- example: inspect-live-todos -->
```vrs
(bind_srv :todos)
# Place the cursor after the closing parenthesis and press C-c C-e.
(get_todos)
```

Press `C-c C-v` at the same position to choose a todo from the result.
The editor replaces `(get_todos)` with the chosen value. Selecting
*VRS*, for example, leaves:

<!-- example: retain-todo-result -->
```vrs
'(:todo :title "VRS" :notes "https://github.com/leoshimo/vrs")
```

The same `interactive` metadata that describes actions for other clients
also works in the editor. With the cursor after the value's closing
parenthesis, press `C-c C-a`. The editor offers *Complete todo* and
*Copy todo title*. Choosing *Complete todo* turns the value into a call:

<!-- example: construct-todo-action -->
```vrs
(complete_todo '(:todo :title "VRS" :notes "https://github.com/leoshimo/vrs"))
```

The editor has assembled a function call from a live value and the service's
metadata. It inserts the call without executing it.

The debugger is another client of the running environment. For a closer
look at execution, open its viewer in a browser or terminal:

```sh
vrsctl dbg --web
# Or use vrsctl dbg for the terminal viewer.
```

Wrap the call in [`dbg!`](manual.md#debugging) and evaluate it:

<!-- example: debug-todos -->
```vrs
(dbg! (complete_todo '(:todo :title "VRS" :notes "https://github.com/leoshimo/vrs")))
```

This completes the todo and records the service call's arguments, result,
and source location in the viewer. `dbg!` returns the call's usual value.


## Toward a personal software ecosystem

A todo service starts as a small script. Extend it to integrate with your
browser, publish events, trigger automations, and power multiple user
interfaces. Its capabilities remain available to other programs and
clients, whether it runs locally or on another device.

VRS uses its programming language as the common interface between programs,
user interfaces, and devices. Programs, data, and user interface markup share
the same representation. This representation is powerful for both users
and programs. An interaction supplies code a person can edit; a program
can construct new actions for clients that already know how to present them.

Even the editor and debugger work with those same expressions and values
in the running environment. Inspect the data an application is using,
change a function, and see the application respond. Using the software
and shaping it happen in the same live environment.

A live, personal software ecosystem, grown one small script at a time.
An environment to make your own.
