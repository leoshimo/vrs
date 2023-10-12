//! Compiler for Lemma Form AST
use crate::{Error, Result, SymbolId, Val};

// TODO: Compact bytecode repr
/// Bytecode instructions
#[derive(Debug, Clone, PartialEq)]
pub enum Inst {
    /// Push constant form onto stack
    PushConst(Val),
    /// Push value bound to given symbol onto stack
    LoadSym(SymbolId),
    /// Pop TOS and store value as given symbol
    StoreSym(SymbolId),
    /// Pop parameter list and function body from stack, and pushes a new function onto stack
    MakeFunc,
    /// Call func by popping N forms and function object off stack, and pushing result
    CallFunc(usize),
    /// Pop the top of the stack
    PopTop,
}

/// Compile a value to bytecode representation
pub fn compile(v: &Val) -> Result<Vec<Inst>> {
    match v {
        Val::List(l) => {
            let (first, args) = l.split_first().ok_or(Error::InvalidExpression(
                "Empty list expression".to_string(),
            ))?;

            // special forms
            if let Val::Symbol(s) = first {
                match s.as_str() {
                    "def" => return compile_def(args),
                    "lambda" => return compile_lambda(args),
                    "begin" => return compile_begin(args),
                    _ => (),
                }
            }

            compile_func_call(first, args)
        }
        Val::Symbol(s) => Ok(vec![Inst::LoadSym(s.clone())]),
        _ => Ok(vec![Inst::PushConst(v.clone())]),
    }
}

/// Compile special form builtin def
fn compile_def(args: &[Val]) -> Result<Vec<Inst>> {
    let (symbol, value) = match args {
        [Val::Symbol(symbol), value] => (symbol, value),
        _ => {
            return Err(Error::InvalidExpression(
                "def accepts one symbol and one form as arguments".to_string(),
            ))
        }
    };

    let mut inst = compile(value)?;
    inst.push(Inst::StoreSym(symbol.clone()));
    Ok(inst)
}

/// Compile special form lambda
fn compile_lambda(args: &[Val]) -> Result<Vec<Inst>> {
    let (param, body) = match args {
        [param, body] => (param, body),
        _ => {
            return Err(Error::InvalidExpression(
                "lambda expects a parameter list and body expression as arguments".to_string(),
            ))
        }
    };

    let bytecode = compile(body)?;

    Ok(vec![
        Inst::PushConst(param.clone()),
        Inst::PushConst(Val::Bytecode(bytecode)),
        Inst::MakeFunc,
    ])
}

/// Compile function calls
fn compile_func_call(func: &Val, args: &[Val]) -> Result<Vec<Inst>> {
    let mut bytecode = vec![];
    let nargs = args.len();

    let func_code = compile(func)?;
    let arg_code = args
        .iter()
        .map(compile)
        .collect::<Result<Vec<_>>>()?
        .concat();

    bytecode.extend(func_code);
    bytecode.extend(arg_code);
    bytecode.push(Inst::CallFunc(nargs));

    Ok(bytecode)
}

/// Compile builtin begin
fn compile_begin(args: &[Val]) -> Result<Vec<Inst>> {
    // Compile to anonymous lambda MakeFunc + CallFunc
    let mut inner = vec![];
    let mut is_first = true;
    for a in args {
        if is_first {
            is_first = false;
        } else {
            inner.push(Inst::PopTop); // discard result from previous call
        }
        inner.extend(compile(a)?);
    }
    Ok(vec![
        Inst::PushConst(Val::List(vec![])),
        Inst::PushConst(Val::Bytecode(inner)),
        Inst::MakeFunc,
        Inst::CallFunc(0),
    ])
}

#[cfg(test)]
mod tests {
    use super::Inst::*;
    use super::*;
    use crate::parse;

    #[test]
    fn compile_self_evaluating() {
        assert_eq!(compile(&Val::Int(10)), Ok(vec![PushConst(Val::Int(10)),]));
        assert_eq!(
            compile(&Val::string("Hello")),
            Ok(vec![PushConst(Val::string("Hello")),])
        );
    }

    #[test]
    fn compile_symbol() {
        assert_eq!(
            compile(&Val::symbol("x")),
            Ok(vec![LoadSym(SymbolId::from("x"))])
        );
    }

    #[test]
    fn compile_empty_list() {
        assert!(matches!(
            compile(&f("()")),
            Err(Error::InvalidExpression(_))
        ))
    }

    #[test]
    fn compile_def() {
        assert_eq!(
            compile(&f("(def x 5)")),
            Ok(vec![PushConst(Val::Int(5)), StoreSym(SymbolId::from("x")),])
        );
    }

    #[test]
    fn compile_lambda() {
        assert_eq!(
            compile(&f("(lambda (x) x)")),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
                MakeFunc
            ])
        );

        assert_eq!(
            compile(&f("(lambda (x) (lambda () x))")),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![
                    PushConst(Val::List(vec![])),
                    PushConst(Val::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
                    MakeFunc,
                ])),
                MakeFunc
            ])
        );
    }

    #[test]
    fn compile_func_call() {
        assert_eq!(
            compile(&f("(echo \"Hello world\")")),
            Ok(vec![
                LoadSym(SymbolId::from("echo")),
                PushConst(Val::string("Hello world")),
                CallFunc(1)
            ])
        );

        assert_eq!(
            compile(&f("(+ 1 2 3 4 5)")),
            Ok(vec![
                LoadSym(SymbolId::from("+")),
                PushConst(Val::Int(1)),
                PushConst(Val::Int(2)),
                PushConst(Val::Int(3)),
                PushConst(Val::Int(4)),
                PushConst(Val::Int(5)),
                CallFunc(5)
            ])
        );

        assert_eq!(
            compile(&f("(one (two 3 (four)))")),
            Ok(vec![
                LoadSym(SymbolId::from("one")),
                LoadSym(SymbolId::from("two")),
                PushConst(Val::Int(3)),
                LoadSym(SymbolId::from("four")),
                CallFunc(0),
                CallFunc(2),
                CallFunc(1),
            ])
        );
    }

    #[test]
    fn compile_func_call_lambda() {
        assert_eq!(
            compile(&f("((lambda () \"hello\"))")),
            Ok(vec![
                PushConst(Val::List(vec![])),
                PushConst(Val::Bytecode(vec![PushConst(Val::string("hello")),])),
                MakeFunc,
                CallFunc(0),
            ])
        );
        assert_eq!(
            compile(&f("((lambda (x) x) 10)")),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![LoadSym(SymbolId::from("x")),])),
                MakeFunc,
                PushConst(Val::Int(10)),
                CallFunc(1),
            ])
        );
    }

    #[test]
    fn compile_func_call_nested() {
        assert_eq!(
            compile(&f("(((lambda (x) (lambda () x)) \"hello\"))")),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![
                    PushConst(Val::List(vec![])),
                    PushConst(Val::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
                    MakeFunc
                ])),
                MakeFunc,
                PushConst(Val::string("hello")),
                CallFunc(1),
                CallFunc(0),
            ])
        );
        assert_eq!(
            compile(&f("(((lambda () (lambda (x) x))) \"hello\")")),
            Ok(vec![
                PushConst(Val::List(vec![])),
                PushConst(Val::Bytecode(vec![
                    PushConst(Val::List(vec![Val::symbol("x")])),
                    PushConst(Val::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
                    MakeFunc
                ])),
                MakeFunc,
                CallFunc(0),
                PushConst(Val::string("hello")),
                CallFunc(1),
            ])
        );
    }

    #[test]
    fn compile_begin() {
        assert_eq!(
            compile(&f("(begin 1 2 3 4 5)")),
            Ok(vec![
                PushConst(Val::List(vec![])),
                PushConst(Val::Bytecode(vec![
                    PushConst(Val::Int(1)),
                    PopTop,
                    PushConst(Val::Int(2)),
                    PopTop,
                    PushConst(Val::Int(3)),
                    PopTop,
                    PushConst(Val::Int(4)),
                    PopTop,
                    PushConst(Val::Int(5)),
                ])),
                MakeFunc,
                CallFunc(0),
            ])
        )
    }

    /// Convenience for creating Val from expressions
    fn f(expr: &str) -> Val {
        parse(expr).expect("expr should be valid form").into()
    }
}