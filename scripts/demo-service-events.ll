#!/usr/bin/env vrsctl
# Run this file, then (bind_srv :demo_todos) and (complete_demo_todo 1).
# Requires unicornleap on the daemon's PATH: https://github.com/kevinliddle/unicornleap

(def tasks '((:id 1 :title "Try service events" :done false)))

(defn! get_demo_todos () tasks)

(defn! complete_demo_todo (id)
  (def completed nil)
  (set tasks (map tasks (fn (task)
    (if (and! (eq? (get task :id) id) (not? (get task :done)))
      (begin
        (set completed `(:id ,id :title ,(get task :title) :done true))
        completed)
      task))))
  (if (not? (eq? completed nil)) (publish :demo_todo_completed completed))
  completed)

(defn! celebrate (task)
  (exec "unicornleap"))

# Start the subscriber first. Returning means its subscription is ready.
(spawn_srv! :demo_celebration
  :interface '()
  :topics '((:demo_todo_completed celebrate)))

(spawn_srv! :demo_todos :interface '(get_demo_todos complete_demo_todo))
