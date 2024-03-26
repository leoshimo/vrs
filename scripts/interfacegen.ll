#!/usr/bin/env vrsctl
# interfacegen.ll - Interface Generator
#

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

(spawn_srv :interfacegen :interface '(interfacegen))
