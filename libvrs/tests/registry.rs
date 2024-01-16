//! E2E Tests involving Process Registry

use assert_matches::assert_matches;
use std::time::Duration;
use tokio::time::timeout;
use vrs::{Error, Extern, Program, Runtime, Val};

#[tokio::test]
async fn list_services_empty() {
    let rt = Runtime::new();

    let prog = Program::from_expr("(ls_srv)").unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let val = hdl.join().await.unwrap().status.unwrap().unwrap();
    assert_eq!(val, Val::List(vec![]));
}

#[tokio::test]
async fn list_services() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    let srv_c = Program::from_expr(
        r#"(begin
        (defn ping (x) x)
        (defn pong (y) y)
        (register :service_c :interface '(ping pong))
        (recv)
    )"#,
    )
    .unwrap();
    let srv_c = rt.run(srv_c).await.unwrap();

    let prog = Program::from_expr("(ls_srv)").unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let val = hdl.join().await.unwrap().status.unwrap().unwrap();

    let svcs = match val {
        Val::List(v) => v,
        _ => panic!("Expected list as result"),
    };

    assert_eq!(svcs.len(), 6);
    assert!(svcs.contains(&Val::List(vec![
        Val::keyword("name"),
        Val::keyword("service_a"),
        Val::keyword("pid"),
        Val::Extern(Extern::ProcessId(srv_a.id())),
    ]),));
    assert!(svcs.contains(&Val::List(vec![
        Val::keyword("name"),
        Val::keyword("service_b"),
        Val::keyword("pid"),
        Val::Extern(Extern::ProcessId(srv_b.id())),
    ])));
    assert!(
        svcs.contains(&Val::List(vec![
            Val::keyword("name"),
            Val::keyword("service_c"),
            Val::keyword("pid"),
            Val::Extern(Extern::ProcessId(srv_c.id())),
            Val::keyword("interface"),
            Val::List(vec![
                Val::List(vec![Val::keyword("ping"), Val::symbol("x")]),
                Val::List(vec![Val::keyword("pong"), Val::symbol("y")]),
            ])
        ])),
        "Register should expand interface argument of register into lambda signatures"
    );
}

#[tokio::test]
async fn find_service() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let _ = rt.run(srv_b).await.unwrap();

    let prog = Program::from_expr("(find_srv :service_a)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let val = hdl.join().await.unwrap().status.unwrap().unwrap();
    assert_eq!(val, Val::Extern(Extern::ProcessId(srv_a.id())),);
}

#[tokio::test]
#[tracing_test::traced_test]
async fn find_service_dropped() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let _ = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    // Send message to service_b
    let prog = Program::from_expr("(send (find_srv :service_b) :hi)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let _ = hdl.join().await.unwrap().status.unwrap().unwrap();

    let _ = timeout(Duration::from_secs(0), srv_b.join())
        .await
        .expect("srv_b should terminate");

    // find_srv should return Nil after message
    let prog = Program::from_expr("(find_srv :service_b)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let val = hdl.join().await.unwrap().status;
    assert_matches!(
        val,
        Err(Error::EvaluationError(lyric::Error::Runtime(s))) if s == "No service found for :service_b",
        "unknown services return error"
    );
}

#[tokio::test]
async fn find_service_unknown() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let _ = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let _ = rt.run(srv_b).await.unwrap();

    let prog = Program::from_expr("(find_srv :unknown)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let val = hdl.join().await.unwrap().status;

    assert_matches!(
        val,
        Err(Error::EvaluationError(lyric::Error::Runtime(s))) if s == "No service found for :unknown",
        "unknown services return error"
    );
}

#[tokio::test]
async fn double_register_fails() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    {
        let exit_status = srv_b.join().await.unwrap().status;
        assert_matches!(
            exit_status,
            Err(Error::EvaluationError(lyric::Error::Runtime(s))) if s.ends_with("Registered process exists for :service_a"),
            "svc_b should fail with error"
        );
    }

    {
        let prog = Program::from_expr("(ls_srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();

        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };

        assert_eq!(
            svcs.len(),
            2,
            "only one service should be registered (two entries in association list)"
        );
        assert!(
            svcs.contains(&Val::List(vec![
                Val::keyword("name"),
                Val::keyword("service_a"),
                Val::keyword("pid"),
                Val::Extern(Extern::ProcessId(srv_a.id())),
            ])),
            "The first service should still be registered"
        );
    }
}

#[tokio::test]
async fn overwrite_register_succeeds() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let _srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_a :overwrite) (recv))").unwrap();
    let _srv_b = rt.run(srv_b).await.unwrap();

    // TODO: Validate srv_b overwrote.
    // TOOD: Invest in better test infra to wait for system to "settle" before running validation on final system state
    //     {
    //         let prog = Program::from_expr("(ls_srv)").unwrap();
    //         let hdl = rt.run(prog).await.unwrap();
    //         let val = hdl.join().await.unwrap().status.unwrap().unwrap();
    //         let svcs = match val {
    //             Val::List(v) => v,
    //             _ => panic!("Expected list as result"),
    //         };

    //         assert_eq!(svcs.len(), 1, "only one service should be registered");
    //         dbg!(&svcs);
    //         assert!(
    //             svcs.contains(&Val::List(vec![
    //                 Val::keyword("name"),
    //                 Val::keyword("service_b"),
    //                 Val::keyword("pid"),
    //                 Val::Extern(Extern::ProcessId(srv_b.id())),
    //             ])),
    //             "The second service registration should overwrite first"
    //         );
    //     }
}

#[tokio::test]
async fn registry_updates_after_exit() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    {
        // Baseline
        let prog = Program::from_expr("(ls_srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 4); // double number of services for a_list
    }

    // Message srv_b, which should exit after first msg
    let msg_b = Program::from_expr(&format!("(send (pid {}) :hi)", srv_b.id().inner())).unwrap();
    rt.run(msg_b).await.expect("msg_b should start");

    srv_b.join().await.expect("srv_b should have exited");

    // Verify srv_b is removed from registry
    {
        let prog = Program::from_expr("(ls_srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 2);

        // TODO: Move to hashmap type instead of association list
        assert!(svcs.contains(&Val::keyword("service_a")));
        assert!(svcs.contains(&Val::List(vec![
            Val::keyword("name"),
            Val::keyword("service_a"),
            Val::keyword("pid"),
            Val::Extern(Extern::ProcessId(srv_a.id())),
        ])));
    }
}

#[tokio::test]
async fn registry_updates_after_kill() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    {
        // Baseline
        let prog = Program::from_expr("(ls_srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 4);
    }

    // Kill srv_a
    let kill_a = Program::from_expr(&format!("(kill (pid {}))", srv_a.id().inner())).unwrap();
    rt.run(kill_a).await.expect("kill_a should start");

    srv_a.join().await.expect("srv_a should be killed");

    {
        let prog = Program::from_expr("(ls_srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 2);
        assert!(svcs.contains(&Val::List(vec![
            Val::keyword("name"),
            Val::keyword("service_b"),
            Val::keyword("pid"),
            Val::Extern(Extern::ProcessId(srv_b.id())),
        ])));
    }
}
