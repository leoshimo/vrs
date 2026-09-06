# Expansion helpers receive source forms, never service registry values.
(for_syntax
  (defn service_options (options allow_ready)
    (if (not? (eq? (len options) 2))
      (if (not? (eq? (len options) 4))
        (error "service macro expects :interface EXPR and optional :ready EXPR")))
    (def found_interface false)
    (def found_ready false)
    (def bindings '())
    (def interface nil)
    (def ready nil)
    (def index 0)
    (map (slice options 0) (fn (ignored)
      (if (not? (eq? index (len options)))
        (begin
          (def key (get options index))
          (def expr (get options (+ index 1)))
          (def name (gensym "option"))
          (cond
            ((eq? key :interface)
              (begin
                (if found_interface (error "duplicate :interface"))
                (set found_interface true)
                (set interface name)))
            ((eq? key :ready)
              (begin
                (if (not? allow_ready) (error "spawn_srv! does not accept :ready"))
                (if found_ready (error "duplicate :ready"))
                (set found_ready true)
                (set ready name)))
            (true (error "unsupported service option")))
          (set bindings (push bindings (list name expr)))
          (set index (+ index 2))))))
    (if (not? found_interface) (error "missing :interface"))
    (list :bindings bindings :interface interface :ready ready :has_ready found_ready)))

(defmacro srv (name & options)
  "(srv! NAME :interface EXPR [:ready PID-EXPR]) - Serve exported runtime lambdas."
  (def parsed (service_options options true))
  (def service (gensym "service"))
  (def resolve (gensym "resolve"))
  (def symbol_arg (gensym "symbol"))
  (def dispatch (gensym "dispatch"))
  (def request (gensym "request"))
  (def source (gensym "source"))
  (def message (gensym "message"))
  (def response (gensym "response"))
  (def ready_forms
    (if (get parsed :has_ready)
      (list `(send ,(get parsed :ready) (list :service_ready (self))))
      '()))
  `(let ((,service ,name) ,@(get parsed :bindings))
     (let ((,resolve (fn (,symbol_arg) (eval ,symbol_arg))))
       (let ((,dispatch (vrs/service_dispatch ,(get parsed :interface) ,resolve)))
         (register ,service :overwrite :interface ,(get parsed :interface))
         ,@ready_forms
         (loop
           (def (,request ,source ,message) (recv))
           (def ,response (try (vrs/dispatch ,dispatch ,resolve ,message)))
           (send ,source (list ,request ,response)))))))

(defmacro spawn_srv (name & options)
  "(spawn_srv! NAME :interface EXPR) - Spawn a service and wait for its registration."
  (def parsed (service_options options false))
  (def service (gensym "service"))
  (def parent (gensym "parent"))
  (def child (gensym "child"))
  `(let ((,service ,name) ,@(get parsed :bindings) (,parent (self)))
     (let ((,child
             (spawn (fn ()
               (try (kill (find_srv ,service)))
               (srv! ,service :interface ,(get parsed :interface) :ready ,parent)))))
       (recv (list :service_ready ,child))
       ,child)))
