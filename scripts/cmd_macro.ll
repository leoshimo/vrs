#!/usr/bin/env vrsctl
# Command recording shares the service's state and event loop.

(def macros '())
(def recording nil)

(defn! get_macros ()
  "(get_macros) - Returns recorded command macros"
  macros)

(defn! clear_macros ()
  "(clear_macros) - Clear saved macros"
  (set macros '()))

(defn! start_macro_record (name)
  "(start_macro_record NAME) - Begin recording handled :cmd events; replace any unfinished recording"
  (set recording `(:name ,name :cmds (begin)))
  :ok)

(defn! macro_is_recording ()
  "(macro_is_recording) - Whether a command macro is being recorded"
  (not? (eq? recording nil)))

(defn! end_macro_record ()
  "(end_macro_record) - Save commands handled so far and stop recording"
  (if (macro_is_recording)
    (begin
      (set macros (push macros recording))
      (set recording nil)
      :ok)
    nil))

(defn! record_command (cmd)
  # vrsjmp publishes these control commands too; never replay them in a macro.
  (def control
    (and! (list? cmd) (not? (empty? cmd))
      (contains? '(start_macro_record end_macro_record) (get cmd 0))))
  (if (and! (macro_is_recording) (not? control))
    (set recording `(:name ,(get recording :name)
                     :cmds ,(push (get recording :cmds) cmd)))))

(spawn_srv! :cmd_macro
  :interface '(get_macros clear_macros start_macro_record end_macro_record macro_is_recording)
  :topics '((:cmd record_command)))
