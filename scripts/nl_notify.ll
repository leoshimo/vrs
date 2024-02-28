#!/usr/bin/env vrsctl
# nl_notify.ll - Natural-Language frontend to Notifications
#

(bind_srv :os_notify)
(bind_srv :chat)

(spawn_chat :nl_notify_chat
   (format "Respond as a program expression in Lyric, a Lisp Dialect, without markdown fences

            You may use the following functions to handle user requests:
            {}

            The result should be a single S-expression wrapped within a (begin ...) form"
           (join "\\n" 
                 (help notify)
                 (help sleep))))
(bind_srv :nl_notify_chat)
   
(defn remind_me (request)
  "(remind_me REQUEST) - Schedules an OS notification to be scheduled in the future for given REQUEST"
  (spawn (fn ()
           (def code (send_message request))
           (eval (read code))))
  :ok)

(spawn_srv :nl_notify :interface '(remind_me))

