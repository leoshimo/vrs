#!/usr/bin/env vrsctl
# nl_shell_demo.ll - Natural-Language Frontend (Demo)
#

(bind_srv :os_notify)
(bind_srv :chat)
(bind_srv :rlist)
(bind_srv :os_cal)

(def system_prompt
  (format "Respond as a program expression in Lyric, a Lisp Dialect, without markdown fences

Only used the following functions when handling user's requests:

{}

Use notifications to communicate with user and describing work being done at each step.

Do work as quick as possible.

If task cannot be completed show a notification saying why.

The result should be a single S-expression wrapped within a (begin ...) form"
           (join "
"                # DEMO:
                 # (help notify)
                 # (help sleep)
                 # (help add_rlist_active_tab)
                 # (help add_event)
)))

(spawn_chat :nl_shell_chat system_prompt)
(bind_srv :nl_shell_chat)

(defn do_it (request)
  "(do_it REQUEST) - Given user request REQUEST, executes operations to service request on local device"
  (notify "Working on your request" "thinking...")
  (spawn (fn ()
           (def code (send_message request))
           (publish :code code)
           (eval (read code))))
  :ok)

(spawn_srv :nl_shell :interface '(do_it))

