#!/usr/bin/env vrsctl
# nl_shell_demo.ll - Natural-Language Frontend (Demo)
#

(bind_srv :os_notify)
(bind_srv :chat)
(bind_srv :rlist)
(bind_srv :os_cal)

(def system_prompt
  (format "Respond as a program expression in Lyric, a Lisp Dialect, without markdown fences

Only use the following functions when handling user's requests:

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

(defn codegen (request)
  "(codegen REQUEST) - Generates an symbolic expression for a program to handle given user request"
  (def code_str (send_message request))
  (try (read code_str)))

(defn codegen_exec (request)
  "(codegen_exec REQUEST) - Generates an program to process user request then executes it"
  (notify "Working on your request" "thinking...")
  (spawn (fn ()
           (def code (codegen request))
           (publish :code code) 
           (eval (read code))))
  :ok)

(spawn_srv :nl_shell :interface '(codegen_exec codegen))
