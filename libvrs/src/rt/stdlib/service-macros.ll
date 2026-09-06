# Ordinary helpers used by the service transformers.
(defn vrs/service_options (options allow_ready)
  (def (interface ready has_ready)
    (match options
      ((:interface interface) (list interface nil false))
      ((:interface interface :ready ready) (list interface ready true))
      ((:ready ready :interface interface) (list interface ready true))
      (_ (error "service macro expects :interface EXPR and optional :ready EXPR"))))
  (if has_ready
    (if (not? allow_ready) (error "spawn_srv! does not accept :ready")))
  (list :interface interface :ready ready :has_ready has_ready))

(defn vrs/service_clauses (interface)
  (if (not? (list? interface)) (error ":interface must be a list"))
  (map interface (fn (name)
    (if (not? (symbol? name)) (error ":interface entries must be symbols"))
    (def callable (eval_caller name))
    (if (not? (lambda? callable)) (error "exported service value must be a lambda"))
    # Keep ordinary parameter names readable. Rename a parameter if it would
    # shadow the handler, and capture wildcard arguments for the direct call.
    (def renamed (gensym "argument"))
    (def params (map (get (meta callable) :args) (fn (arg)
      (def param (get arg :name))
      (cond
        ((eq? param '_) (gensym "argument"))
        ((eq? param name) renamed)
        (true param)))))
    (list (concat (list (keyword name)) params) (concat (list name) params)))))

(defmacro srv (name & options)
  "(srv! NAME :interface EXPR [:ready PID-EXPR]) - Expand a match over the current handlers."
  (def parsed (vrs/service_options options true))
  (def interface (eval_caller (get parsed :interface)))
  (def clauses (vrs/service_clauses interface))
  (def service (gensym "service"))
  (def request (gensym "request"))
  (def source (gensym "source"))
  (def message (gensym "message"))
  (def response (gensym "response"))
  (def ready (gensym "ready"))
  (def ready_bindings
    (if (get parsed :has_ready) (list (list ready (get parsed :ready))) '()))
  (def ready_forms
    (if (get parsed :has_ready)
      (list `(send ,ready (list :service_ready (self))))
      '()))
  `(let ((,service ,name) ,@ready_bindings)
     (register ,service :overwrite :interface ',interface)
     ,@ready_forms
     (loop
       (def (,request ,source ,message) (recv))
       (def ,response
         (try (match ,message
                ,@clauses
                (_ '(:err "Unrecognized message")))))
       (send ,source (list ,request ,response)))))

(defmacro spawn_srv (name & options)
  "(spawn_srv! NAME :interface EXPR) - Spawn a service and wait for its registration."
  (def parsed (vrs/service_options options false))
  (def interface (eval_caller (get parsed :interface)))
  # Validate before spawning so errors reach the caller instead of losing readiness.
  (vrs/service_clauses interface)
  (def service (gensym "service"))
  (def parent (gensym "parent"))
  (def child (gensym "child"))
  `(let ((,service ,name) (,parent (self)))
     (let ((,child
             (spawn (fn ()
               (try (kill (find_srv ,service)))
               (srv! ,service :interface ',interface :ready ,parent)))))
       (recv (list :service_ready ,child))
       ,child)))
