use vrs::{ProcessResult, Program, Runtime, Val};

async fn run(source: &str) -> Val {
    let rt = Runtime::new("test");
    let result = rt.run(Program::from_expr(source).unwrap()).await.unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), result.join())
        .await
        .expect("program should finish without hanging")
        .unwrap();
    match result.status.unwrap() {
        ProcessResult::Done(value) => value,
        other => panic!("{other:?}"),
    }
}
fn value(source: &str) -> Val {
    Val::from_expr(source).unwrap()
}

#[tokio::test]
async fn service_expansion_inspects_runtime_exports_without_starting_service() {
    let result = run("(begin
      (def inspections 0)
      (defn! start (exports)
        (defn! echo (x) x)
        (macroexpand_1 '(srv! (error \"name ran\")
          :interface (begin (set inspections (+ inspections 1)) exports)
          :ready (error \"ready ran\"))))
      (def expansion (start '(echo)))
      (list expansion inspections (ls_srv)))")
    .await;
    let parts = result.as_list().unwrap();
    let rendered = parts[0].to_string();
    assert!(rendered.contains("((:echo x) (echo x))"), "{rendered}");
    assert!(
        rendered.contains("(_ '(:err \"Unrecognized message\"))"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("service_dispatch") && !rendered.contains("resolve"),
        "{rendered}"
    );
    assert_eq!(parts[1], Val::Int(1));
    assert_eq!(parts[2], value("()"));
}

#[tokio::test]
async fn service_macro_preserves_dynamic_arguments_and_global_imports() {
    let result = run("(begin
      (def names 0) (def interfaces 0)
      (defn! start (captured)
        (defn! echo (msg) (list captured msg))
        (spawn_srv! (begin (set names (+ names 1)) :dynamic)
          :interface (begin (set interfaces (+ interfaces 1)) '(echo))))
      (def child (start 42))
      (defn! import () (bind_srv :dynamic))
      (import)
      (list names interfaces (echo 7) (eq? child (find_srv :dynamic))))")
    .await;
    assert_eq!(result, value("(1 1 (42 7) true)"));
}

#[tokio::test]
async fn dispatch_retains_startup_patterns_and_looks_up_current_callable() {
    let result = run("(begin
      (def parent (self))
      (def child (spawn (fn ()
      (defn! handler (arg) (list :before arg))
      (defn! replace () (set handler (fn (arg) (list :after arg))) :ok)
      (defn! duplicate (x x) x)
      (defn! ignored (_) :ignored)
      (srv! :changing :interface '(handler replace duplicate ignored) :ready parent))))
      (recv (list :service_ready child))
      (def target (find_srv :changing))
      (def before (call target '(:handler 1)))
      (call target '(:replace))
      (list before (call target '(:handler 2))
        (call target '(:duplicate 3 3)) (call target '(:duplicate 3 4))
        (call target '(:ignored :anything)) (call target '(:handler))))")
    .await;
    assert_eq!(result,value("((:before 1) (:after 2) 3 (:err \"Unrecognized message\") :ignored (:err \"Unrecognized message\"))"));
}

#[tokio::test]
async fn service_loop_locals_do_not_shadow_handlers_and_interface_is_evaluated_first() {
    let result = run("(begin
      (def parent (self))
      (def child (spawn (fn ()
        (def trace '())
        (def service :names)
        (def interface '(request source message response resolve symbol description))
        (defn! request () :request)
        (defn! source () :source)
        (defn! message () :message)
        (defn! response () :response)
        (defn! resolve () :resolve)
        (defn! symbol () :symbol)
        (defn! description () trace)
        (srv! (begin (set trace (push trace :name)) service)
          :ready (begin (set trace (push trace :ready)) parent)
          :interface (begin (set trace (push trace :interface)) interface)))))
      (recv (list :service_ready child))
      (list (eq? child (find_srv :names))
        (call child '(:request)) (call child '(:source))
        (call child '(:message)) (call child '(:response))
        (call child '(:resolve)) (call child '(:symbol))
        (call child '(:description))))")
    .await;
    assert_eq!(
        result,
        value(
            "(true :request :source :message :response :resolve :symbol (:interface :name :ready))"
        )
    );
}

#[tokio::test]
async fn child_macro_namespace_is_a_snapshot_and_message_data_stays_data() {
    let result = run("(begin
      (defmacro m () 1)
      (def parent (self))
      (spawn (fn ()
        (eval '(defmacro m () 2))
        (send parent (list (m!) (eval '(m!)) '(m!)))))
      (list (recv) (eval '(m!))))")
    .await;
    assert_eq!(result, value("((2 2 (m!)) 1)"));
}

#[tokio::test]
async fn host_lambdas_without_lexical_parents_inherit_process_macros() {
    let worker = Val::Lambda(vrs::Lambda {
        metadata: vec![],
        doc: None,
        params: vec![],
        parent: None,
        code: lyric::compile(&value("(send (find_srv :macro_parent) (eval '(m!)))")).unwrap(),
    });
    let source = Val::List(vec![
        Val::symbol("begin"),
        value("(register :macro_parent)"),
        value("(defmacro m () 42)"),
        Val::List(vec![Val::symbol("spawn"), worker]),
        value("(recv)"),
    ]);
    let rt = Runtime::new("test");
    let handle = rt.run(Program::from_val(source).unwrap()).await.unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), handle.join())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.status.unwrap(), ProcessResult::Done(Val::Int(42)));
}

#[tokio::test]
async fn source_scripts_register_macros_in_order() {
    let rt = Runtime::new("test");
    let program =
        Program::from_script("(defmacro plus_one (x) `(+ ,x 1))\n(plus_one! 41)").unwrap();
    let result = rt.run(program).await.unwrap().join().await.unwrap();
    assert_eq!(result.status.unwrap(), ProcessResult::Done(Val::Int(42)));
}

#[tokio::test]
async fn service_options_preserve_source_and_explicit_ready_presence() {
    let result = run("(list
      (vrs/service_options '(:interface (exports)) false)
      (vrs/service_options '(:interface '() :ready nil) true)
      (vrs/service_options '(:ready nil :interface '()) true))")
    .await;
    assert_eq!(
        result,
        value(
            "((:interface (exports) :ready nil :has_ready false)
      (:interface '() :ready nil :has_ready true)
      (:interface '() :ready nil :has_ready true))"
        )
    );
}

#[tokio::test]
async fn service_macro_option_errors_are_catchable_before_spawning() {
    for name in ["srv!", "spawn_srv!"] {
        assert_eq!(
            run(&format!("(err? (try ({name})))")).await,
            Val::Bool(true)
        );
        for options in [
            "",
            ":interface",
            ":ready (self)",
            ":unknown '()",
            ":interface '() :unknown nil",
            ":interface '() :ready",
            ":interface '() :interface '()",
            ":ready nil :ready nil",
            ":interface '() :ready nil :ready nil",
        ] {
            let result = run(&format!(
                "(begin (def effects 0)
              (def failed (err? (try ({name} (set effects 1) {options}))))
              (list failed effects (ls_srv)))"
            ))
            .await;
            assert_eq!(result, value("(true 0 ())"), "{name} {options}");
        }
    }
    for options in [":interface '() :ready nil", ":ready nil :interface '()"] {
        assert_eq!(
            run(&format!(
                "(list (err? (try (spawn_srv! :bad {options}))) (ls_srv))"
            ))
            .await,
            value("(true ())")
        );
    }
}

#[tokio::test]
async fn generated_patterns_do_not_capture_handler_names_or_match_temporary() {
    let result = run("(begin
      (defn! echo (echo) echo)
      (defn! _expr (x) x)
      (defn! wild (_ _) :ok)
      (spawn_srv! :collisions :interface '(echo _expr wild))
      (def target (find_srv :collisions))
      (list (call target '(:echo 42)) (call target '(:_expr 43))
        (call target '(:wild 1 2))))")
    .await;
    assert_eq!(result, value("(42 43 :ok)"));
}

#[tokio::test]
async fn macros_can_call_async_runtime_functions_and_catch_their_errors() {
    let result = run("(begin
      (defmacro ask ()
        (send (self) 42)
        (recv))
      (defmacro fail () (find_srv :missing_macro_service))
      (list (ask!) (err? (try (fail!))) (when! true :recovered)))")
    .await;
    assert_eq!(result, value("(42 true :recovered)"));
}

#[tokio::test]
async fn service_construction_has_no_legacy_function_bindings() {
    assert_eq!(
        run("(list (contains? (ls_env) 'srv) (contains? (ls_env) 'spawn_srv))").await,
        value("(false false)")
    );
}

#[tokio::test]
async fn defn_macro_uses_the_spawned_process_macro_namespace() {
    let result = run("(begin
      (defn! original () 1)
      (def parent (self))
      (spawn (fn ()
        (defmacro defn (name params & body) `(def ,name (fn ,params 42)))
        (defn! child_function () 2)
        (send parent (list (original) (child_function)))))
      (defn! parent_function () 3)
      (list (recv) (parent_function)))")
    .await;
    assert_eq!(result, value("((1 42) 3)"));
}
