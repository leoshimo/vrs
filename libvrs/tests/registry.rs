//! E2E Tests involving Process Registry

use assert_matches::assert_matches;
use vrs::rt::program::Val;
use vrs::{Connection, Error, Extern, Program, Request, Runtime};

#[tokio::test]
async fn list_services_empty() {
    let rt = Runtime::new();

    let prog = Program::from_expr("(ls-srv)").unwrap();
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
        (defn pong (x) x)
        (register :service_c :interface '(ping pong))
        (recv)
    )"#,
    )
    .unwrap();
    let srv_c = rt.run(srv_c).await.unwrap();

    let prog = Program::from_expr("(ls-srv)").unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let val = hdl.join().await.unwrap().status.unwrap().unwrap();

    let svcs = match val {
        Val::List(v) => v,
        _ => panic!("Expected list as result"),
    };

    assert_eq!(svcs.len(), 3);
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
    assert!(svcs.contains(&Val::List(vec![
        Val::keyword("name"),
        Val::keyword("service_c"),
        Val::keyword("pid"),
        Val::Extern(Extern::ProcessId(srv_c.id())),
        Val::keyword("interface"),
        Val::List(vec![Val::symbol("ping"), Val::symbol("pong"),])
    ])));
}

#[tokio::test]
async fn find_service() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let _ = rt.run(srv_b).await.unwrap();

    let prog = Program::from_expr("(find-srv :service_a)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let val = hdl.join().await.unwrap().status.unwrap().unwrap();
    assert_eq!(val, Val::Extern(Extern::ProcessId(srv_a.id())),);
}

#[tokio::test]
async fn find_service_dropped() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let _ = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    // Send message to service_b
    let prog = Program::from_expr("(send (find-srv :service_b) :hi)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let _ = hdl.join().await.unwrap().status.unwrap().unwrap();

    srv_b.join().await.expect("srv_b should terminate");

    // find-srv should return Nil after message
    let prog = Program::from_expr("(find-srv :service_b)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let val = hdl.join().await.unwrap().status.unwrap().unwrap();
    assert_eq!(val, Val::Nil);
}

#[tokio::test]
async fn find_service_unknown() {
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let _ = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let _ = rt.run(srv_b).await.unwrap();

    let prog = Program::from_expr("(find-srv :unknown)").unwrap();
    let hdl = rt.run(prog).await.unwrap();
    let val = hdl.join().await.unwrap().status.unwrap().unwrap();

    assert_eq!(val, Val::Nil, "unknown services return nil");
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
            Err(Error::RegistryError(_)),
            "svc_b should fail with error"
        );
    }

    {
        let prog = Program::from_expr("(ls-srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();

        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };

        assert_eq!(svcs.len(), 1, "only one service should be registered");
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
async fn registry_updates_after_exit() {
    let (conn_rt, mut conn_client) = Connection::pair().unwrap();
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    let _ = rt.handle_conn(conn_rt).await.unwrap();

    {
        // Baseline
        let prog = Program::from_expr("(ls-srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 2);
    }

    // Message srv_b, which should exit
    conn_client
        .send_req(Request {
            req_id: 0,
            contents: lyric::parse(&format!("(send (pid {}) :hi)", srv_b.id().inner())).unwrap(),
        })
        .await
        .unwrap();
    srv_b.join().await.expect("srv_b should have exited");

    {
        let prog = Program::from_expr("(ls-srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 1);
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
    let (conn_rt, mut conn_client) = Connection::pair().unwrap();
    let rt = Runtime::new();

    let srv_a = Program::from_expr("(begin (register :service_a) (recv))").unwrap();
    let srv_a = rt.run(srv_a).await.unwrap();

    let srv_b = Program::from_expr("(begin (register :service_b) (recv))").unwrap();
    let srv_b = rt.run(srv_b).await.unwrap();

    let _ = rt.handle_conn(conn_rt).await.unwrap();

    {
        // Baseline
        let prog = Program::from_expr("(ls-srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 2);
    }

    // Kill srv_a
    conn_client
        .send_req(Request {
            req_id: 0,
            contents: lyric::parse(&format!("(kill (pid {}))", srv_a.id().inner())).unwrap(),
        })
        .await
        .unwrap();
    srv_a.join().await.expect("srv_a should be killed");

    {
        let prog = Program::from_expr("(ls-srv)").unwrap();
        let hdl = rt.run(prog).await.unwrap();
        let val = hdl.join().await.unwrap().status.unwrap().unwrap();
        let svcs = match val {
            Val::List(v) => v,
            _ => panic!("Expected list as result"),
        };
        assert_eq!(svcs.len(), 1);
        assert!(svcs.contains(&Val::List(vec![
            Val::keyword("name"),
            Val::keyword("service_b"),
            Val::keyword("pid"),
            Val::Extern(Extern::ProcessId(srv_b.id())),
        ])));
    }
}
