(defn! vrs/service_stub_form (service record)
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
  (set metadata (+ (list :service service) metadata))
  `(def ,(symbol message)
     (with_meta
       (lambda ,params ,doc
         (call (find_srv ,service) (list ,message ,@params)))
       ',metadata)))

(defn! bind_srv (service)
  "(bind_srv NAME) - Discover current interfaces and install global message-passing stubs."
  (map (info_srv service :interface_doc) (fn (record)
    (eval_global (vrs/service_stub_form service record))))
  (import_entity_completions service (info_srv service :entity_completions)))
