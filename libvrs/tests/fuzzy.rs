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
async fn default_keys_match_display_and_preserve_original_values() {
    for query in ["", "alpha", "7", "missing"] {
        let source = format!(
            r#"(def items '(alpha "alpha" 7 (:title "Alpha" :id 7) (unbound alpha)))
               (eq? (fuzzy_match "{query}" items)
                    (fuzzy_match "{query}" items display))"#
        );
        assert_eq!(eval(&source).await, Val::Bool(true));
    }
    assert_eq!(eval(r#"(fuzzy_match "" '())"#).await, Val::List(vec![]));
    assert_eq!(
        eval(r#"(fuzzy_match "alpha" '((:title "Alpha" :id 1) (:title "Beta" :id 2)))"#).await,
        Val::from_expr(r#"((:title "Alpha" :id 1))"#).unwrap()
    );
    for source in [
        r#"(fuzzy_match "a")"#,
        r#"(fuzzy_match 1 '())"#,
        r#"(fuzzy_match "a" 1)"#,
        r#"(fuzzy_match "a" '() display display)"#,
    ] {
        assert_eq!(
            eval(&format!("(err? (try {source}))")).await,
            Val::Bool(true)
        );
    }
}

#[tokio::test]
async fn fuzzy_preserves_objects_and_stable_equal_labels() {
    assert_eq!(
        eval(
            r#"
        (fuzzy_match "alp" '((:title "Alpha" :id 1) (:title "Beta" :id 2)
                            (:title "Alpha" :id 3))
                     (fn (item) (get item :title)))
    "#
        )
        .await,
        Val::from_expr(r#"((:title "Alpha" :id 1) (:title "Alpha" :id 3))"#).unwrap()
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
async fn exact_fields_win_without_title_only_priority_or_duplicate_rows() {
    assert_eq!(
        eval(r#"(fuzzy_match "" '(("a") ("b" "")) (fn (fields) fields))"#).await,
        Val::from_expr(r#"(("a") ("b" ""))"#).unwrap()
    );
    assert_eq!(eval(r#"
      (def items '((:title "Ghost in the machine" :app "Browser")
                   (:title "Terminal" :app "Ghostty")
                   (:title "Ghostty notes" :app "Notes")))
      (fuzzy_match "ghostty" items (fn (item) (list (get item :title) (get item :app) (display item))))
    "#).await.as_list().unwrap()[0], Val::from_expr(r#"(:title "Terminal" :app "Ghostty")"#).unwrap());
    assert_eq!(
        eval(
            r#"(fuzzy_match "Read Later" '("Save a Page to Read Later…" "Read Later")
          (fn (title) (list title title)))"#
        )
        .await,
        Val::from_expr(r#"("Read Later" "Save a Page to Read Later…")"#).unwrap()
    );
}

#[tokio::test]
async fn excerpts_preserve_text_and_graphemes_and_clip_around_matches() {
    assert_eq!(
        eval(r#"(match_excerpt "needle" "Title\n  <b>needle</b> & body")"#).await,
        Val::from_expr(r#"("Title <b>" (:match "needle") "</b> & body")"#).unwrap()
    );
    assert_eq!(
        eval("(match_excerpt \"cafe\" \"🧑‍💻 cafe\u{301} notes\")").await,
        Val::from_expr("(\"🧑‍💻 \" (:match \"cafe\u{301}\") \" notes\")").unwrap()
    );
    for query in ["", "absent", "!absent"] {
        assert_eq!(
            eval(&format!("(match_excerpt {query:?} \"note\")")).await,
            Val::Nil
        );
    }
    let content = format!("{}needle{}", "x".repeat(200), "z".repeat(200));
    let result = eval(&format!("(match_excerpt \"needle\" {content:?})")).await;
    assert_eq!(
        result,
        Val::List(vec![
            Val::string("…"),
            Val::string(&"x".repeat(32)),
            Val::from_expr("(:match \"needle\")").unwrap(),
            Val::string(&"z".repeat(122)),
            Val::string("…"),
        ])
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
