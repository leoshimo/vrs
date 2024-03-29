# Ordinary helpers used by the service transformers.
(defn! vrs/service_option_pairs (options)
  (cond
    ((empty? options) '())
    ((eq? (len options) 1) (error "service option needs a value"))
    (true (+ (list (list (get options 0) (get options 1)))
             (vrs/service_option_pairs (slice options 2))))))

(defn! vrs/service_options (options allow_ready)
  (def interface nil)
  (def topics ''())
  (def ready nil)
  (def seen '())
  (map (vrs/service_option_pairs options) (fn (pair)
    (def (key value) pair)
    (if (contains? seen key) (error "duplicate service option"))
    (set seen (push seen key))
    (match key
      (:interface (set interface value))
      (:topics (set topics value))
      (:ready (begin
        (if (not? allow_ready) (error "spawn_srv! does not accept :ready"))
        (set ready value)))
      (_ (error "service macro expects :interface, :topics, or :ready")))))
  (if (not? (contains? seen :interface)) (error "service macro requires :interface"))
  (list :interface interface :topics topics :ready ready :has_ready (contains? seen :ready)))

(defn! vrs/service_clauses (interface)
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

# Each topic has one named, single-argument handler. Keep payloads as data.
(defn! vrs/topic_clauses (topics service)
  (if (not? (list? topics)) (error ":topics must be a list"))
  (def seen '())
  (map topics (fn (entry)
    (if (not? (list? entry)) (error ":topics entries must be (TOPIC HANDLER)"))
    (if (not? (eq? (len entry) 2)) (error ":topics entries must be (TOPIC HANDLER)"))
    (def (topic handler) entry)
    (if (not? (keyword? topic)) (error "topic must be a keyword"))
    (if (contains? seen topic) (error "duplicate service topic"))
    (set seen (push seen topic))
    (if (not? (symbol? handler)) (error "topic handler must be a symbol"))
    (def callable (eval_caller handler))
    (if (not? (lambda? callable)) (error "topic handler must be a lambda"))
    (if (not? (eq? (len (get (meta callable) :args)) 1))
      (error "topic handler must accept exactly one payload argument"))
    (def payload (gensym "payload"))
    (def result (gensym "result"))
    `((:topic_updated ,topic ,payload) (begin
      (def ,result (try (,handler ,payload)))
      (if (err? ,result) (vrs/report_service_event_error ,service ,topic ,result)))))))

(defmacro srv (name & options)
  "(srv! NAME :interface EXPR [:topics EXPR] [:ready PID-EXPR]) - Serve calls and topic handlers in the current process."
  (def parsed (vrs/service_options options true))
  (def interface (eval_caller (get parsed :interface)))
  (def clauses (vrs/service_clauses interface))
  (def topics (eval_caller (get parsed :topics)))
  (def service (gensym "service"))
  (def topic_clauses (vrs/topic_clauses topics service))
  (def subscriptions (map topics (fn (entry) `(subscribe ,(get entry 0)))))
  (def request (gensym "request"))
  (def source (gensym "source"))
  (def message (gensym "message"))
  (def incoming (gensym "incoming"))
  (def response (gensym "response"))
  (def ready (gensym "ready"))
  (def ready_bindings
    (if (get parsed :has_ready) (list (list ready (get parsed :ready))) '()))
  (def ready_forms
    (if (get parsed :has_ready)
      (list `(send ,ready (list :service_ready (self))))
      '()))
  `(let ((,service ,name) ,@ready_bindings)
     ,@subscriptions
     (register ,service :overwrite :interface ',interface)
     ,@ready_forms
     (loop
       (def ,incoming (recv))
       (match ,incoming
         ,@topic_clauses
         ((:topic_updated _ _) nil)
         ((,request ,source ,message)
          (if (vrs/service_request? ,incoming) (begin
           (def ,response
             (try (match ,message
                    ,@clauses
                    (_ '(:err "Unrecognized message")))))
           # A departed caller must not kill the service.
           (try (send ,source (list ,request ,response))))))
         (_ nil)))))

(defmacro spawn_srv (name & options)
  "(spawn_srv! NAME :interface EXPR [:topics EXPR]) - Spawn a service and wait for subscriptions and registration."
  (def parsed (vrs/service_options options false))
  (def interface (eval_caller (get parsed :interface)))
  # Validate before spawning so errors reach the caller instead of losing readiness.
  (vrs/service_clauses interface)
  (def topics (eval_caller (get parsed :topics)))
  (vrs/topic_clauses topics nil)
  (def service (gensym "service"))
  (def parent (gensym "parent"))
  (def child (gensym "child"))
  `(let ((,service ,name) (,parent (self)))
     (let ((,child
             (spawn (fn ()
               (try (kill (find_srv ,service)))
               (srv! ,service :interface ',interface :topics ',topics :ready ,parent)))))
       (recv (list :service_ready ,child))
       ,child)))
