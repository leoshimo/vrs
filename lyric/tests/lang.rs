//! Tests for implementation of language

use assert_matches::assert_matches;
use lyric::{Error, NativeFn, NativeFnOp, Result, Signal, SymbolId};
use void::Void;

type Fiber = lyric::Fiber<Void, ()>;
type Val = lyric::Val<Void, ()>;
type Env = lyric::Env<Void, ()>;

// Convenience to eval top-level expr
fn eval_expr(e: &str) -> Result<Val> {
    let mut env = Env::standard();
    env.bind_native(
        SymbolId::from("echo_args"),
        NativeFn {
            doc: "".to_string(),
            func: |_, x| Ok(NativeFnOp::Return(Val::List(x.to_vec()))),
        },
    );

    let mut f = Fiber::from_expr(e, env, ())?;

    // TODO: Think about ergonomics here
    let res = match f.start()? {
        Signal::Done(res) => res,
        Signal::Yield(_) | Signal::Await(_) => panic!("fiber is not complete"),
    };

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
fn begin_empty() {
    assert_eq!(eval_expr("(begin)").unwrap(), Val::Nil)
}

#[test]
fn begin_no_scope() {
    let prog = r#"
        (begin
            (def x :before)
            (def get_x (lambda () x))
            (begin
                (def x :after)
                (get_x)         # should be :after
            )
        )"#;
    assert_eq!(
        eval_expr(prog).unwrap(),
        Val::keyword("after"),
        "begin should not create a new environment"
    );
}

#[test]
fn lexical_scope_vars() {
    let prog = r#"
        (let ()
            (def scope :lexical)
            (def get_scope (lambda () scope))
            (let ()
                (def scope :dynamic)
                (get_scope)  # should be lexical
            )
        )"#;
    assert_eq!(eval_expr(prog).unwrap(), Val::keyword("lexical"));
}

#[test]
fn lexical_scope_funcs() {
    let prog = r#"
             (let () (def get_scope (lambda () :lexical))
                    (def calls-get_scope (lambda () (get_scope)))
                    (let ()
                        (def get_scope (lambda () :dynamic))
                        (calls-get_scope)  # should be lexical
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
            (def make_addr (lambda (x) (lambda (y) (+ y x))))
            (def add2 (make_addr 2))
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
            Err(Error::UnexpectedStack(_)),
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
        eval_expr("(if true \"true\")").unwrap(),
        Val::string("true"),
    );

    assert_eq!(eval_expr("(if false \"true\")").unwrap(), Val::Nil);

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
fn cond_empty() {
    assert_eq!(eval_expr("(cond)"), Ok(Val::Nil));
}

#[test]
fn cond_simple() {
    assert_eq!(
        eval_expr(
            r#"
            (cond (true :true))
        "#
        ),
        Ok(Val::keyword("true"))
    );

    assert_eq!(
        eval_expr(
            r#"
            (cond (false :false))
        "#
        ),
        Ok(Val::Nil)
    );

    assert_eq!(
        eval_expr(
            r#"
            (cond
                (true :true)
                (false :false))
        "#
        ),
        Ok(Val::keyword("true"))
    );

    assert_eq!(
        eval_expr(
            r#"
            (cond
                (false :false)
                (true :true))
        "#
        ),
        Ok(Val::keyword("true"))
    );
}

#[test]
fn cond_multi() {
    {
        let prog = r#"(begin
            (defn categorize (x)
                (cond
                    ((eq? x 10) "is int ten")
                    ((eq? x "ten") "is string ten")
                    ((eq? x :ten) "is keyword ten")
                    ((eq? x 3) "is int three")
                    (true "unrecognized")))
            (list (categorize 10) (categorize "ten") (categorize :ten) (categorize 3) (categorize nil) (categorize "jibberish")))
            "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::List(vec![
                Val::string("is int ten"),
                Val::string("is string ten"),
                Val::string("is keyword ten"),
                Val::string("is int three"),
                Val::string("unrecognized"),
                Val::string("unrecognized"),
            ]))
        );
    }

    {
        let prog = r#"(begin
            (defn categorize (x)
                (cond
                    ((eq? x 10) "is int ten")
                    ((eq? x "ten") "is string ten")
                    ((eq? x :ten) "is keyword ten")
                    ((eq? x 3) "is int three")))
            (list (categorize 10) (categorize "ten") (categorize :ten) (categorize 3) (categorize nil) (categorize "jibberish")))
            "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::List(vec![
                Val::string("is int ten"),
                Val::string("is string ten"),
                Val::string("is keyword ten"),
                Val::string("is int three"),
                Val::Nil,
                Val::Nil,
            ]))
        );
    }
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

#[test]
fn eval_let() {
    let prog = r#"
        (let ((a 40)
            (b 2))
            (+ a b))
    "#;

    assert_eq!(eval_expr(prog), Ok(Val::Int(42)));
}

#[test]
fn eval_let_nested() {
    let prog = r#"
        (let ((a 10))
            (let ((b (+ a 20)))
                (let ((c (+ (+ a b) 30)))
                    (def d 40)
                    (+ c d))))
    "#;
    assert_eq!(eval_expr(prog), Ok(Val::Int(110)));
}

#[test]
fn try_deep_callframes() {
    {
        let prog = r#"
            (try ((lambda () (begin
                    (+ 1 1)
                    unknown_var
                    (+ 1 1)))))
        "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::Error(Error::UndefinedSymbol(SymbolId::from(
                "unknown_var"
            ))))
        );
    }

    {
        let prog = r#"
            (try
                ((lambda () (begin
                    (+ 1 1)
                    (unknown_func)
                    (+ 1 1)))))
        "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::Error(Error::UndefinedSymbol(SymbolId::from(
                "unknown_func"
            ))))
        );
    }

    {
        let prog = r#"
            (try (+ (+ 1 (+ 1 (+ 1 unknown_var)))))
        "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::Error(Error::UndefinedSymbol(SymbolId::from(
                "unknown_var"
            ))))
        );
    }
}

#[test]
fn try_in_try() {
    {
        let prog = r#"
            (try (try
                (begin
                    (+ 1 1)
                    unknown_var
                    (+ 1 1))))
        "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::Error(Error::UndefinedSymbol(SymbolId::from(
                "unknown_var"
            ))))
        );
    }

    {
        let prog = r#"
            (try (try
                (begin
                    (+ 1 1)
                    (unknown_func)
                    (+ 1 1))))
        "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::Error(Error::UndefinedSymbol(SymbolId::from(
                "unknown_func"
            ))))
        );
    }

    {
        let prog = r#"
            (try (try (+ (+ 1 (+ 1 (+ 1 unknown_var))))))
        "#;
        assert_eq!(
            eval_expr(prog),
            Ok(Val::Error(Error::UndefinedSymbol(SymbolId::from(
                "unknown_var"
            ))))
        );
    }
}

#[test]
fn refs() {
    let r1 = eval_expr("(ref)");
    let r2 = eval_expr("(ref)");
    assert_matches!(r1, Ok(Val::Ref(_)));
    assert_matches!(r2, Ok(Val::Ref(_)));
    assert_ne!(r1, r2, "refs should be unique-ish");
}

#[test]
fn def_destructuring() {
    assert_eq!(eval_expr("(def :ok :ok)").unwrap(), Val::keyword("ok"));
    assert_eq!(eval_expr("(def 10 10)").unwrap(), Val::Int(10));
    assert_eq!(eval_expr("(def \"hi\" \"hi\")").unwrap(), Val::string("hi"));
    assert_eq!(eval_expr("(def _ :ok)").unwrap(), Val::keyword("ok"));
    assert_eq!(eval_expr("(def _ 10)").unwrap(), Val::Int(10));
    assert_eq!(eval_expr("(def _ \"hi\")").unwrap(), Val::string("hi"));
}

#[test]
fn def_destructuring_nested() {
    {
        let prog = r#"(begin
            (def (a b) '(1 2))
            (list a b)
        )"#;
        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![Val::Int(1), Val::Int(2),])
        );
    }

    {
        let prog = r#"(begin
            (def exp '(:ok (1 2) (3 4)))
            (def (:ok a b) exp)
            (list a b)
        )"#;
        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![
                Val::List(vec![Val::Int(1), Val::Int(2),]),
                Val::List(vec![Val::Int(3), Val::Int(4),])
            ])
        );
    }

    {
        let prog = r#"(begin
            (def (a b a)
                '(1 2 1))
            (list a b)
        )"#;
        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![Val::Int(1), Val::Int(2),])
        );
    }

    {
        let prog = r#"(begin
            (def (a b (a b))
                '(1 2 (1 2)))
            (list a b)
        )"#;
        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![Val::Int(1), Val::Int(2),])
        );
    }

    {
        let prog = r#"(begin
            (def (a b)
                '(1 2 3))
        )"#;
        assert_eq!(eval_expr(prog), Err(Error::InvalidPatternMatch));
    }

    {
        let prog = r#"(begin
            (def (a b a)
                '(1 2 3))
        )"#;
        assert_eq!(eval_expr(prog), Err(Error::InvalidPatternMatch));
    }

    {
        let prog = r#"(begin
            (def (a b) '(0 0))
            (try (def (a b a) '(1 2 1)))
            (list a b)
        )"#;
        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![Val::Int(1), Val::Int(2),]),
            "a and b should be updated"
        );

        let prog = r#"(begin
            (def (a b) '(0 0))
            (try (def (a b a) '(1 2 3)))
            (list a b)
        )"#;
        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![Val::Int(0), Val::Int(0),]),
            "a and b should not be updated if match failed"
        );
    }
}

#[test]
fn eval_try() {
    {
        // baseline
        assert_eq!(
            eval_expr("(eval '(+ x x))"),
            Err(Error::UndefinedSymbol(SymbolId::from("x"))),
            "eval propagates errors at top-level"
        );

        // w/ try
        assert_eq!(
            eval_expr("(try (eval '(+ x x)))"),
            Ok(Val::Error(Error::UndefinedSymbol(SymbolId::from("x")))),
            "try propagates error as a value"
        );
    }
    {
        // baseline
        let prog = "(def (a b) '(1 2 3))";
        assert_matches!(eval_expr(prog), Err(Error::InvalidPatternMatch));

        // w/ try
        let prog = "(try (def (a b) '(1 2 3)))";
        assert_matches!(
            eval_expr(prog).expect("eval should succeed"),
            Val::Error(Error::InvalidPatternMatch)
        );
    }
}

#[test]
fn eval_not() {
    assert_eq!(eval_expr("(not true)").unwrap(), Val::Bool(false),);
    assert_eq!(eval_expr("(not false)").unwrap(), Val::Bool(true),);
    assert_eq!(
        eval_expr(
            r#"(begin
            (defn is_true () true)
            (not (is_true)))"#
        )
        .unwrap(),
        Val::Bool(false),
    );
    assert_eq!(
        eval_expr(
            r#"(begin
            (defn is_false () false)
            (not (is_false)))"#
        )
        .unwrap(),
        Val::Bool(true),
    );
}

#[test]
#[tracing_test::traced_test]
fn eval_match() {
    {
        assert_eq!(eval_expr("(match :hi)").unwrap(), Val::Nil,);
    }
    {
        let prog = r#"(begin
            (defn matcher (x)
                (match x
                    (10 "got ten")
                    (20 "got twenty")
                    (_ "got unknown")))
            (list (matcher 10) (matcher 20) (matcher 30) (matcher :hi))
        )
        "#;

        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![
                Val::string("got ten"),
                Val::string("got twenty"),
                Val::string("got unknown"),
                Val::string("got unknown"),
            ])
        );
    }
    {
        let prog = r#"(begin
            (defn matcher (x)
                (match x
                    ((:ok val) val)
                    ((:err val) val)))
            (list (matcher '(:ok "was ok")) (matcher '(:err "was err")) (matcher '(:jibberish 3)))
        )
        "#;

        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![
                Val::string("was ok"),
                Val::string("was err"),
                Val::Nil
            ])
        );
    }
    {
        let prog = r#"(begin
            (defn weird_add (x)
                (match x
                    ((:add x y) (+ x y))
                    (("add" x y) (+ x y))))
            (list (weird_add '(:add 1 2)) (weird_add '("add" 3 4)))
        )
        "#;

        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![Val::Int(3), Val::Int(7),])
        );
    }

    {
        let prog = r#"(begin
            (defn extract (x)
                (match x
                    ((:first (a b c)) a)
                    ((:second (a b c)) b)
                    ((:third (a b c)) c)))
            (list
                (extract '(:first (1 2 3)))
                (extract '(:second (1 2 3)))
                (extract '(:third (1 2 3)))
            )
        )
        "#;

        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![Val::Int(1), Val::Int(2), Val::Int(3),])
        );
    }

    {
        // match w/ nesting
        let prog = r#"(begin
            (defn matcher (x)
                (match x
                    ((a (a (a))) :one)
                    ((a (b (c))) :two)
                    (((a) (a) (a)) :three)
                    (((a) (b) (c)) :four)
                    ((((a) (a) (a))) :five)
                    ((((a) (b) (c))) :six)))
            (list
                (matcher '(1 (1 (1))))
                (matcher '(1 (2 (3))))
                (matcher '((1) (1) (1)))
                (matcher '((1) (2) (3)))
                (matcher '(((1) (1) (1))))
                (matcher '(((1) (2) (3))))
            )
        )
        "#;

        assert_eq!(
            eval_expr(prog).unwrap(),
            Val::List(vec![
                Val::keyword("one"),
                Val::keyword("two"),
                Val::keyword("three"),
                Val::keyword("four"),
                Val::keyword("five"),
                Val::keyword("six"),
            ])
        );
    }
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

// TODO: Test Recursive Fibonacci
