#!/usr/bin/env vrsctl
# vrsjmp_interfacegen_demo.ll - Demo for interface generation
#

(bind_srv :os_notify)
(bind_srv :interfacegen)

(def items (interfacegen "UI for pomodoro timer for 10 seconds, 5 minutes, and 25 minutes"))

(defn get_items (query)
  "Return interface items"
  items)

(defn on_click (item)
  "Handle an on_click payload from item"
  (def cmd (get item :on_click))
  (publish :cmd cmd)
  (spawn (fn ()
           (def res (try (eval cmd)))
           (if (err? res)
             (notify "Encountered error" (format "{}" err)))))
  :ok)

(spawn_srv :vrsjmp :interface '(get_items on_click))
