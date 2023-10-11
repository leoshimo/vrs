//! Compiler for Lemma Form AST
use crate::{Form, SymbolId};

// TODO: Compact bytecode repr
/// Bytecode instructions
#[derive(Debug, Clone, PartialEq)]
pub enum Inst {
    /// Push constant form onto stack
    PushConst(Form),
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
    /// Start a new scope in environment
    BeginScope,
    /// End active scope
    EndScope,
}

/// Errors during compilation
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum CompileError {
    #[error("Invalid expression - {0}")]
    InvalidExpression(String),
}

pub type Result<T> = std::result::Result<T, CompileError>;

/// Compile expression
pub fn compile(f: &Form) -> Result<Vec<Inst>> {
    match f {
        Form::List(l) => {
            let (first, args) = l.split_first().ok_or(CompileError::InvalidExpression(
                "Empty list expression".to_string(),
            ))?;

            // special forms
            if let Form::Symbol(s) = first {
                match s.as_str() {
                    "def" => return compile_def(args),
                    "lambda" => return compile_lambda(args),
                    "begin" => return compile_begin(args),
                    _ => (),
                }
            }

            compile_func_call(first, args)
        }
        Form::Symbol(s) => Ok(vec![Inst::LoadSym(s.clone())]),
        _ => Ok(vec![Inst::PushConst(f.clone())]),
    }
}

/// Compile special form builtin def
fn compile_def(args: &[Form]) -> Result<Vec<Inst>> {
    let (symbol, value) = match args {
        [Form::Symbol(symbol), value] => (symbol, value),
        _ => {
            return Err(CompileError::InvalidExpression(
                "def accepts one symbol and one form as arguments".to_string(),
            ))
        }
    };

    let mut inst = compile(value)?;
    inst.push(Inst::StoreSym(symbol.clone()));
    Ok(inst)
}

/// Compile special form lambda
fn compile_lambda(args: &[Form]) -> Result<Vec<Inst>> {
    let (param, body) = match args {
        [param, body] => (param, body),
        _ => {
            return Err(CompileError::InvalidExpression(
                "lambda expects a parameter list and body expression as arguments".to_string(),
            ))
        }
    };

    let bytecode = compile(body)?;

    Ok(vec![
        Inst::PushConst(param.clone()),
        Inst::PushConst(Form::Bytecode(bytecode)),
        Inst::MakeFunc,
    ])
}

/// Compile function calls
fn compile_func_call(func: &Form, args: &[Form]) -> Result<Vec<Inst>> {
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
fn compile_begin(args: &[Form]) -> Result<Vec<Inst>> {
    let mut bc = vec![Inst::BeginScope];
    let mut is_first = true;
    for a in args {
        if is_first {
            is_first = false;
        } else {
            bc.push(Inst::PopTop); // discard result from previous call
        }
        bc.extend(compile(a)?);
    }
    bc.push(Inst::EndScope);
    Ok(bc)
}

#[cfg(test)]
mod tests {
    use super::Inst::*;
    use super::*;
    use crate::parse;

    #[test]
    fn compile_self_evaluating() {
        assert_eq!(compile(&Form::Int(10)), Ok(vec![PushConst(Form::Int(10)),]));
        assert_eq!(
            compile(&Form::string("Hello")),
            Ok(vec![PushConst(Form::string("Hello")),])
        );
    }

    #[test]
    fn compile_symbol() {
        assert_eq!(
            compile(&Form::symbol("x")),
            Ok(vec![LoadSym(SymbolId::from("x"))])
        );
    }

    #[test]
    fn compile_empty_list() {
        assert!(matches!(
            compile(&f("()")),
            Err(CompileError::InvalidExpression(_))
        ))
    }

    #[test]
    fn compile_def() {
        assert_eq!(
            compile(&f("(def x 5)")),
            Ok(vec![PushConst(Form::Int(5)), StoreSym(SymbolId::from("x")),])
        );
    }

    #[test]
    fn compile_lambda() {
        assert_eq!(
            compile(&f("(lambda (x) x)")),
            Ok(vec![
                PushConst(Form::List(vec![Form::symbol("x")])),
                PushConst(Form::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
                MakeFunc
            ])
        );

        assert_eq!(
            compile(&f("(lambda (x) (lambda () x))")),
            Ok(vec![
                PushConst(Form::List(vec![Form::symbol("x")])),
                PushConst(Form::Bytecode(vec![
                    PushConst(Form::List(vec![])),
                    PushConst(Form::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
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
                PushConst(Form::string("Hello world")),
                CallFunc(1)
            ])
        );

        assert_eq!(
            compile(&f("(+ 1 2 3 4 5)")),
            Ok(vec![
                LoadSym(SymbolId::from("+")),
                PushConst(Form::Int(1)),
                PushConst(Form::Int(2)),
                PushConst(Form::Int(3)),
                PushConst(Form::Int(4)),
                PushConst(Form::Int(5)),
                CallFunc(5)
            ])
        );

        assert_eq!(
            compile(&f("(one (two 3 (four)))")),
            Ok(vec![
                LoadSym(SymbolId::from("one")),
                LoadSym(SymbolId::from("two")),
                PushConst(Form::Int(3)),
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
                PushConst(Form::List(vec![])),
                PushConst(Form::Bytecode(vec![PushConst(Form::string("hello")),])),
                MakeFunc,
                CallFunc(0),
            ])
        );
        assert_eq!(
            compile(&f("((lambda (x) x) 10)")),
            Ok(vec![
                PushConst(Form::List(vec![Form::symbol("x")])),
                PushConst(Form::Bytecode(vec![LoadSym(SymbolId::from("x")),])),
                MakeFunc,
                PushConst(Form::Int(10)),
                CallFunc(1),
            ])
        );
    }

    #[test]
    fn compile_func_call_nested() {
        assert_eq!(
            compile(&f("(((lambda (x) (lambda () x)) \"hello\"))")),
            Ok(vec![
                PushConst(Form::List(vec![Form::symbol("x")])),
                PushConst(Form::Bytecode(vec![
                    PushConst(Form::List(vec![])),
                    PushConst(Form::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
                    MakeFunc
                ])),
                MakeFunc,
                PushConst(Form::string("hello")),
                CallFunc(1),
                CallFunc(0),
            ])
        );
        assert_eq!(
            compile(&f("(((lambda () (lambda (x) x))) \"hello\")")),
            Ok(vec![
                PushConst(Form::List(vec![])),
                PushConst(Form::Bytecode(vec![
                    PushConst(Form::List(vec![Form::symbol("x")])),
                    PushConst(Form::Bytecode(vec![LoadSym(SymbolId::from("x"))])),
                    MakeFunc
                ])),
                MakeFunc,
                CallFunc(0),
                PushConst(Form::string("hello")),
                CallFunc(1),
            ])
        );
    }

    #[test]
    fn compile_begin() {
        assert_eq!(
            compile(&f("(begin 1 2 3 4 5)")),
            Ok(vec![
                BeginScope,
                PushConst(Form::Int(1)),
                PopTop,
                PushConst(Form::Int(2)),
                PopTop,
                PushConst(Form::Int(3)),
                PopTop,
                PushConst(Form::Int(4)),
                PopTop,
                PushConst(Form::Int(5)),
                EndScope,
            ])
        )
    }

    /// Convenience for creating Forms from strs
    fn f(expr: &str) -> Form {
        parse(expr).expect("expr should be valid form")
    }
}
