use std::time::Duration;
use tokio::time::timeout;
use vrs::{ProcessResult, Program, Runtime, Val};

#[tokio::test]
async fn single_process_pubsub() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (publish :my_topic :before_subscribe)
        (subscribe :my_topic)
        (publish :my_topic :after_subscribe)
        (ls-msgs)
    )"#;
    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_secs(0), hdl.join())
        .await
        .expect("shouldn't timeout")
        .unwrap();

    let msgs = match exit.status.unwrap() {
        ProcessResult::Done(Val::List(res)) => res,
        _ => panic!("should be done w/ list"),
    };

    assert!(
        !msgs.contains(&Val::from_expr("(:topic_updated :my_topic :before_subscribe)").unwrap()),
        "should not see published data before subscribe"
    );
    assert!(
        msgs.contains(&Val::from_expr("(:topic_updated :my_topic :after_subscribe)").unwrap()),
        "should see published data after subscribe"
    );
}

#[tokio::test]
async fn two_process_pubsub_child_to_parent() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (subscribe :my_topic)
        (spawn (lambda () (publish :my_topic :from_child)))
        (recv '(:topic_updated _ _))
    )"#;
    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_secs(0), hdl.join())
        .await
        .expect("shouldn't timeout")
        .unwrap();

    let res = match exit.status.unwrap() {
        ProcessResult::Done(res) => res,
        _ => panic!("should be done w/ list"),
    };

    assert_eq!(
        res,
        Val::from_expr("(:topic_updated :my_topic :from_child)").unwrap()
    );
}

#[tokio::test]
async fn two_process_pubsub_parent_to_child() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (def parent (self))

        (spawn (lambda () (begin
            (subscribe :my_topic)
            (send parent :ready)
            (send parent (list :from_child (recv '(:topic_updated _ _)))))))

        (recv :ready)
        (publish :my_topic :from_parent)

        (recv)
    )"#;
    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_secs(0), hdl.join())
        .await
        .expect("shouldn't timeout")
        .unwrap();

    let res = match exit.status.unwrap() {
        ProcessResult::Done(res) => res,
        _ => panic!("should be done w/ list"),
    };

    assert_eq!(
        res,
        Val::from_expr("(:from_child (:topic_updated :my_topic :from_parent))").unwrap()
    );
}

#[tokio::test]
async fn two_process_pubsub_child_loopback() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (def parent (self))

        # spawn child
        (spawn (lambda () (begin
            (def result '())
            (subscribe :my_topic)
            (send parent :ready)
            (loop
                (def ev (recv '(:topic_updated _ _)))
                (set result (push result ev))
                (send parent result)))))
    
        (recv :ready)

        (publish :my_topic :one)
        (publish :my_topic :two)
        (publish :my_topic :three)

        # match when all three is received
        (recv '((:topic_updated _ _) (:topic_updated _ _) (:topic_updated _ _))) 
    )"#;

    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_secs(10), hdl.join())
        .await
        .expect("shouldn't timeout")
        .unwrap();

    let res = match exit.status.unwrap() {
        ProcessResult::Done(res) => res,
        _ => panic!("should be done w/ list"),
    };

    assert_eq!(
        res,
        Val::from_expr(
            "(
            (:topic_updated :my_topic :one)
            (:topic_updated :my_topic :two)
            (:topic_updated :my_topic :three)
            )"
        )
        .unwrap()
    );
}

#[tokio::test]
async fn two_process_pubsub_many() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (def parent (self))

        # spawn child
        (spawn (lambda () (begin
            (def results '())
            (subscribe :my_topic)
            (send parent :child_ready)  # notify parent that child has subscribed
            (loop (match (recv '(topic_updated _ _))
                ((_ t :done) (send parent (list :child_results results)))
                ((_ t val) (set results (push results (list t val))))
                (_ (error "Unexpected result"))
            )))))

        (recv :child_ready)

        (publish :my_topic :one)
        (publish :my_topic :two)
        (publish :my_topic :three)

        (publish :not_my_topic :one)
        (publish :not_my_topic :two)
        (publish :not_my_topic :three)

        (publish :my_topic :done)

        (recv '(:child_results _))
    )"#;
    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_secs(0), hdl.join())
        .await
        .expect("shouldn't timeout")
        .unwrap();

    let res = match exit.status.unwrap() {
        ProcessResult::Done(res) => res,
        _ => panic!("should be done w/ list"),
    };

    assert_eq!(
        res,
        Val::from_expr(
            "(:child_results (
                (:my_topic :one)
                (:my_topic :two)
                (:my_topic :three)))"
        )
        .unwrap()
    );
}
