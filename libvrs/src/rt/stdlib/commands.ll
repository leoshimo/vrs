# Metadata and call helpers shared by native clients and vrsjmp.
# Functions use this process's bindings; services use the registry.
# Completion providers run only when requested.

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

(defn! service_names (query)
  "Search registered services, including those not yet bound in this process."
  (fuzzy_match query (filter (ls_srv) keyword?) display))

(defn! service_interface_functions (service query)
  "Bind SERVICE and search only its current exported interface."
  (bind_srv service)
  (def names (map (info_srv service :interface) (fn (signature)
    (symbol (get signature 0)))))
  (filter (service_functions query) (fn (name) (contains? names name))))

(defn! literal_form (value)
  (if (or! (list? value) (symbol? value)) (list 'quote value) value))

(defn! vrs/command_title (name metadata)
  # A docstring's summary is a label; its remaining lines are documentation.
  (def lines (filter (split "\n" (or! (get metadata :doc) ""))
    (fn (line)
      (not? (empty? (filter (split "" line)
        (fn (char) (not? (contains? '("" " " "\t" "\r") char)))))))))
  (if (empty? lines) (display name) (first lines)))

(defn! command_title (name)
  (vrs/command_title name (meta (eval name))))

(defn! accepts_context? (name entity)
  (def args (get (meta (eval name)) :args))
  (if (empty? args) false
    (eq? (get (get args 0) :type) (get entity 0))))

(defn! interactive_functions (entity)
  "Return bound function names whose first interactive argument matches this entity's tag. Does not run functions or completion providers."
  (if (or! (not? (list? entity)) (not? (keyword? (get entity 0))))
    (error "Expected a tagged entity"))
  (filter (interactive_commands) (fn (name) (accepts_context? name entity))))

(defn! entity_title (entity)
  (if (get entity :title)
    (if (get entity :app) (format "{} — {}" (get entity :app) (get entity :title))
      (get entity :title))
    (display entity)))

(defn! entities (type)
  "Retrieve available entities of a type from its registered sources."
  (def providers (if (eq? type nil) '() (entity_sources type)))
  (def values '())
  (map providers (fn (provider)
    (def found (try (apply (eval provider) '())))
    (if (list? found)
      (map found (fn (entity)
        (when! (and! (list? entity)
                    (eq? (get entity 0) type)
                    (not? (contains? values entity)))
          (set values (push values entity))))))))
  values)

(defn! vrs/execute_command (cmd)
  "Publish a selected command for macro recording, then execute it once."
  (publish :cmd cmd)
  (def result (eval cmd))
  (if (list? result)
    (if (not? (eq? (get result :exit) nil))
      (if (not? (eq? (get result :exit) 0))
        (error (str "Command failed: " (get result :stderr))))))
  result)
