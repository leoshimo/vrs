//! Runtime tests    
use std::time::Duration;
use tokio::time::timeout;
use vrs::{ProcessResult, Program, Runtime};

#[ignore]
#[tokio::test]
async fn spawn_pid_is_different() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (def origin_pid (self))
        (spawn (lambda () (send origin_pid (self))))
        (def spawn_pid (recv))
        (list origin_pid spawn_pid)
    )
    "#;
    let prog: Program = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_millis(5), hdl.join())
        .await
        .expect("shouldn't timeout")
        .unwrap();
    let val = match exit.status.unwrap() {
        ProcessResult::Done(v) => v,
        _ => panic!("should be done"),
    };

    println!("{}", val);
    panic!();
}
