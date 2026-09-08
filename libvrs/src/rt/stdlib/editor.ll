# Native editor packets contain only lists and strings. Actual Lyric values,
# symbols, and call forms cross the editor boundary as Lyric source strings.
# No client needs to read a Lyric value using its own language's reader.

(defn! vrs/source (value)
  "Serialize source data losslessly; reject opaque or unprintable values."
  (def source (display value))
  (def parsed (try (read source)))
  (if (or! (err? parsed) (not? (eq? parsed value)))
    (error "This value cannot be retained as Lyric source"))
  source)

(defn! vrs/editor_value (value)
  (list (display value) (vrs/source (literal_form value))))

(defn! vrs/editor_choices (values fields)
  (if fields (set values (vrs/record_fields values)))
  (if (not? (list? values)) (error "Expected a list of choices"))
  (if (empty? values) (error "No values to choose from"))
  (map values (fn (value)
    (if fields
      (list (display (get value 0))
            (vrs/source (literal_form (get value 1))))
      (vrs/editor_value value)))))

(defn! vrs/editor_function (name)
  (def metadata (meta (eval name)))
  (list (vrs/source name)
        (or! (get metadata :doc) "")
        (if (eq? (get metadata :service) nil) "session" (display (get metadata :service)))
        (vrs/source (call_form name '()))
        (map (get metadata :args) (fn (arg)
          (list (vrs/source (get arg :name))
                (if (eq? (get arg :type) nil) "" (vrs/source (get arg :type))))))))

(defn! vrs/editor_functions ()
  (map (service_functions "") vrs/editor_function))

(defn! vrs/editor_actions (entity)
  (if (or! (not? (list? entity)) (not? (keyword? (get entity 0))))
    (error "Expected a tagged entity"))
  (map (filter (interactive_commands) (fn (name) (accepts_context? name entity)))
       vrs/editor_function))

(defn! vrs/editor_argument (type)
  (map (argument_entities type) vrs/editor_value))
