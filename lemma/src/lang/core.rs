//! Defines opt-in language constructs
//! Lemma interpreter does not have built-in procedures and special forms by default.
//! The language features are "opt in" by defining symbols within the environment

// #[cfg(test)]
// mod tests {

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
// }
