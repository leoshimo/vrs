//! Tests for evaluation API
// TODO: Revive tests

mod builtin {}
// use crate::eval::eval;
// use crate::{Env, Error, Form, Result};
//     use super::*;
//     use crate::eval_expr;
//     use crate::lang::std_env;
//     use tracing_test::traced_test;

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_lambda() {
//         let mut env = std_env();

//         assert!(
//             matches!(eval_expr("lambda", &mut env), Ok(Form::NativeFunc(_))),
//             "lambda symbol should be defined"
//         );

//         // assert!(
//         //     matches!(
//         //         eval_expr(
//         //             "(lambda (x y) 10)",
//         //             &mut env
//         //         ),
//         //         Ok(Form::Lambda(Lambda { params, .. })) if params == vec![SymbolId::from("x"), SymbolId::from("y")]
//         //     ),
//         //     "lambda special form returns a lambda value"
//         // );

//         // ((lambda (x) x) 5) => 5
//         assert_eq!(eval_expr("((lambda (x) x) 5)", &mut env), Ok(Form::Int(5)));

//         // ((lambda () (lambda (x) x)))
//         assert!(matches!(
//             eval_expr("((lambda () (lambda (x) x)))", &mut env),
//             Ok(Form::Lambda(_))
//         ));

//         // (((lambda () (lambda (x) x))) 10) => 10
//         assert_eq!(
//             eval_expr("(((lambda () (lambda (x) x))) 10)", &mut env),
//             Ok(Form::Int(10))
//         );
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_quote() {
//         let mut env = std_env();

//         assert_eq!(
//             eval_expr("(quote (one :two three))", &mut env),
//             Ok(Form::List(vec![
//                 Form::symbol("one"),
//                 Form::keyword("two"),
//                 Form::symbol("three"),
//             ]))
//         );

//         assert_eq!(
//             eval_expr("(quote (lambda (x) x))", &mut env),
//             Ok(Form::List(vec![
//                 Form::symbol("lambda"),
//                 Form::List(vec![Form::symbol("x")]),
//                 Form::symbol("x"),
//             ]))
//         );

//         assert!(
//             matches!(
//                 eval_expr("((quote (lambda (x) x)) 5)", &mut env),
//                 Err(Error::NotAProcedure(_))
//             ),
//             "A quoted operation does not recursively evaluate without explicit call to eval"
//         );
//     }

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
//     fn eval_def_vals() {
//         {
//             let mut env = std_env();
//             assert_eq!(eval_expr("(def x 10)", &mut env), Ok(Form::Int(10)));
//         }

//         {
//             let mut env = std_env();
//             assert_eq!(
//                 eval_expr("(def x \"hello\")", &mut env),
//                 Ok(Form::string("hello"))
//             );
//         }

//         {
//             // def + eval
//             let mut env = std_env();
//             assert_eq!(
//                 eval_expr("(def x \"hello\")", &mut env),
//                 Ok(Form::string("hello"))
//             );

//             assert_eq!(
//                 eval_expr("x", &mut env),
//                 Ok(Form::string("hello")),
//                 "x should evaluate to def value"
//             );
//         }
//     }

//     #[test]
//     #[traced_test]
//     #[ignore]
//     fn eval_def_func() {
//         let mut env = std_env();

//         assert!(matches!(
//             eval_expr("(def echo (lambda (x) x))", &mut env),
//             Ok(Form::Lambda(_))
//         ));

//         assert_eq!(
//             eval_expr("(echo \"hello\")", &mut env),
//             Ok(Form::string("hello"))
//         );

//         assert_eq!(
//             eval_expr("(echo (echo \"hello\"))", &mut env),
//             Ok(Form::string("hello"))
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
