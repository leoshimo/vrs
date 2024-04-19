#!/usr/bin/env vrsctl
# nl_shell.ll - Natural-Language Frontend
#

(bind_srv :os_notify)
(bind_srv :chat)
(bind_srv :todos)
(bind_srv :rlist)
(bind_srv :os_cal)

# nil-out high priv bindings
(def exec nil)
(def fread nil)
(def fdump nil)

(def system_prompt
  (format "Respond as a program expression in Lyric, a Lisp Dialect, without markdown fences

Only use the following functions when handling user's requests:

{}

Use consistent titles when notifying and creating new todos.
Show notification describing work being done at each step.

Do work as quick as possible.

If task cannot be completed show a notification saying why.

Do not use conditional forms
Comment lines are marked by #

The result should be a single S-expression wrapped within a (begin ...) form"
           (join "
"                (help add_todo)
                 (help notify)
                 (help sleep)
                 (help add_rlist_active_tab)
                 (help create_event))))

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
           (eval code)))
  :ok)

(spawn_srv :nl_shell :interface '(codegen_exec codegen))

