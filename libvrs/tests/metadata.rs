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
async fn bind_imports_metadata_and_completions_without_running_provider() {
    let value = eval(
        r#"
        (defn! objects () (error "must not run during registration or binding"))
        (defn! choose (object) (interactive :example/object) object)
        (set_entity_completions :example/object 'objects)
        (spawn_srv! :example :interface '(objects choose))
        (set_entity_completions :example/object nil)
        (bind_srv :example)
        (bind_srv :example)
        (list (get_entity_completions :example/object)
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
async fn defaults_compose_and_local_override_survives_rebinding() {
    let value = eval(
        r#"
        (defn! first_objects () '())
        (defn! second_objects () '())
        (defn! local_objects () '())
        (set_entity_completions :example/object 'first_objects)
        (spawn_srv! :first :interface '(first_objects))
        (set_entity_completions :example/object 'second_objects)
        (spawn_srv! :second :interface '(second_objects))
        (set_entity_completions :example/object nil)
        (bind_srv :first) (bind_srv :second)
        (def defaults (get_entity_completions :example/object))
        (set_entity_completions :example/object 'local_objects)
        (bind_srv :first)
        (def overridden (get_entity_completions :example/object))
        (set_entity_completions :example/object nil)
        (list defaults overridden (get_entity_completions :example/object))
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
        (set_entity_completions :public/object 'public_objects)
        (set_entity_completions :private/object 'private_objects)
        (spawn_srv! :example :interface '(public_objects))
        (set_entity_completions :public/object nil)
        (set_entity_completions :private/object nil)
        (bind_srv :example)
        (def before (get_entity_completions :public/object))
        (def private (get_entity_completions :private/object))
        (defn! ping () :pong)
        (spawn_srv! :example :interface '(ping))
        (bind_srv :example)
        (list before private (get_entity_completions :public/object))
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
        (set_entity_completions :example/object 'parent_objects)
        (spawn (fn ()
          (set_entity_completions :example/object 'child_objects)
          (send parent (get_entity_completions :example/object))))
        (list (recv) (get_entity_completions :example/object))
    "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr("((child_objects) (parent_objects))").unwrap()
    );
}
