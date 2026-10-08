# Native editor packets contain only lists and strings. Actual Lyric values,
# symbols, and call forms cross the editor boundary as Lyric source strings.
# No client needs to read a Lyric value using its own language's reader.

(defn! vrs/editor_bind_services (services)
  "Prepare literal editor imports, preserving existing session bindings."
  # Snapshot once: later source imports win for newly introduced names.
  (def existing (eval_global '(ls_env)))
  (map services (fn (service)
    (map (info_srv service :interface_doc) (fn (record)
      (def exported (symbol (get (get record :interface) 0)))
      (if (not? (contains? existing exported))
        (eval_global (vrs/service_stub_form service record)))))))
  # Import completion providers only where the service's binding is in use.
  (map services (fn (service)
    (def completions (info_srv service :entity_completions))
    (import_entity_completions service
      (apply concat (map (vrs/record_fields completions) (fn (pair)
        (list (get pair 0)
          (filter (get pair 1) (fn (provider)
            (def metadata (try (meta (eval_global provider))))
            (if (err? metadata) false (eq? (get metadata :service) service)))))))))))
  :ok)

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

(defn! vrs/editor_services ()
  (map (service_names "") (fn (service) (list (vrs/source service)))))

(defn! vrs/editor_service_functions (service)
  (map (service_interface_functions service "") vrs/editor_function))

(defn! vrs/editor_actions (entity)
  (map (interactive_functions entity) vrs/editor_function))

(defn! vrs/editor_argument (type)
  (map (entities type) vrs/editor_value))
