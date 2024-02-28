#!/usr/bin/env vrsctl
# nl_notify.ll - Natural-Language frontend to Notifications
#

(bind-srv :os_notify)
(bind-srv :chat)

(defn notify_user (title subtitle)
  "(notify_user TITLE SUBTITLE) - shows a notification, where TITLE and SUBTITLE are strings"
  (notify title subtitle))

(def prompt "Show notification with title hello and subtitle world after a three second delay")

(defn remind_me (prompt)
  "(remind_me PROMPT) - Schedules an OS notification to be scheduled in the future for given prompt"
  (spawn (fn ()
           (def code (chat (format
                "Respond as a program expression in Lyric, a Lisp Dialect, without markdown fences

                 You may use the following functions to handle request
                 {}

                 The result should be a single S-expression wrapped within a (begin ...) form

                 REQUEST: {}"
                            (join "\\n" 
                                  (help notify_user)
                                  (help sleep)
                                  )
                            prompt)))
           (eval (read code))))
  :ok)

(spawn-srv :nl_notify :interface '(remind_me))
