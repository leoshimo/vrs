use vrs::{ProcessResult, Program, Runtime, Val};

async fn eval(source: &str) -> Val {
    let rt = Runtime::new("metadata-test");
    let result = rt
        .run(Program::from_script(source).unwrap())
        .await
        .unwrap()
        .join()
        .await
        .unwrap();
    let ProcessResult::Done(value) = result.status.unwrap() else {
        panic!()
    };
    value
}

#[tokio::test]
async fn command_titles_use_a_doc_summary_or_the_function_name_after_binding() {
    let value = eval(
        r#"
        (defn! complete_todo (todo) (interactive :todo) (error "must not run"))
        (defn! move_window (window)
          "\n\t\nMove window\nA longer explanation of placement and side effects."
          (interactive :os/window) (error "must not run"))
        (defn! blank_doc () " \t\n\r" (error "must not run"))
        (spawn_srv! :labels :interface '(complete_todo move_window blank_doc))
        (bind_srv :labels)
        (list (command_title 'complete_todo)
              (command_title 'move_window)
              (command_title 'blank_doc)
              (get (meta move_window) :doc)
              (vrs/command_title 'unbound_function '(:doc "Short summary\nMore detail")))
        "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr(
            r#"("complete_todo" "Move window" "blank_doc"
                 "\n\t\nMove window\nA longer explanation of placement and side effects."
                 "Short summary")"#
        )
        .unwrap()
    );
}

#[tokio::test]
async fn bind_imports_metadata_and_completions_without_running_provider() {
    let value = eval(
        r#"
        (defn! objects () (error "must not run during registration or binding"))
        (defn! choose (object) (interactive :example/object) object)
        (register_entity_source :example/object 'objects)
        (spawn_srv! :example :interface '(objects choose))
        (register_entity_source :example/object nil)
        (bind_srv :example)
        (bind_srv :example)
        (list (entity_sources :example/object)
              (get (meta choose) :interactive)
              (get (get (get (meta choose) :args) 0) :type)
              (choose '(:example/object :id 1)))
    "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr("((objects) true :example/object (:example/object :id 1))").unwrap()
    );
}

#[tokio::test]
async fn entities_queries_registered_sources_for_live_values() {
    let value = eval(
        r#"
        (def fetches 0)
        (defn! first_objects ()
          (set fetches (+ fetches 1))
          '((:example/object :id 1) (:other/type :id 7) "not an entity"))
        (defn! second_objects ()
          '((:example/object :id 1) (:example/object :id 2)))
        (register_entity_source :example/object '(first_objects second_objects))
        (def sources (entity_sources :example/object))
        (def before fetches)
        (def found (entities :example/object))
        (defn! first_objects ()
          (set fetches (+ fetches 1))
          '((:example/object :id 3)))
        (list sources before found (entities :example/object)
              (entities :unknown/type) fetches)
    "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr(
            "((first_objects second_objects) 0
              ((:example/object :id 1) (:example/object :id 2))
              ((:example/object :id 3) (:example/object :id 1) (:example/object :id 2))
              () 2)"
        )
        .unwrap()
    );
}

#[tokio::test]
async fn interactive_functions_discovers_current_bindings_without_executing_them() {
    let value = eval(
        r#"
        (defn! objects () (error "provider must not run"))
        (defn! copy_id (object) (interactive :example/object) (error "must not run"))
        (defn! move_object (object destination)
          (interactive :example/object :example/place) (error "must not run"))
        (defn! plain (object) object)
        (defn! reversed (place object)
          (interactive :example/place :example/object) (error "must not run"))
        (register_entity_source :example/object 'objects)
        (spawn_srv! :example :interface '(objects copy_id move_object plain reversed))
        (bind_srv :example)
        (def object '(:example/object :id 7))
        (def found (interactive_functions object))
        (def checks (list (eq? (len found) 2)
                          (contains? found 'copy_id) (contains? found 'move_object)
                          (empty? (interactive_functions '(:unknown/type :id 7)))))
        (defn! copy_id (place) (interactive :example/place) place)
        (list checks
              (interactive_functions object)
              (map (vrs/editor_actions object) (fn (entry) (get entry 0)))
              (map '(nil 42 "text" () (untagged 7))
                   (fn (value) (err? (try (interactive_functions value))))))
    "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr(
            "((true true true true) (move_object) (\"move_object\") (true true true true true))"
        )
        .unwrap()
    );
}

#[tokio::test]
async fn defaults_compose_and_local_override_survives_rebinding() {
    let value = eval(
        r#"
        (defn! first_objects () '())
        (defn! second_objects () '())
        (defn! local_objects () '())
        (register_entity_source :example/object 'first_objects)
        (spawn_srv! :first :interface '(first_objects))
        (register_entity_source :example/object 'second_objects)
        (spawn_srv! :second :interface '(second_objects))
        (register_entity_source :example/object nil)
        (bind_srv :first) (bind_srv :second)
        (def defaults (entity_sources :example/object))
        (register_entity_source :example/object 'local_objects)
        (bind_srv :first)
        (def overridden (entity_sources :example/object))
        (register_entity_source :example/object nil)
        (list defaults overridden (entity_sources :example/object))
    "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr(
            "((first_objects second_objects) (local_objects) (first_objects second_objects))"
        )
        .unwrap()
    );
}

#[tokio::test]
async fn rebinding_replaces_stale_defaults_and_private_providers_are_not_exported() {
    let value = eval(
        r#"
        (defn! public_objects () '())
        (defn! private_objects () '())
        (register_entity_source :public/object 'public_objects)
        (register_entity_source :private/object 'private_objects)
        (spawn_srv! :example :interface '(public_objects))
        (register_entity_source :public/object nil)
        (register_entity_source :private/object nil)
        (bind_srv :example)
        (def before (entity_sources :public/object))
        (def private (entity_sources :private/object))
        (defn! ping () :pong)
        (spawn_srv! :example :interface '(ping))
        (bind_srv :example)
        (list before private (entity_sources :public/object))
    "#,
    )
    .await;
    assert_eq!(value, Val::from_expr("((public_objects) () ())").unwrap());
}

#[tokio::test]
async fn spawned_process_completion_overrides_do_not_taint_the_parent() {
    let value = eval(
        r#"
        (def parent (self))
        (register_entity_source :example/object 'parent_objects)
        (spawn (fn ()
          (register_entity_source :example/object 'child_objects)
          (send parent (entity_sources :example/object))))
        (list (recv) (entity_sources :example/object))
    "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr("((child_objects) (parent_objects))").unwrap()
    );
}
