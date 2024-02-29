#!/usr/bin/env vrsctl
# nl_agent.ll - Natural-Language Frontend
#

(bind_srv :os_notify)
(bind_srv :chat)
(bind_srv :todos)
(bind_srv :os_cal)

(spawn_chat :nl_agent_chat
   (format "Respond as a program expression in Lyric, a Lisp Dialect, without markdown fences

            You may use the following functions to handle user requests:
            {}

            Use consistent titles when notifying and creating new todos.
            Show notification describing work being done at each step.

            Do work as quick as possible.

            If task cannot be completed show a notification saying why.

            The result should be a single S-expression wrapped within a (begin ...) form"
           (join "
" 
                 (help add_todo)
                 (help notify)
                 (help sleep)
                 (help add_event))))
(bind_srv :nl_agent_chat)

   
(defn do_it (request)
  "(do_it REQUEST) - Schedules an OS notification to be scheduled in the future for given REQUEST"
  (notify "Working on your request" "thinking...")
  (spawn (fn ()
           (def code (send_message request))
           (publish :code code) 
           (eval (read code))))
  :ok)

(spawn_srv :nl_agent :interface '(do_it))

