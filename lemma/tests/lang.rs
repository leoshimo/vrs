//! Tests for implementation of language

use assert_matches::assert_matches;
use lemma::fiber::{Fiber, FiberState};
use lemma::{Error, NativeFn, Result, SymbolId, Val};

// Convenience to eval top-level expr
fn eval_expr(e: &str) -> Result<Val> {
    let mut f = Fiber::from_expr(e)?;

    // TODO: Replace with real add?
    f.bind(NativeFn {
        symbol: SymbolId::from("+"),
        func: |x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(Val::Int(a + b)),
            _ => panic!("only supports ints"),
        },
    });
    f.bind(NativeFn {
        symbol: SymbolId::from("echo_args"),
        func: |x| Ok(Val::List(x.to_vec())),
    });

    // TODO: Think about ergonomics here
    let res = match f.resume()? {
        FiberState::Done(res) => res,
        FiberState::Yield(_) => panic!("fiber is not complete"),
    };

    if !f.is_stack_empty() {
        panic!("fiber completed with nonempty stack");
    }

    Ok(res)
}

#[test]
fn booleans() {
    assert_eq!(eval_expr("true").unwrap(), Val::Bool(true));
    assert_eq!(eval_expr("false").unwrap(), Val::Bool(false));
}

#[test]
fn int() {
    assert_eq!(eval_expr("5").unwrap(), Val::Int(5));
}

#[test]
fn string() {
    assert_eq!(eval_expr("\"hello\"").unwrap(), Val::string("hello"));
}

#[test]
fn symbols() {
    let prog = r#"
             (begin (def greeting "Hello world")
                    greeting)
        "#;
    assert_eq!(eval_expr(prog).unwrap(), Val::string("Hello world"));
}

#[test]
fn symbols_undefined() {
    assert_matches!(eval_expr("greeting"), Err(Error::UndefinedSymbol(_)));
}

#[test]
fn func_def_call() {
    let prog = r#"
             (begin (def echo (lambda (x) x))
                    (echo "Hello world"))
        "#;
    assert_eq!(eval_expr(prog).unwrap(), Val::string("Hello world"));
}

#[test]
fn begin_block() {
    assert_eq!(eval_expr("(begin 1 2 3 4 5)").unwrap(), Val::Int(5));
}

#[test]
fn begin_block_nested() {
    assert_eq!(
        eval_expr("(begin 1 (begin 2 (begin 3 (begin 4 (begin 5)))))").unwrap(),
        Val::Int(5)
    );
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
    assert_eq!(eval_expr(prog).unwrap(), Val::keyword("lexical"));
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
    assert_eq!(eval_expr(prog).unwrap(), Val::keyword("lexical"));
}

#[test]
fn lambda() {
    let prog = "(lambda (x) x)";
    assert_matches!(
    eval_expr(prog).unwrap(),
        Val::Lambda(l) if l.params == vec![SymbolId::from("x")]
    );
}

#[test]
fn lambda_func_call() {
    let prog = "((lambda (x) x) :echo)";
    assert_eq!(eval_expr(prog).unwrap(), Val::keyword("echo"));
}

#[test]
fn lambda_nested() {
    let prog = r#"
         (((lambda () (lambda (x) x))) 10)
    "#;
    assert_eq!(eval_expr(prog).unwrap(), Val::Int(10));
}

#[test]
fn def() {
    assert_eq!(eval_expr("(def x 5)").unwrap(), Val::Int(5));
}

#[test]
fn def_lambda() {
    let prog = r#"(def echo (lambda (x) x))"#;
    assert_matches!(
        eval_expr(prog).unwrap(),
        Val::Lambda(l) if l.params == vec![SymbolId::from("x")]
    );
}

#[test]
fn nested_func_call() {
    let prog = r#"
             (begin (def echo (lambda (x) x))
                    (echo (echo (echo (echo "hi")))))
        "#;
    assert_eq!(eval_expr(prog).unwrap(), Val::string("hi"));
}

#[test]
fn native_bindings() {
    assert_eq!(
        eval_expr("(echo_args :one \"two\" '(:three))").unwrap(),
        Val::List(vec![
            Val::keyword("one"),
            Val::string("two"),
            Val::List(vec![Val::keyword("three")]),
        ])
    );
}

#[test]
fn adder() {
    let prog = r#"
        (begin
            (def make-addr (lambda (x) (lambda (y) (+ y x))))
            (def add2 (make-addr 2))
            (add2 40))
    "#;
    assert_eq!(eval_expr(prog).unwrap(), Val::Int(42));
}

#[test]
fn eval_quote() {
    {
        assert_eq!(
            eval_expr("(quote (one :two three))").unwrap(),
            Val::List(vec![
                Val::symbol("one"),
                Val::keyword("two"),
                Val::symbol("three"),
            ])
        );
    }
    {
        assert_eq!(
            eval_expr("(quote (lambda (x) x))").unwrap(),
            Val::List(vec![
                Val::symbol("lambda"),
                Val::List(vec![Val::symbol("x")]),
                Val::symbol("x"),
            ])
        );
    }
    {
        assert_matches!(
            eval_expr("((quote (lambda (x) x)) 5)"),
            Err(Error::UnexpectedStack(s)) if s == "Missing function object",
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

    assert_eq!(eval_expr(prog).unwrap(), Val::Int(15),);
}

// TODO(test): Test using def referencing var in parent scope, e.g. (def count (+ count 1)) w/ sequence in eval_defn

#[test]
fn eval_if() {
    assert_eq!(
        eval_expr("(if true \"true\" \"false\")").unwrap(),
        Val::string("true")
    );

    assert_eq!(
        eval_expr("(if false \"true\" \"false\")").unwrap(),
        Val::string("false")
    );
}

#[test]
fn eval_if_cond_symbol() {
    let t_prog = r#"
        (begin
            (def is_true true)
            (if is_true "got true" "got false")
        )
    "#;
    assert_eq!(eval_expr(t_prog), Ok(Val::string("got true")));

    let f_prog = r#"
        (begin
            (def is_false false)
            (if is_false "got true" "got false")
        )
    "#;
    assert_eq!(eval_expr(f_prog), Ok(Val::string("got false")));
}

#[test]
fn eval_if_cond_lambda() {
    let t_prog = r#"
        (begin
            (def is_true (lambda () true))
            (if (is_true) "got true" "got false")
        )
    "#;
    assert_eq!(eval_expr(t_prog), Ok(Val::string("got true")));

    let f_prog = r#"
        (begin
            (def is_false (lambda () false))
            (if (is_false) "got true" "got false")
        )
    "#;
    assert_eq!(eval_expr(f_prog), Ok(Val::string("got false")));
}

#[test]
fn eval_if_body_begin() {
    let t_prog = r#"
        (begin 
            (def n 0)
            (if true
                (begin
                    (set n (+ n 1))
                    (set n (+ n 1))
                    (set n (+ n 1))
                    n)
                (begin
                    (set n (+ n 2))
                    (set n (+ n 2))
                    (set n (+ n 2))
                    n)))
    "#;
    assert_eq!(eval_expr(t_prog), Ok(Val::Int(3)));

    let f_prog = r#"
        (begin 
            (def n 0)
            (if false
                (begin
                    (set n (+ n 1))
                    (set n (+ n 2))
                    (set n (+ n 3))
                    n)
                (begin
                    (set n (+ n 2))
                    (set n (+ n 2))
                    (set n (+ n 2))
                    n)))
    "#;
    assert_eq!(eval_expr(f_prog), Ok(Val::Int(6)));
}

// TODO: Test - if with blocks

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
