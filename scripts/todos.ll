#!/usr/bin/env vrsctl
# todos.ll - Simple TODOs
#

(def todos_path "~/todos.ll")

(def (:id id :todos todos) (begin
    (def res (try (fread todos_path)))
    (if (ok? res) res '(:id 0 :todos ()))))

(defn save_todos ()
  "(save_todos) - save current state to file"
  (publish :probe (list :id id :todos todos))
  (fdump todos_path (list :id id :todos todos)))

(defn next_id ()
  "(next_id) - Return the next ID to assign"
  (def res id)
  (set id (+ id 1))
  res)

(defn add_todo (title)
  "(add_todo TITLE) - Add a new todo named TITLE"
  (def id (next_id))
  (set todos (push todos
                     (list :id id
                           :title (format "TODO - {}" title)
                           :on_click (list 'todos_on_click id))))
  (save_todos))

(defn get_todos ()
  "(get_todos) - Returns the set of pending todos"
  todos)

(defn todos_on_click (id)
  "(todos_on_click ID) - Handle the click on a given todos item with ID"
  # filter clicked todos
  (set todos (filter todos (fn (it) (not? (contains? it id)))))
  (save_todos))

(spawn_srv :todos :interface '(get_todos add_todo todos_on_click))
