#!/usr/bin/env vrsctl
# todos.ll - Simple TODOs
#

(def todos_path "~/todos.ll")

(def (:id id :todos todos) (begin
    (def res (try (fread todos_path)))
    (if (ok? res) res '(:id 0 :todos ()))))

(defn save_todos ()
  "(save_todos) - save current state to file"
  (fdump todos_path (list :id id :todos todos)))

(defn next_id ()
  "(next_id) - Return the next ID to assign"
  (def res id)
  (set id (+ id 1))
  res)

(defn add_todo (title)
  "(add_todo TITLE) - Add a new todo named TITLE"
  (def id (next_id))
  (publish :todos_event (list :todos_created title))
  (set todos (push todos
                   (list :todo
                      :id id
                      :title title)))
  (save_todos))

(defn get_todos ()
  "(get_todos) - Returns the set of pending todos"
  todos)

(defn set_todos_done (todo)
  "(set_todos_done TODO) - Mark the given TODO item from (get_todos) as done"
  (set_todos_done_by_id (get todo :id))
  (save_todos))

(defn set_todos_done_by_id (id)
  "(set_todos_done_by_id ID) - Mark the given TODO item with given ID as done "
  # filter clicked todos
  (set todos (filter todos (fn (it) (not? (contains? it id)))))
  (publish :todos_event (list :todos_completed id))
  (save_todos))

(spawn_srv :todos :interface '(get_todos add_todo set_todos_done set_todos_done_by_id))
