#!/usr/bin/env vrsctl
# chat.ll - Chat Dynamic Supervisor Service
#

(defn msgs_to_cogni_cmd (msgs)
  "(msgs_to_cogni_cmd MSGS) - Given a set of message s-exprs, return the cogni command for messages"
  (def exec_cmd '(exec "cogni"))

  (map msgs (fn (m) (match m
                      ((:system msg) (set exec_cmd (+ exec_cmd (list "-s" msg))))
                      ((:user msg) (set exec_cmd (+ exec_cmd (list "-u" msg))))
                      ((:assistant msg) (set exec_cmd (+ exec_cmd (list "-a" msg))))
                      (_ (error "Unrecognized message")))))

  exec_cmd)

(defn run_llm (msgs)
  "(run_llm MSGS) - Given a set of message s-exprs, run the LLM to receive an assistant message"
  (eval (msgs_to_cogni_cmd msgs)))

(defn spawn_chat (chat_name system_prompt)
  "(spawn_chat CHAT_NAME SYSTEM_PROMPT) - Spawn a new process registered as CHAT_NAME with SYSTEM_PROMPT for a new chat session"
  (spawn (fn ()
           (def msgs (list (list :system system_prompt)))

           (defn send_message (message)
             "(send_message MESSAGE) - Send message to chat session then return new assistant message"
             (set msgs (push msgs (list :user message)))
             (def (:ok assistant_msg) (run_llm msgs))
             (set msgs (push msgs (list :assistant assistant_msg)))
             assistant_msg)

           (defn get_messages ()
             "(get_messages) - Returns all messages in session"
             msgs)

           (spawn-srv chat_name :interface '(get_messages send_message))
           )))

(spawn-srv :chat :interface '(spawn_chat))
