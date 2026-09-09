(def ditoo_executable (shell_expand "~/.local/bin/ditooctl"))

(defn! ditoo_invoke (arguments)
  (def result (apply exec (+ (list ditoo_executable "--json") arguments)))
  (if (eq? (get result :exit) 0)
    (decode :json (get result :stdout))
    (error (get result :stderr))))

(defn! ditoo_text (text)
  "Show 1–4 characters still; 5–40 scroll and repeat. ASCII letters, digits, spaces, - . !; lowercase becomes uppercase. Longer or unsupported text errors without truncation."
  # Splitting on the empty separator includes an empty entry at each end.
  (def length (- (len (split "" text)) 2))
  (ditoo_invoke
    (if (contains? '(1 2 3 4) length)
      (list "text" "--" text)
      (list "text" "--scroll" "--" text))))

(defn! ditoo_brightness (percent)
  "Set display brightness to an integer 0–100; 0 blanks the display. Returns live readback."
  (ditoo_invoke (list "brightness" "--" (str percent))))

(defn! ditoo_mode (mode)
  "Select clock, light, gallery, visualizer, custom, or off. Custom can recall saved artwork; use ditoo_text for a new message. Returns live readback."
  (ditoo_invoke (list "mode" "--" mode)))

(spawn_srv! :ditoo :interface '(ditoo_text ditoo_brightness ditoo_mode))
