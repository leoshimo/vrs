#!/usr/bin/env vrsctl
# vrsjmp_interfacegen_demo.ll - Demo for interface generation
#

(bind_srv :os_notify)
(bind_srv :interfacegen)

(call_timeout 60)
(def items (interfacegen "UI for pomodoro timer for 10 seconds, 5 minutes, and 25 minutes"))

(defn! get_items (callback args query)
  (apply (eval callback) (push args query)))

(defn! root_page ()
  '(:push_page :get_items root_items :prompt "Search"))

(defn! root_items (query)
  "Return interface items"
  (map (fuzzy_match query items) (fn (item)
    `(:title ,(get item :title)
      :on_click (run_in_background ',(get item :on_click))))))

(defn! run_in_background (command)
  "Timers and generated commands can outlive the palette"
  (spawn (fn ()
    (def result (try (eval command)))
    (if (err? result) (notify "Encountered error" (display result))))))

(defn! on_click (item)
  "Handle an on_click payload from item"
  (def cmd (get item :on_click))
  (publish :cmd cmd)
  (def result (eval cmd))
  (if (list? result)
    (if (eq? (get result 0) :push_page) result :close)
    :close))

(spawn_srv! :vrsjmp :interface '(root_page get_items on_click))
