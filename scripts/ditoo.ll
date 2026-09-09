(def ditoo_executable (shell_expand "~/.local/bin/ditooctl"))

(defn! ditoo_invoke (arguments)
  (def result (apply exec (+ (list ditoo_executable "--json") arguments)))
  (if (eq? (get result :exit) 0)
    (decode :json (get result :stdout))
    (error (get result :stderr))))

(defn! ditoo_text (text)
  "Show 1–4 characters on the Ditoo"
  (ditoo_invoke (list "text" "--" text)))

(defn! ditoo_scroll (text)
  "Scroll 1–40 characters; the animation loops after the CLI exits"
  (ditoo_invoke (list "text" "--scroll" "--" text)))

(spawn_srv! :ditoo :interface '(ditoo_text ditoo_scroll))
