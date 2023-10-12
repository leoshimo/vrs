//! Tests for implementation of language

use assert_matches::assert_matches;
use lemma::fiber::{self, Fiber, Status};
use lemma::{Error, NativeFn, SymbolId, Val};

#[test]
fn booleans() {
    let mut f = Fiber::from_expr("true").unwrap();
    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::Bool(true));

    let mut f = Fiber::from_expr("false").unwrap();
    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::Bool(false));

    assert!(f.is_stack_empty());
}

#[test]
fn int() {
    let mut f = Fiber::from_expr("5").unwrap();
    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::Int(5));

    assert!(f.is_stack_empty());
}

#[test]
fn string() {
    let mut f = Fiber::from_expr("\"hello\"").unwrap();
    assert_eq!(
        *fiber::start(&mut f).unwrap().unwrap(),
        Val::string("hello")
    );

    assert!(f.is_stack_empty());
}

#[test]
fn symbols() {
    let prog = r#"
             (begin (def greeting "Hello world")
                    greeting)
        "#;
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_eq!(
        *fiber::start(&mut f).unwrap().unwrap(),
        Val::string("Hello world")
    );

    assert!(f.is_stack_empty());
}

#[test]
fn symbols_undefined() {
    let mut f = Fiber::from_expr("greeting").unwrap();
    assert_matches!(
        *fiber::start(&mut f).unwrap(),
        Status::Completed(Err(Error::UndefinedSymbol(_)))
    );

    assert!(f.is_stack_empty());
}

#[test]
fn func_def_call() {
    let prog = r#"
             (begin (def echo (lambda (x) x))
                    (echo "Hello world"))
        "#;
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_eq!(
        *fiber::start(&mut f).unwrap().unwrap(),
        Val::string("Hello world")
    );

    assert!(f.is_stack_empty());
}

#[test]
fn begin_block() {
    let mut f = Fiber::from_expr("(begin 1 2 3 4 5)").unwrap();
    let status = fiber::start(&mut f).unwrap();

    assert_eq!(*status, Status::Completed(Ok(Val::Int(5))));
    assert!(f.is_stack_empty());
}

#[test]
fn begin_block_nested() {
    let mut f = Fiber::from_expr("(begin 1 (begin 2 (begin 3 (begin 4 (begin 5)))))").unwrap();

    let status = fiber::start(&mut f).unwrap();
    assert_eq!(*status, Status::Completed(Ok(Val::Int(5))));
    assert!(f.is_stack_empty());
}

#[test]
fn lexical_scope_vars() {
    let prog = r#"
             (begin (def scope :lexical)
                    (def get-scope (lambda () scope))
                    (begin
                        (def scope :dynamic)
                        (get-scope)  # should be lexical
                    ))
        "#;
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_eq!(
        *fiber::start(&mut f).unwrap().unwrap(),
        Val::keyword("lexical")
    );

    assert!(f.is_stack_empty());
}

#[test]
fn lexical_scope_funcs() {
    let prog = r#"
             (begin (def get-scope (lambda () :lexical))
                    (def calls-get-scope (lambda () (get-scope)))
                    (begin
                        (def get-scope (lambda () :dynamic))
                        (calls-get-scope)  # should be lexical
                    ))
        "#;
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_eq!(
        *fiber::start(&mut f).unwrap().unwrap(),
        Val::keyword("lexical")
    );

    assert!(f.is_stack_empty());
}

#[test]
fn lambda() {
    let prog = "(lambda (x) x)";
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_matches!(
        fiber::start(&mut f).unwrap().unwrap(),
        Val::Lambda(l) if l.params == vec![SymbolId::from("x")]
    );

    assert!(f.is_stack_empty());
}

#[test]
fn lambda_func_call() {
    let prog = "((lambda (x) x) :echo)";
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_eq!(
        *fiber::start(&mut f).unwrap().unwrap(),
        Val::keyword("echo")
    );

    assert!(f.is_stack_empty());
}

#[test]
fn lambda_nested() {
    let prog = r#"
         (((lambda () (lambda (x) x))) 10)
    "#;
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::Int(10));
}

#[test]
fn def() {
    let mut f = Fiber::from_expr("(def x 5)").unwrap();
    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::Int(5));

    assert!(f.is_stack_empty());
}

#[test]
fn def_lambda() {
    let prog = r#"(def echo (lambda (x) x))"#;
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_matches!(
        fiber::start(&mut f).unwrap().unwrap(),
        Val::Lambda(l) if l.params == vec![SymbolId::from("x")]
    );

    assert!(f.is_stack_empty());
}

#[test]
fn nested_func_call() {
    let prog = r#"
             (begin (def echo (lambda (x) x))
                    (echo (echo (echo (echo "hi")))))
        "#;
    let mut f = Fiber::from_expr(prog).unwrap();
    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::string("hi"));
    assert!(f.is_stack_empty());
}

#[test]
fn native_bindings() {
    let prog = r#"(native-first :one :two :three)"#;
    let mut f = Fiber::from_expr(prog).unwrap();
    f.bind(NativeFn {
        symbol: SymbolId::from("native-first"),
        func: |x| Ok(x[0].clone()),
    });

    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::keyword("one"));
    assert!(f.is_stack_empty());
}

#[test]
fn adder() {
    let prog = r#"
        (begin
            (def make-addr (lambda (x) (lambda (y) (+ y x))))
            (def add2 (make-addr 2))
            (add2 40))
    "#;
    let mut f = Fiber::from_expr(prog).unwrap();

    // TODO: Replace with real add?
    f.bind(NativeFn {
        symbol: SymbolId::from("+"),
        func: |x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(Val::Int(a + b)),
            _ => panic!("only supports ints"),
        },
    });

    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::Int(42));
    assert!(f.is_stack_empty());
}

#[test]
fn eval_quote() {
    {
        let mut f = Fiber::from_expr("(quote (one :two three))").unwrap();
        assert_eq!(
            *fiber::start(&mut f).unwrap().unwrap(),
            Val::List(vec![
                Val::symbol("one"),
                Val::keyword("two"),
                Val::symbol("three"),
            ])
        );
    }
    {
        let mut f = Fiber::from_expr("(quote (lambda (x) x))").unwrap();
        assert_eq!(
            *fiber::start(&mut f).unwrap().unwrap(),
            Val::List(vec![
                Val::symbol("lambda"),
                Val::List(vec![Val::symbol("x")]),
                Val::symbol("x"),
            ])
        );
    }
    {
        let mut f = Fiber::from_expr("((quote (lambda (x) x)) 5)").unwrap();
        assert_matches!(
            fiber::start(&mut f).unwrap(),
            Status::Completed(Err(Error::UnexpectedStack(s))) if s == "Missing function object",
            "Quoted expressions don't evaluate inner forms - no function yet"
        );
    }
}

#[test]
fn eval_defn() {
    let prog = r#"(begin
        (def count 0)
        (defn inc (x)
            (set count (+ count x))
            count)
        (inc 1)
        (inc 2)
        (inc 3)
        (inc 4)
        (inc 5)
    )
    "#;
    let mut f = Fiber::from_expr(prog).unwrap();
    // TODO: Replace with real add?
    f.bind(NativeFn {
        symbol: SymbolId::from("+"),
        func: |x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(Val::Int(a + b)),
            _ => panic!("only supports ints"),
        },
    });

    assert_eq!(*fiber::start(&mut f).unwrap().unwrap(), Val::Int(15),);
}

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_eval() {
//         let mut env = std_env();

//         assert_eq!(eval_expr("(eval (quote 5))", &mut env), Ok(Form::Int(5)));

//         assert_eq!(
//             eval_expr("(eval (quote ((lambda (x) x) 5)))", &mut env),
//             Ok(Form::Int(5))
//         );
//     }
//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_if() {
//         let mut env = std_env();

//         assert_eq!(
//             eval_expr("(if true \"true\" \"false\")", &mut env),
//             Ok(Form::string("true"))
//         );

//         assert_eq!(
//             eval_expr("(if false \"true\" \"false\")", &mut env),
//             Ok(Form::string("false"))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_if_with_symbols() {
//         let mut env = std_env();

//         eval_expr("(def is_true true)", &mut env).unwrap();
//         eval_expr("(def is_false false)", &mut env).unwrap();

//         assert_eq!(
//             eval_expr("(if is_true \"true\" \"false\")", &mut env),
//             Ok(Form::string("true"))
//         );

//         assert_eq!(
//             eval_expr("(if is_false \"true\" \"false\")", &mut env),
//             Ok(Form::string("false"))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_if_with_lambda() {
//         let mut env = std_env();

//         eval_expr("(def is_true (lambda () true))", &mut env).unwrap();
//         eval_expr("(def is_false (lambda () false))", &mut env).unwrap();

//         assert_eq!(
//             eval_expr("(if (is_true) \"true\" \"false\")", &mut env),
//             Ok(Form::string("true"))
//         );

//         assert_eq!(
//             eval_expr("(if (is_false) \"true\" \"false\")", &mut env),
//             Ok(Form::string("false"))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_type() {
//         let mut env = std_env();

//         assert_eq!(eval_expr("(type nil)", &mut env), Ok(Form::keyword("nil")));
//         assert_eq!(
//             eval_expr("(type true)", &mut env),
//             Ok(Form::keyword("bool"))
//         );
//         assert_eq!(
//             eval_expr("(type false)", &mut env),
//             Ok(Form::keyword("bool"))
//         );
//         assert_eq!(eval_expr("(type 1)", &mut env), Ok(Form::keyword("int")));
//         assert_eq!(
//             eval_expr("(type \"one\")", &mut env),
//             Ok(Form::keyword("string"))
//         );
//         assert_eq!(
//             eval_expr("(type :a_keyword)", &mut env),
//             Ok(Form::keyword("keyword"))
//         );
//         assert_eq!(
//             eval_expr("(type (quote ()))", &mut env),
//             Ok(Form::keyword("list"))
//         );
//         assert_eq!(
//             eval_expr("(type (lambda (x) x))", &mut env),
//             Ok(Form::keyword("lambda"))
//         );
//         assert_eq!(
//             eval_expr("(type type)", &mut env),
//             Ok(Form::keyword("nativefn"))
//         );
//         assert_eq!(
//             eval_expr("(type ((lambda (x) x) 5))", &mut env),
//             Ok(Form::keyword("int"))
//         );
//     }

mod list {}
//     use super::*;
//     use crate::eval_expr;
//     use crate::lang::std_env;
//     use tracing_test::traced_test;

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_list() {
//         let mut env = std_env();

//         assert_eq!(eval_expr("(list)", &mut env), Ok(Form::List(vec![])),);

//         assert_eq!(
//             eval_expr("(list 1 2 3)", &mut env),
//             Ok(Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),]))
//         );

//         assert_eq!(
//             eval_expr("(list \"one\" 2 :three)", &mut env),
//             Ok(Form::List(vec![
//                 Form::string("one"),
//                 Form::Int(2),
//                 Form::keyword("three"),
//             ]))
//         );

//         eval_expr("(def echo (lambda (x) x))", &mut env).expect("Should define echo function");

//         assert_eq!(
//             eval_expr("(list (echo \"one\") (echo 2) (echo :three))", &mut env),
//             Ok(Form::List(vec![
//                 Form::string("one"),
//                 Form::Int(2),
//                 Form::keyword("three"),
//             ]))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_len() {
//         let mut env = std_env();

//         assert!(matches!(
//             eval_expr("(len)", &mut env),
//             Err(Error::UnexpectedArguments(_))
//         ));

//         assert!(matches!(
//             eval_expr("(len 0)", &mut env),
//             Err(Error::UnexpectedArguments(_))
//         ));

//         assert_eq!(eval_expr("(len (quote ()))", &mut env), Ok(Form::Int(0)));

//         assert_eq!(
//             eval_expr("(len (quote (1 2 3 4 5)))", &mut env),
//             Ok(Form::Int(5))
//         );

//         assert_eq!(
//             eval_expr("(len (list :one 2 \"three\"))", &mut env),
//             Ok(Form::Int(3))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_get() {
//         let mut env = std_env();

//         assert!(matches!(
//             eval_expr("(get \"hello\" 0)", &mut env),
//             Err(Error::UnexpectedArguments(_))
//         ));

//         assert_eq!(eval_expr("(get (quote ()) 0)", &mut env), Ok(Form::Nil));

//         assert_eq!(
//             eval_expr("(get (quote (1 2 3)) 0)", &mut env),
//             Ok(Form::Int(1)),
//         );

//         assert_eq!(
//             eval_expr("(get (list :one :two :three) 2)", &mut env),
//             Ok(Form::keyword("three"))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_getn() {
//         let mut env = std_env();

//         assert!(matches!(
//             eval_expr("(getn \"hello\" 0)", &mut env),
//             Err(Error::UnexpectedArguments(_))
//         ));

//         assert_eq!(eval_expr("(getn (quote ()) 0)", &mut env), Ok(Form::Nil));

//         assert_eq!(
//             eval_expr("(getn (quote (1 2 3)) 1)", &mut env),
//             Ok(Form::Int(2)),
//         );

//         assert_eq!(
//             eval_expr(
//                 "(getn (list :one \"one\" :two \"two\" :three \"three\") :two)",
//                 &mut env
//             ),
//             Ok(Form::string("two"))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_map() {
//         let mut env = std_env();

//         eval_expr("(def echo (lambda (x) x))", &mut env).unwrap();
//         eval_expr("(def zero (lambda (x) 0))", &mut env).unwrap();

//         assert_eq!(
//             eval_expr("(map (quote ()) (lambda (x) x))", &mut env),
//             Ok(Form::List(vec![]))
//         );

//         assert_eq!(
//             eval_expr("(map (quote (:one \"two\" 3)) echo)", &mut env),
//             Ok(Form::List(vec![
//                 Form::keyword("one"),
//                 Form::string("two"),
//                 Form::Int(3),
//             ]))
//         );

//         assert_eq!(
//             eval_expr("(map (quote (1 2 3)) echo)", &mut env),
//             Ok(Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),]))
//         );

//         assert_eq!(
//             eval_expr("(map (quote (1 2 3)) zero)", &mut env),
//             Ok(Form::List(vec![Form::Int(0), Form::Int(0), Form::Int(0),]))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_push() {
//         let mut env = std_env();

//         eval_expr("(def my_lst (list))", &mut env).unwrap();

//         assert_eq!(
//             eval_expr("(push my_lst 1)", &mut env),
//             Ok(Form::List(vec![Form::Int(1)])),
//         );

//         assert_eq!(
//             eval_expr("my_lst", &mut env),
//             Ok(Form::List(vec![Form::Int(1)])),
//         );

//         assert_eq!(
//             eval_expr("(push my_lst :two)", &mut env),
//             Ok(Form::List(vec![Form::Int(1), Form::keyword("two"),])),
//         );

//         assert_eq!(
//             eval_expr("(push my_lst \"three\")", &mut env),
//             Ok(Form::List(vec![
//                 Form::Int(1),
//                 Form::keyword("two"),
//                 Form::string("three"),
//             ])),
//         );

//         assert_eq!(
//             eval_expr("my_lst", &mut env),
//             Ok(Form::List(vec![
//                 Form::Int(1),
//                 Form::keyword("two"),
//                 Form::string("three"),
//             ])),
//         )
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_pop() {
//         let mut env = std_env();

//         eval_expr("(def lst (list 1 2 3))", &mut env).unwrap();

//         assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Int(3)));

//         assert_eq!(
//             eval_expr("lst", &mut env),
//             Ok(Form::List(vec![Form::Int(1), Form::Int(2),]))
//         );

//         assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Int(2)));

//         assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Int(1)));

//         assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Nil));

//         assert_eq!(eval_expr("lst", &mut env), Ok(Form::List(vec![])));
//     }

mod math {}
//     use super::*;
//     use crate::eval_expr;
//     use crate::lang::std_env;

//     #[test]
//     fn eval_add() {
//         let mut env = std_env();

//         assert_eq!(eval_expr("(+ 3 4)", &mut env), Ok(Form::Int(7)));

//         assert_eq!(
//             eval_expr("(+ (+ 1 2) (+ 3 (+ 4 5)))", &mut env),
//             Ok(Form::Int(15))
//         );
//     }

//     #[test]
//     fn eval_sub() {
//         let mut env = std_env();

//         assert_eq!(eval_expr("(sub 3 4)", &mut env), Ok(Form::Int(-1)));

//         assert_eq!(
//             eval_expr("(sub (sub 1 2) (sub 3 (sub 4 5)))", &mut env),
//             Ok(Form::Int(-5))
//         );
//     }

//     #[test]
//     fn eval_less() {
//         let mut env = std_env();

//         assert_eq!(eval_expr("(< 3 4)", &mut env), Ok(Form::Bool(true)));
//         assert_eq!(eval_expr("(< 500 4)", &mut env), Ok(Form::Bool(false)));
//     }
// }
