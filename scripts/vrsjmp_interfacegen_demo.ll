#!/usr/bin/env vrsctl
# vrsjmp_interfacegen_demo.ll - Demo for interface generation
#

(bind_srv :os_notify)
(bind_srv :chat)

(def system_prompt
  "Respond as a expression in Lyric, a Lisp Dialect. No markdown fences.

The response is a single list literal containing lists of format: (:title TITLE :on_click CLICK_FORM)

TITLE is a string expression

CLICK_FORM is a single function call expressions below:
(notify TITLE SUBTITLE) - Show OS Desktop UI for notification
(sleep SECS) - Sleep current process for SECS seconds, blocking execution.

Only use aforementioned functions for CLICK_FORM, or several function calls in a (begin ...) expression

Example output:
((:title \"Encourage\" :on_click (notify \"You're doing great!\" \"Keep up the good work!\"))
 (:title \"5 minute timer\" :on_click (begin (notify \"Timer Created\" \"Timer for 5 minutes\")
                                                  (sleep 300)
                                                  (notify \"Timer Completed\" \"5 minute timer completed\"))))
")

(spawn_chat :interfacegen_chat system_prompt)
(bind_srv :interfacegen_chat)

(defn interfacegen (request)
  "(interfacegen REQUEST) - Generates an interface for vrsjmp given request"
  (def interface_str (send_message request))
  (try (read interface_str)))

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
