use vrs::{ProcessResult, Program, Runtime, Val};

#[tokio::test]
async fn saved_pages_passes_the_requested_limit_to_feedbinctl() {
    // Load the real function definitions without starting the indexer or services.
    let definitions = lyric::parse_script(include_str!("../../scripts/feedbin.ll"))
        .unwrap()
        .into_iter()
        .filter(|form| {
            matches!(form, lyric::Form::List(values)
                if values.first() == Some(&lyric::Form::symbol("defn!")))
        });
    let mut body = vec![Val::symbol("begin")];
    body.extend(definitions.map(Val::from));
    body.extend(
        lyric::parse_script(
            r#"
            (def invocations '())
            (defn! exec (program command operation flag count)
              (set invocations
                (+ invocations (list (list program command operation flag count))))
              (list :exit 0 :stdout "[]"))
            (list (feedbin_saved_pages 3) (feedbin_saved_pages 20) invocations)
            "#,
        )
        .unwrap()
        .into_iter()
        .map(Val::from),
    );

    let runtime = Runtime::new("test");
    let result = runtime
        .run(Program::from_val(Val::List(body)).unwrap())
        .await
        .unwrap()
        .join()
        .await
        .unwrap();

    let expected = lyric::parse(
        r#"(() () (("feedbinctl" "pages" "list" "--limit" "3")
                    ("feedbinctl" "pages" "list" "--limit" "20")))"#,
    )
    .unwrap();
    assert_eq!(result.status.unwrap(), ProcessResult::Done(expected.into()));
}
