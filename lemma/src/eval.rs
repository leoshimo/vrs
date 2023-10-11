#![allow(unused_variables, dead_code)]
//! Implements evaluation of expressions
use crate::{Env, Form, Result};

/// Evaluate a given expression
pub fn eval_expr(expr: &str, env: &mut Env) -> Result<Form> {
    Ok(Form::Int(0))
}

// #[cfg(test)]
// mod tests {

//     use super::*;
//     use crate::Form as F;

//     #[test]
//     #[ignore]
//     fn eval_bool() {
//         let mut env = Env::new();
//         assert_eq!(eval_expr("true", &mut env), Ok(F::Bool(true)));
//         assert_eq!(eval_expr("false", &mut env), Ok(F::Bool(false)));
//     }

//     #[test]
//     #[ignore]
//     fn eval_int() {
//         let mut env = Env::new();
//         assert_eq!(eval_expr("5", &mut env), Ok(F::Int(5)));
//     }

//     #[test]
//     #[ignore]
//     fn eval_string() {
//         let mut env = Env::new();
//         assert_eq!(eval_expr("\"Hello\"", &mut env), Ok(F::string("Hello")));
//     }

//     /// Eval symbols
//     #[test]
//     #[ignore]
//     fn eval_symbols() {
//         let mut env = Env::new();
//         env.bind(&SymbolId::from("greeting"), F::string("hello world"));

//         assert_eq!(
//             eval_expr("greeting", &mut env),
//             Ok(F::string("hello world"))
//         );

//         assert!(matches!(
//             eval_expr("undefined", &mut env),
//             Err(Error::UndefinedSymbol(_))
//         ));
//     }

//     /// Eval list
//     #[test]
//     #[ignore]
//     fn eval_list_empty() {
//         let mut env = Env::new();
//         assert_eq!(eval_expr("()", &mut env), Err(Error::MissingProcedure),);
//     }

//     /// Eval functions
//     #[test]
//     #[ignore]
//     fn eval_function() {
//         // let mut env = Env::new();
//         // env.bind(
//         //     &SymbolId::from("echo"),
//         //     F::Lambda(Lambda {
//         //         params: vec![SymbolId::from("x")],
//         //         body: vec![Form::symbol("x")],
//         //     }),
//         // );

//         // assert!(matches!(eval_expr("echo", &mut env), Ok(F::Lambda(_)),));

//         // assert_eq!(eval_expr("(echo 10)", &mut env), Ok(F::Int(10)));
//     }

//     /// Eval special forms
//     #[test]
//     #[ignore]
//     fn eval_special_form() {
//         let mut env = Env::new();
//         env.bind_native(NativeFunc {
//             symbol: SymbolId::from("quote"),
//             func: |arg_forms, _env| Ok(arg_forms[0].clone()),
//         });

//         assert!(matches!(
//             eval_expr("quote", &mut env),
//             Ok(F::NativeFunc(l)) if l.symbol == SymbolId::from("quote"),
//         ));

//         assert_eq!(
//             eval_expr("(quote (1 2 3))", &mut env),
//             Ok(F::List(vec![F::Int(1), F::Int(2), F::Int(3),]))
//         );
//     }
// }
