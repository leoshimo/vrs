# A Tour of VRS

VRS is a personal programming environment for building and composing
software across your applications and devices. Its design combines a
practical, exploratory style of programming with relentlessly uniform
abstractions.

In VRS, *services* make their functions available to other programs and user
interfaces. They are written in [Lyric](guide-lyric.org), VRS's scripting language.
Like a Unix shell, it's built for hacking software together from
command-line programs, local databases, and UI automation—whatever gets
the job done.

The REPL, shell scripts, graphical applications, and even your [editor](guide-emacs.org) and [debugger](guide-debug.org)
are *clients* of VRS. Each can run code, introspect live state, and use the full
capabilities of the runtime.

The environment extends across devices. Connect a laptop and a home
server as peers, and programs can use capabilities from both through
ordinary function calls.

Programs and user interface markup share the same exact representation.
An expression from the REPL can be included directly in that markup as
an action. User interactions, in turn, can be recorded directly into
programs—then edited, combined, and run again.

Development happens iteratively, against the running system. The editor
can call functions, inspect their results, and insert those values into
source code. Update a running service and the programs using it pick up
the changes without restarting.

VRS is a live, personal software ecosystem, grown one small script at a time—with
the whole system available to programs and user interfaces across all your devices.


## Working through the tour

With [VRS running](../README.md), the examples can be run in the `vrsctl` REPL.

For the full live editing experience, use [the editor](guide-emacs.org):

```sh
emacs -Q -L emacs -l vrs-mode scratch.ll
```

In the editor, `C-c C-e` evaluates an expression and `C-c C-c` evaluates
the buffer.


## Build a todo service

A small script can add a capability to the whole VRS environment,
available across connected devices through ordinary function calls.

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

# Start (or update) the service with these functions.
(spawn_srv! :todos :interface '(get_todos add_todo complete_todo))
```

Once deployed, its functions are available from the REPL:

<!-- example: use-todos -->
```vrs
(bind_srv :todos)
(get_todos)
# => ()

(add_todo "Buy coffee" "")
# => (:title "Buy coffee" :notes "")

(get_todos)
# => ((:title "Buy coffee" :notes ""))
```

The todo functions can compose with existing services. VRS's
[browser service](../scripts/os_browser.ll) exposes Safari's current page through `active_tab`.
Inspect its result in the REPL, then use the title and URL as a todo's notes:

<!-- example: compose-browser-todo -->
```vrs
(bind_srv :os_browser)
(def tab (active_tab))
# => (:title "VRS" :url "https://github.com/leoshimo/vrs")

(add_todo "Try VRS"
  (format "{}\n{}" (get tab :title) (get tab :url)))
# => (:title "Try VRS" :notes "VRS\nhttps://github.com/leoshimo/vrs")
```

The REPL shows the browser's actual data, so the composition can be tried
before it becomes part of the service. In the editor, the result can also
be inserted directly into source:

<!-- example: retain-browser-result -->
```vrs
# Place the cursor after this expression and press C-u C-c C-e.
(active_tab)

# The expression is replaced with its result:
'(:title "VRS" :url "https://github.com/leoshimo/vrs")
```

Add a function that creates a todo with the current Safari page attached:

<!-- example: todo-from-tab -->
```vrs
# todos.ll — replace the final spawn_srv! with this block, then reevaluate.
(bind_srv :os_browser)

(defn! add_todo_from_tab (title)
  (def tab (active_tab))
  (add_todo title
    (format "{}\n{}" (get tab :title) (get tab :url))))

# Update the service to include the new function.
(spawn_srv! :todos
  :interface '(get_todos add_todo complete_todo add_todo_from_tab))
```

Reevaluating `todos.ll` makes this new function available through the service:

<!-- example: capture-todo -->
```vrs
(bind_srv :todos) # Discover the newly added function.
(add_todo_from_tab "Try VRS")
# => (:title "Try VRS" :notes "VRS\nhttps://github.com/leoshimo/vrs")
```


## Build a user interface

A function that works at the REPL can become an action in a user interface
with very little code. VRS's user interface markup is *self-describing*:
it specifies what to display and what to execute.

Each `:on_click` field in the markup contains the actual expression to
execute, arguments included. Add two actions: create a todo, or create a
todo with context from Safari.

<!-- example: todo-ui -->
```vrs
# todo-ui.ll
(bind_srv :todos)

(defn! add_todo_items (query)
  # Use the search text as the title of a new todo.
  `((:title ,(format "Add todo: {}" query)
     :on_click (add_todo ,query ""))
    (:title ,(format "Add todo with this page: {}" query)
     :on_click (add_todo_from_tab ,query))))

(defn! todo_items (query)
  (add_todo_items query))
```

Build a `:vrsjmp` service to serve the page and evaluate selections:

<!-- example: ui-service -->
```vrs
# Append to todo-ui.ll, then evaluate the file.
(defn! root_page ()
  # The page names the function that supplies its entries.
  '(:push_page :title "Todos" :prompt "Add a todo…"
    :get_items todo_items :args ()))

(defn! get_items (callback args query)
  # Call the provider named by the page, passing the search text.
  (apply (eval callback) (push args query)))

(defn! on_click (item)
  # Execute the expression included in the selected item.
  (eval (get item :on_click))
  :close) # Close the user interface after the selection.

# The desktop app and terminal script use this shared service.
(spawn_srv! :vrsjmp :interface '(root_page get_items on_click))
```

The REPL can show what a client receives:

<!-- example: ui-interaction -->
```vrs
(bind_srv :vrsjmp)
(def page (root_page)) # The description of the current user interface.

# Ask the provider named in the page for items matching the query.
(def items
  (get_items (get page :get_items)
             (get page :args)
             "Buy coffee"))

(get items 1) # Inspect the second choice, including its executable action.
# => (:title "Add todo with this page: Buy coffee"
#     :on_click (add_todo_from_tab "Buy coffee"))
```

The desktop app `vrsjmp` and the shell script
[vrsjmp-terminal](../scripts/vrsjmp-terminal) share this `:vrsjmp` service. Both display the same markup;
selecting an entry asks the service to evaluate the expression included in it.
The same user interface runs in the terminal:

<!-- example: shell-client -->
```sh
./scripts/vrsjmp-terminal
```

Typing *Buy coffee* in either client displays the two actions. Neither
client contains a todo implementation: the service supplies the markup
and the expressions to run. The terminal script uses `fzf` for presentation;
the desktop app renders graphical entries.


### Show and complete todos

Extend the running application to list, search, and complete todos.
Keep the capture actions and add a function that turns existing todos into
choices:

<!-- example: live-todos -->
```vrs
# todo-ui.ll — add this helper, replace todo_items, and reevaluate the file.
(defn! matching_todo_items (query)
  # Search real todos, then attach the call that completes each one.
  (map (fuzzy_match query (get_todos)) (fn (todo)
    `(:title ,(format "Complete: {}" (get todo :title))
      :subtitle ,(get todo :notes)
      :on_click (complete_todo ',todo)))))

(defn! todo_items (query)
  (+ (add_todo_items query) (matching_todo_items query)))
```

Search for *coffee*: *Complete: Buy coffee* now appears alongside the
capture actions. The new search and completion actions are immediately
available in both user interfaces—without rebuilding the desktop app or
editing the terminal script.


## Turn an interaction into a program

Selecting an item executes the `:on_click` expression from its markup—the
same code a program would run. Use the runtime's built-in *pubsub* to inspect those
expressions as the software is used. Publish each selected action to a
topic, `:tour_cmd`, for another client to follow:

<!-- example: publishing-clicks -->
```vrs
# todo-ui.ll — replace on_click and reevaluate the file, including spawn_srv!.
(defn! on_click (item)
  (def command (get item :on_click))
  (publish :tour_cmd command) # Send the expression to subscribers before running it.
  (eval command)
  :close)
```

A second client follows the topic:

```sh
vrsctl --subscribe tour_cmd --follow
```

Selecting *Add todo: Buy coffee* in the terminal or desktop app prints:

```
(add_todo "Buy coffee" "")
```

The output is exactly the code that ran. It can be pasted into source,
edited, combined with other expressions, or run again.

For example, the [command-macro service](../scripts/cmd_macro.ll) collects these expressions into a
program to replay. The full `:vrsjmp` service also uses executable expressions
to retry failed calls, retaining their argument values in the call itself.

Each interaction can reveal a function call with real arguments.
Using the application reveals how to program it.


## Build interactions from functions and data

Earlier, we wrote the function calls to add and complete todos. VRS's
metadata interface also lets programs discover which functions accept a
value, or fetch available values from the environment to pass to a function.

Give each todo a leading `:todo` tag, then associate two operations with it: complete
the todo or copy its title.

<!-- example: tagged-todos -->
```vrs
# todos.ll — replace add_todo.
(defn! add_todo (title notes)
  (def todo `(:todo :title ,title :notes ,notes)) # The leading tag identifies a todo.
  (set todos (push todos todo))
  todo)
```

Functions declare what they accept through `interactive` metadata:

<!-- example: todo-operations -->
```vrs
# todos.ll — replace complete_todo and add copy_todo.
(defn! complete_todo (todo)
  "Complete todo"
  (interactive :todo) # This function accepts a :todo entity.
  (set todos (filter todos (fn (item) (not? (eq? item todo))))))

(defn! copy_todo (todo)
  "Copy todo title"
  (interactive :todo)
  (exec "pbcopy" :stdin (get todo :title)))
```

Register `get_todos` as the provider of available `:todo` entities. Clients
can then fetch current todos to use as arguments. Updating the service
makes both the functions and their metadata available:

<!-- example: todo-metadata-service -->
```vrs
# todos.ll — replace the final spawn_srv!, then reevaluate the file.
(set_entity_completions :todo 'get_todos)
(spawn_srv! :todos
  :interface '(get_todos add_todo complete_todo add_todo_from_tab copy_todo))
```

VRS makes this metadata available to programs. The REPL can inspect
exactly what another client would receive:

<!-- example: inspect-todo-metadata -->
```vrs
(bind_srv :todos)
(get (meta complete_todo) :args)
# => ((:type :todo :name todo))

(get (meta copy_todo) :doc)
# => "Copy todo title"
```

These declarations support two directions through the same interface.


### Discover actions for a todo

Given a todo, discover which functions accept it:

<!-- example: todo-actions -->
```vrs
(def todo (add_todo "Buy coffee" ""))
# => (:todo :title "Buy coffee" :notes "")

(entity_functions todo)
# => (complete_todo copy_todo)
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

Start with a function this time. Read its argument type, then fetch
available values of that type from the environment:

<!-- example: todo-arguments -->
```vrs
(add_todo "Buy tea" "")
(def argument (first (get (meta complete_todo) :args)))
# => (:type :todo :name todo)

(argument_entities (get argument :type))
# => ((:todo :title "Buy tea" :notes ""))
```

The provider supplies the todos currently available through the service,
including when that service runs on another device:

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
tasks to a daily journal, or update a pixel display on the desk. Each is
a small program using capabilities from the same environment.


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
device named `mac-mini` [connected as a VRS peer](../README.md#init-scripts-and-nodes), send it the same `todos.ll`:

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
(add_todo_from_tab "Try VRS")
(get_todos)
# => ((:todo :title "Try VRS" :notes "VRS\nhttps://github.com/leoshimo/vrs"))
```

The todo service runs on the remote device and calls the laptop's
`:os_browser` service to capture its Safari page. The implementation of
`add_todo_from_tab` needs no edits. The separate `todo-ui.ll` service and
both user interfaces work without changes too.

Source can also be evaluated on another device directly from the REPL or
editor. The `remote!` block runs on the named device:

<!-- example: remote-expression -->
```vrs
(remote! "mac-mini"
  (node_name))
# => "mac-mini"
```

Making a capability available to VRS makes it available to all these uses,
without another integration for each client or device.

For another example, the [Ditoo service](../scripts/ditoo.ll) exposes a Bluetooth pixel display
attached to a home Mac. A program on another device can put a message on it:

<!-- example: desk-display -->
```vrs
(call_timeout 120) # Allow time for the Bluetooth connection and upload.
(bind_srv :ditoo)
(ditoo_text "OFF DUTY")
```

The same call can appear in a user interface or an automation. A build
script on another device could display *BUILD PASSED* on the desk.


## Inspect execution

The debugger is another client of the running environment. VRS's
[source-embedded tools](guide-debug.org) make execution observable through code. For example,
`dbg!` records calls, arguments, and results while returning the expression's
usual value. Its viewer can run in a browser or terminal:

```sh
vrsctl dbg --web
# Or use vrsctl dbg for the terminal viewer.
```

The `dbg!` expression returns todo titles while recording how they were
computed:

<!-- example: debug-todos -->
```vrs
(bind_srv :todos)
(dbg! (map (get_todos) (fn (todo) (get todo :title))))
```

The viewer shows the service call and each local function call, with their
arguments, results, and source locations.


## That's VRS

A todo service starts as a small script. Extend it to integrate with your
browser, publish events, trigger automations, and power multiple user
interfaces. Its capabilities remain available to other programs and
clients, whether it runs locally or on another device.

VRS uses its programming language as the common interface between programs,
user interfaces, and devices. Source code, user interface markup, and
recorded actions share symbolic expressions. An expression can be tried
at the REPL and included directly in user interface markup as an action.
Recording the interaction produces that same expression as source code.
The shared representation does much of the work that would otherwise
require custom integration code.

Even the editor and debugger work with those same expressions and values
in the running environment. Inspect the data an application is using,
change a function, and see the application respond. Using the software
and shaping it happen in the same live environment.

A live, personal software ecosystem, grown one small script at a time.
