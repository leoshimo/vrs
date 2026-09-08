# Metadata and call helpers shared by native clients and vrsjmp.
# Discovery inspects this process's bindings; providers run only when requested.

(defn! interactive_commands ()
  (filter (ls_env) (fn (name) (eq? (get (meta (eval name)) :interactive) true))))

(defn! call_form (name arguments)
  "Fill remaining positions with the function's actual parameter names."
  (concat (list name) arguments
          (map (slice (get (meta (eval name)) :args) (len arguments))
               (fn (arg) (get arg :name)))))

(defn! choice_label (value)
  (def label (try (str (if (list? value) (entity_title value) value))))
  (if (or! (err? label) (eq? label "")) (display value) label))

(defn! service_functions (query)
  "Search bound service methods by name, documentation, or service."
  (def names (filter (ls_env) (fn (name)
    (def value (eval name))
    (and! (lambda? value) (keyword? (get (meta value) :service))))))
  (fuzzy_match query names (fn (name)
    (def metadata (meta (eval name)))
    (list (display name) (or! (get metadata :doc) "")
          (display (get metadata :service))))))

(defn! literal_form (value)
  (if (or! (list? value) (symbol? value)) (list 'quote value) value))

(defn! command_title (name)
  (def metadata (meta (eval name)))
  (or! (get metadata :doc) (display name)))

(defn! accepts_context? (name entity)
  (def args (get (meta (eval name)) :args))
  (if (empty? args) false
    (eq? (get (get args 0) :type) (get entity 0))))

(defn! entity_title (entity)
  (if (get entity :title)
    (if (get entity :app) (format "{} — {}" (get entity :app) (get entity :title))
      (get entity :title))
    (display entity)))

(defn! argument_entities (type)
  "Shared argument choices for interactive execution and call construction."
  (def providers (if (eq? type nil) '() (get_entity_completions type)))
  (def entities '())
  (map providers (fn (provider)
    (def found (try (apply (eval provider) '())))
    (if (list? found)
      (map found (fn (entity)
        (when! (and! (list? entity)
                    (eq? (get entity 0) type)
                    (not? (contains? entities entity)))
          (set entities (push entities entity))))))))
  entities)

(defn! vrs/execute_command (cmd)
  "Publish a selected command for macro recording, then execute it once."
  (publish :cmd cmd)
  (def result (eval cmd))
  (if (list? result)
    (if (not? (eq? (get result :exit) nil))
      (if (not? (eq? (get result :exit) 0))
        (error (str "Command failed: " (get result :stderr))))))
  result)
