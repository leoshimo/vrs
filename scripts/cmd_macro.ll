#!/usr/bin/env vrsctl
# cmd_macro.ll - Command macro record and replay
#

# TODO: New builtin - throw / error
# TODO: New builtin - "hashmap" type? Set value for key, instead of (list :name ... :cmds ...) rebuilding
# TODO: New builtin - "or" to gracefully fallback nil values?

(def macros '())
(def record_pid nil)

(defn get_macros ()
  "(get_macros) - Returns list of macros"
  macros)

(defn clear_macros ()
  "(clear_macros) - Clear list of macros"
  (set macros '()))

(defn start_macro_record (name)
  "(start_macro_record NAME) - Starts recording the :cmd ran by user in macro called NAME"
  (if (macro_is_recording)
    (kill_record_proc))
  (start_record_proc name)
  :ok)

(defn macro_is_recording ()
  "(macro_is_recording) - Whether or not macro is currently being recorded"
  (not? (eq? record_pid nil)))

(defn end_macro_record ()
  "(end_macro_record) - Ends current macro recording."
  (if (not? (macro_is_recording)) nil
      (begin
       (save_macro)
       (kill_record_proc)
       :ok)))

(defn save_macro ()
  "(save_macro) - Save current macro stored in RECORDING"
  (def recording (call record_pid '(:get_recording)))
  (set macros (push macros recording))
  :ok)

# TODO: Instead of child process + manual `call`, consider ergonomic hook for topics on `spawn_srv` macro?
(defn start_record_proc (name)
  "(start_record_proc NAME) - Start a process that is recording commands"
  (set record_pid
       (spawn (fn ()
                (def recording (list :name name :cmds '(begin)))
                (subscribe :cmd)
                (loop (begin
                       (match (recv)
                         ((:topic_updated :cmd ('end_macro_record)) nil)
                         ((:topic_updated :cmd cmd) 
                            (set recording (list :name (get recording :name)
                                                :cmds (push (get recording :cmds) cmd))))
                         ((r src (:get_recording)) (send src (list r recording))))))))))

(defn kill_record_proc ()
  "(kill_record_proc) - Kill process listening to commands"
  (kill record_pid)
  (set record_pid nil))

(spawn_srv :cmd_macro :interface '(get_macros clear_macros start_macro_record end_macro_record macro_is_recording))
