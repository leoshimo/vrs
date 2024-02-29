#!/usr/bin/env vrsctl
# todos.ll - Simple TODOs
#

(def todos '())
(def id 0)

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
                           :on_click (list 'todos_on_click id)))))

(defn get_todos ()
  "(get_todos) - Returns the set of pending todos"
  todos)

(defn todos_on_click (id)
  "(todos_on_click ID) - Handle the click on a given todos item with ID"
  # filter clicked todos
  (set todos (filter todos (fn (it) (not? (contains? it id))))))

(spawn_srv :todos :interface '(get_todos add_todo todos_on_click))
