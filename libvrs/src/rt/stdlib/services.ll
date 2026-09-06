# These functions run in the process, after macro expansion. Registry data and
# handler signatures are discovered at runtime; helper names are plain symbols.
(defn vrs/service_dispatch (interface resolve)
  (if (not? (list? interface)) (error ":interface must be a list"))
  (map interface (fn (name)
    (if (not? (symbol? name)) (error ":interface entries must be symbols"))
    (def callable (resolve name))
    (if (not? (lambda? callable)) (error "exported service value must be a lambda"))
    (list :name name :pattern
      (concat (list (keyword name))
        (map (get (meta callable) :args) (fn (arg) (get arg :name))))))))

(defn vrs/dispatch (description resolve message)
  (def choices (filter description (fn (entry) (matches? (get entry :pattern) message))))
  (if (empty? choices)
    '(:err "Unrecognized message")
    (apply (resolve (get (get choices 0) :name)) (slice message 1))))

(defn vrs/service_loop (description resolve)
  # The resolver captures the caller's scope before entering this helper, so
  # these ordinary local names cannot shadow the caller's exported handlers.
  (loop
    (def (request source message) (recv))
    (def response (try (vrs/dispatch description resolve message)))
    (send source (list request response))))

(defn vrs/service_stub_form (service record)
  "Build source for one imported method; does not install or call it."
  (def signature (get record :interface))
  (if (not? (list? signature)) (error "interface record needs a signature"))
  (def message (get signature 0))
  (if (not? (keyword? message)) (error "interface signature needs a selector keyword"))
  (def params (slice signature 1))
  (map params (fn (param)
    (if (not? (symbol? param)) (error "interface parameters must be symbols"))))
  (def doc (get record :doc))
  (def metadata (get record :metadata))
  (if (eq? metadata nil) (set metadata '()))
  `(def ,(symbol message)
     (with_meta
       (lambda ,params ,doc
         (call (find_srv ,service) (list ,message ,@params)))
       ',metadata)))

(defn bind_srv (service)
  "(bind_srv NAME) - Discover current interfaces and install global message-passing stubs."
  (map (info_srv service :interface_doc) (fn (record)
    (eval_global (vrs/service_stub_form service record))))
  (import_entity_completions service (info_srv service :entity_completions)))
