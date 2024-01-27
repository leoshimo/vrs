use vrs::{Extern, ProcessId, ProcessResult, Program, Runtime, Val};

#[tokio::test]
async fn script_has_implicit_begin() {
    let runtime = Runtime::new("test");
    let handle = runtime
        .run(Program::from_script("(def answer 40)\n(+ answer 2)").unwrap())
        .await
        .unwrap();
    assert_eq!(
        handle.join().await.unwrap().status.unwrap(),
        ProcessResult::Done(Val::Int(42))
    );
}

#[tokio::test]
async fn run_uses_a_fresh_process_environment() {
    let path = std::env::temp_dir().join(format!(
        "vrs-run-test-{}-{}.ll",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, "(def value 99)\nvalue").unwrap();

    let expression = format!("(begin (def value 1) (run \"{}\") value)", path.display());
    let runtime = Runtime::new("test");
    let handle = runtime
        .run(Program::from_expr(&expression).unwrap())
        .await
        .unwrap();
    assert_eq!(
        handle.join().await.unwrap().status.unwrap(),
        ProcessResult::Done(Val::Int(1))
    );

    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn node_name_and_process_id_use_the_runtime_node() {
    let runtime = Runtime::new("node");
    let handle = runtime
        .run(Program::from_expr("(list (node_name) (self))").unwrap())
        .await
        .unwrap();
    assert_eq!(
        handle.join().await.unwrap().status.unwrap(),
        ProcessResult::Done(Val::List(vec![
            Val::String("node".to_string()),
            Val::Extern(Extern::ProcessId(ProcessId::new("node", 0))),
        ]))
    );
}
