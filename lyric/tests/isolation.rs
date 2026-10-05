use lyric::{Inst, Signal};
use std::sync::{Arc, Mutex};

type Val = lyric::Val<String, ()>;
type Env = lyric::Env<String, ()>;
type Fiber = lyric::Fiber<String, ()>;

#[test]
fn fork_copies_captures_inside_bytecode_and_values_with_the_same_root() {
    let mut source = Fiber::from_expr(
        "(begin (def count 0) (fn () (set count (+ count 1))))",
        Env::standard(),
        (),
    )
    .unwrap();
    let Signal::Done(increment) = source.start().unwrap() else {
        panic!("expected function")
    };
    let code = vec![Inst::DebugScope(
        vec![Inst::PushConst(increment.clone()), Inst::CallFunc(0)],
        lyric::source::SourceSite::synthetic("host callback".into()),
    )];
    let payload = Val::List(vec![Val::Bytecode(code), increment]);
    let mut owner = Fiber::fork(
        &vec![Inst::PushConst(payload)],
        source.global_env(),
        None,
        (),
    );
    let Signal::Done(Val::List(mut copied)) = owner.start().unwrap() else {
        panic!("expected list")
    };
    let increment = copied.pop().unwrap();
    let Val::Lambda(ref lambda) = increment else {
        panic!("expected function")
    };
    assert!(Arc::ptr_eq(
        lambda.parent.as_ref().unwrap(),
        owner.global_env()
    ));
    let Val::Bytecode(code) = copied.pop().unwrap() else {
        panic!("expected code")
    };
    let mut first = Fiber::from_bytecode(code, Env::standard(), ());
    assert_eq!(first.start().unwrap(), Signal::Done(Val::Int(1)));
    let mut second = Fiber::from_bytecode(
        vec![Inst::PushConst(increment), Inst::CallFunc(0)],
        Env::standard(),
        (),
    );
    assert_eq!(second.start().unwrap(), Signal::Done(Val::Int(2)));
    assert_eq!(
        source.global_env().lock().unwrap().get(&"count".into()),
        Some(Val::Int(0))
    );
    drop((first, second)); // Host captures are only valid while owner is alive.
    let root = Arc::downgrade(owner.global_env());
    drop(owner);
    assert!(root.upgrade().is_none());
}

#[test]
fn fork_reclaims_new_unreachable_closure_cycles_when_dropped() {
    let source = Arc::new(Mutex::new(Env::standard()));
    let code = vec![Inst::Prepare(
        Val::from_expr("(begin (defn! make () (defn! inner () 42) inner) (make))").unwrap(),
    )];
    let mut fiber = Fiber::fork(&code, &source, None, ());
    let Signal::Done(Val::Lambda(inner)) = fiber.start().unwrap() else {
        panic!("expected function")
    };
    let root = Arc::downgrade(fiber.global_env());
    let scope = Arc::downgrade(inner.parent.as_ref().unwrap());
    drop(inner);
    assert!(
        scope.upgrade().is_some(),
        "local recursive binding forms a cycle"
    );
    drop(fiber);
    assert!(root.upgrade().is_none());
    assert!(scope.upgrade().is_none());
    assert!(source.lock().unwrap().get(&"make".into()).is_none());
}
