#!/usr/bin/env vrsctl

(def msgs '())

# HACK: Needs list splicing from msgs into args
(def msgs_args '(exec "cogni"
                 "-s" "You are a helpful assistant. Prefer concise answers of one to two sentences"))

(defn chat (user_msg)
  (set msgs (push msgs (list :user user_msg)))
  (set msgs_args (push msgs_args "-u"))
  (set msgs_args (push msgs_args user_msg))

  (def (:ok assistant_msg) (eval msgs_args))

  (set msgs (push msgs (list :assistant assistant_msg)))
  (set msgs_args (push msgs_args "-a"))
  (set msgs_args (push msgs_args assistant_msg))

  assistant_msg)

(defn messages ()
  msgs)

(spawn-srv :chat :interface '(chat messages))
