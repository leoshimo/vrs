use vrs::{ProcessResult, Program, Runtime, Val};

async fn eval(source: &str) -> Val {
    let rt = Runtime::new("test");
    let result = rt
        .run(Program::from_script(source).unwrap())
        .await
        .unwrap()
        .join()
        .await
        .unwrap();
    let ProcessResult::Done(value) = result.status.unwrap() else {
        panic!("program failed")
    };
    value
}

#[tokio::test]
async fn fuzzy_preserves_objects_and_stable_equal_labels() {
    assert_eq!(
        eval(
            r#"
        (fuzzy_match "saf" '((:title "Safari" :id 1) (:title "Notes" :id 2)
                            (:title "Safari" :id 3))
                     (fn (item) (get item :title)))
    "#
        )
        .await,
        Val::from_expr(r#"((:title "Safari" :id 1) (:title "Safari" :id 3))"#).unwrap()
    );
}

#[tokio::test]
async fn fuzzy_empty_query_preserves_order_and_errors_propagate() {
    assert_eq!(
        eval(r#"(fuzzy_match "" '(b a c) display)"#).await,
        Val::from_expr("(b a c)").unwrap()
    );
    assert_eq!(
        eval(r#"(err? (try (fuzzy_match "a" '(1) (fn (it) it))))"#).await,
        Val::Bool(true)
    );
    assert_eq!(
        eval(r#"(fuzzy_match "unknown" '(a b) display)"#).await,
        Val::List(vec![])
    );
}

#[tokio::test]
async fn apply_preserves_data_and_supports_async_functions() {
    assert_eq!(
        eval(r#"(apply list '(name (:os/window :id 7) (not_a_call)))"#).await,
        Val::from_expr("(name (:os/window :id 7) (not_a_call))").unwrap()
    );
    assert_eq!(eval("(apply sleep '(0)) 42").await, Val::Int(42));
    assert_eq!(
        eval("(err? (try (apply (fn (x) x) '())))").await,
        Val::Bool(true)
    );
}
