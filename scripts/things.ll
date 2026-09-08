#!/usr/bin/env vrsctl
# Things owns persistence and sync. Arguments are data, never AppleScript source.

(defn! get_things_tasks ()
  "(get_things_tasks) - Read open Things tasks for local search"
  (def result (exec "osascript" "-l" "JavaScript" "-" :stdin """
    const tasks = Application('Things3').toDos;
    // Bulk property reads avoid one Apple event per task.
    const ids = tasks.id(), titles = tasks.name(), notes = tasks.notes(), statuses = tasks.status();
    if (![titles, notes, statuses].every(values => values.length === ids.length)) {
      throw new Error('Things changed while reading tasks; try again');
    }
    JSON.stringify(ids.map((id, i) => ({id, title: titles[i], notes: notes[i] || '', status: statuses[i]}))
      .filter(task => task.status === 'open'));
    """))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not read Things: " (get result :stderr))))
  (map (decode :json (get result :stdout)) (fn (task) (+ '(:things/task) task))))

(defn! open_things_task (task)
  "(open_things_task TASK) - Reveal an existing task in Things"
  (exec "osascript" "-" (get task :id) :stdin """
    on run argv
      set taskID to item 1 of argv
      tell application "Things3"
        show to do id taskID
        activate
      end tell
    end run
    """))

(defn! things_add (title notes)
  "(things_add TITLE NOTES) - Create a task in Things Inbox; use empty NOTES if none"
  (def result (exec "osascript" "-" title notes :stdin """
    on run argv
      set taskTitle to item 1 of argv
      if taskTitle is "" then error "A task title is required"
      tell application "Things3"
        set taskItem to make new to do with properties {name:taskTitle, notes:item 2 of argv} at beginning of list "Inbox"
        return id of taskItem
      end tell
    end run
    """))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not add to Things: " (get result :stderr))))
  (def task `(:things/task :id ,(get (split "\n" (get result :stdout)) 0)
             :title ,title :notes ,notes))
  (publish :things_event `(:created ,task))
  task)

(spawn_srv! :things :interface '(things_add get_things_tasks open_things_task))
