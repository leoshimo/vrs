//! Compiler for Lyric Form AST
use crate::{Bytecode, Error, Extern, Locals, Result, SymbolId, Val};

// TODO: Compact bytecode repr
/// Bytecode instructions
#[derive(Debug, Clone, PartialEq)]
pub enum Inst<T, L>
where
    T: Extern,
    L: Locals,
{
    /// Push constant form onto stack
    PushConst(Val<T, L>),
    /// Push value bound to given symbol onto stack
    GetSym(SymbolId),
    /// Pop TOS and store value as given symbol
    DefSym(SymbolId),
    /// Pop TOS twice for pattern and constant value, and define symbols in env if pattern matches
    DefBind,
    /// Set given symbol to value popped from TOS
    SetSym(SymbolId),
    /// Pop parameter list and function body from stack, and pushes a new function onto stack
    MakeFunc,
    /// Call func by popping N forms and function object off stack, and pushing result
    CallFunc(usize),
    /// Pop the top of the stack
    PopTop,
    /// Jump forward N inst
    JumpFwd(usize),
    /// Jump backward N inst
    JumpBck(usize),
    /// Conditional Jump forward N inst
    PopJumpFwdIfTrue(usize),
    /// Yield TOS as value
    YieldTop,
    /// Evaluate TOS and push value back onto stack. May be protected eval
    Eval(bool),
}

/// Compile a value to bytecode representation
pub fn compile<T: Extern, L: Locals>(v: &Val<T, L>) -> Result<Bytecode<T, L>> {
    match v {
        Val::List(l) => {
            let (first, args) = l.split_first().ok_or(Error::InvalidExpression(
                "Empty list expression".to_string(),
            ))?;

            // special forms
            if let Val::Symbol(s) = first {
                match s.as_str() {
                    "begin" => return compile_begin(args),
                    "def" => return compile_def(args),
                    "defn" => return compile_defn(args),
                    "if" => return compile_if(args),
                    "cond" => return compile_cond(args),
                    "lambda" => return compile_lambda(args),
                    "let" => return compile_let(args),
                    "quote" => return compile_quote(args),
                    "set" => return compile_set(args),
                    "eval" => return compile_eval(args, false),
                    "peval" => return compile_eval(args, true),
                    "yield" => return compile_yield(args),
                    "loop" => return compile_loop(args),
                    _ => (),
                }
            }
            compile_func_call(first, args)
        }
        Val::Symbol(s) => Ok(vec![Inst::GetSym(s.clone())]),
        _ => Ok(vec![Inst::PushConst(v.clone())]),
    }
}

/// Compile special form builtin def
fn compile_def<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    match args {
        [Val::Symbol(symbol), value] => {
            let mut inst = compile(value)?;
            inst.push(Inst::DefSym(symbol.clone()));
            Ok(inst)
        }
        [pat, value] => {
            let mut inst = compile(value)?;
            inst.push(Inst::PushConst(pat.clone()));
            inst.push(Inst::DefBind);
            Ok(inst)
        }
        _ => Err(Error::InvalidExpression(
            "def accepts one symbol and one form as arguments".to_string(),
        )),
    }
}

/// Compile special form builtin set
fn compile_set<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let (symbol, value) = match args {
        [Val::Symbol(symbol), value] => (symbol, value),
        _ => {
            return Err(Error::InvalidExpression(
                "def accepts one symbol and one form as arguments".to_string(),
            ))
        }
    };

    let mut inst = compile(value)?;
    inst.push(Inst::SetSym(symbol.clone()));
    Ok(inst)
}

// TODO: Replace `defn` with a macro
/// Compile defn
fn compile_defn<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let (name, params, body) = match args {
        [name, params, body @ ..] if !body.is_empty() => (name, params, body),
        _ => {
            return Err(Error::InvalidExpression(
                "defn expects at least three arguments with nonempty body".to_string(),
            ))
        }
    };
    let inst = compile(&Val::List(vec![
        Val::symbol("def"),
        name.clone(),
        Val::List(vec![
            Val::symbol("lambda"),
            params.clone(),
            Val::List(
                std::iter::once(Val::symbol("begin"))
                    .chain(body.iter().cloned())
                    .collect(),
            ),
        ]),
    ]))?;
    Ok(inst)
}

/// Compile special form lambda
fn compile_lambda<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
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

/// Compile quote special forms
fn compile_quote<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let v = match args {
        [v] => v,
        _ => {
            return Err(Error::InvalidExpression(
                "quote expects a single argument".to_string(),
            ))
        }
    };
    Ok(vec![Inst::PushConst(v.clone())])
}

fn compile_eval<T: Extern, L: Locals>(
    args: &[Val<T, L>],
    is_protected: bool,
) -> Result<Bytecode<T, L>> {
    let v = match args {
        [v] => v,
        _ => {
            return Err(Error::InvalidExpression(
                "eval expects one argument".to_string(),
            ))
        }
    };

    let mut bc = compile(v)?;
    bc.push(Inst::Eval(is_protected));
    Ok(bc)
}

/// Compile function calls
fn compile_func_call<T: Extern, L: Locals>(
    func: &Val<T, L>,
    args: &[Val<T, L>],
) -> Result<Bytecode<T, L>> {
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

/// Compile builtin let
fn compile_let<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let (bindings, body) = match args.split_first() {
        Some((Val::List(bindings), body)) => (bindings, body),
        _ => {
            return Err(Error::InvalidExpression(
                "let expects binding list and body expression as args".to_string(),
            ))
        }
    };

    let mut params: Vec<Val<_, _>> = vec![]; /* get first symbol in each binding pair */
    let mut args: Vec<Val<_, _>> = vec![]; /* get second symbol in each thing */
    for b in bindings {
        let pair = match b {
            Val::List(pair) => pair,
            _ => {
                return Err(Error::InvalidExpression(
                    "non-list in let bindings".to_string(),
                ))
            }
        };
        match &pair[..] {
            [sym, val] => {
                params.push(sym.clone());
                args.push(val.clone());
            }

            _ => {
                return Err(Error::InvalidExpression(
                    "pair in let bindings must contain one symbol and one expression".to_string(),
                ))
            }
        }
    }

    let mut body_block = vec![Val::symbol("begin")];
    body_block.extend(body.iter().cloned());

    let mut lambda = vec![Val::List(vec![
        Val::symbol("lambda"),
        Val::List(params),
        Val::List(body_block),
    ])];
    lambda.extend(args);

    compile(&Val::List(lambda))
}

/// Compile builtin begin
fn compile_begin<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let mut inst = vec![];
    let mut is_first = true;
    for a in args {
        if is_first {
            is_first = false;
        } else {
            inst.push(Inst::PopTop); // discard result from previous call
        }
        inst.extend(compile(a)?);
    }

    if inst.is_empty() {
        inst.push(Inst::PushConst(Val::Nil));
    }

    Ok(inst)
}

/// Compile if
fn compile_if<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let (cond, t, f) = match args {
        [c, t, f] => (c, t, f),
        [c, t] => (c, t, &Val::Nil),
        _ => {
            return Err(Error::InvalidExpression(
                "if expects three arguments".to_string(),
            ))
        }
    };

    let mut bc = compile(cond)?;
    let t_code = compile(t)?;
    let f_code = compile(f)?;

    bc.push(Inst::PopJumpFwdIfTrue(f_code.len() + 1));
    bc.extend(f_code);
    bc.push(Inst::JumpFwd(t_code.len()));
    bc.extend(t_code);

    Ok(bc)
}

// TODO: Replace `cond` with a macro
/// Compile cond
fn compile_cond<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let mut res = Val::Nil;

    for f in args.iter().rev() {
        let pair = match f {
            Val::List(pair) => pair,
            _ => {
                return Err(Error::UnexpectedArguments(
                    "cond expects a list of pairs".to_string(),
                ))
            }
        };
        let (cond, expr) = match &pair[..] {
            [cond, expr] => (cond.clone(), expr.clone()),
            _ => {
                return Err(Error::UnexpectedArguments(
                    "cond expects a list of pairs".to_string(),
                ))
            }
        };

        // transform to nested `if`
        res = Val::List(vec![Val::symbol("if"), cond, expr, res]);
    }

    compile(&res)
}

/// Compile yield statement
fn compile_yield<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let v = match args {
        [] => &Val::Nil,
        [v] => v,
        _ => {
            return Err(Error::InvalidExpression(
                "yield accepts zero or one argument".to_string(),
            ))
        }
    };
    let mut inst = compile(v)?;
    inst.push(Inst::YieldTop);
    Ok(inst)
}

/// Compile loop expr
fn compile_loop<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let mut inst = compile_begin(args)?;
    inst.push(Inst::PopTop);
    inst.push(Inst::JumpBck(inst.len() + 1));
    Ok(inst)
}

impl<T: Extern, L: Locals> std::fmt::Display for Inst<T, L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Inst::PushConst(c) => write!(f, "pushco {c}"),
            Inst::GetSym(s) => write!(f, "getsym {s}"),
            Inst::DefSym(s) => write!(f, "defsym {s}"),
            Inst::DefBind => write!(f, "defbind"),
            Inst::SetSym(s) => write!(f, "setsym {s}"),
            Inst::MakeFunc => write!(f, "makefn"),
            Inst::CallFunc(nargs) => write!(f, "callfn {nargs}"),
            Inst::PopTop => write!(f, "poptop"),
            Inst::JumpFwd(o) => write!(f, "jmpfwd {o}"),
            Inst::JumpBck(o) => write!(f, "jmpbck {o}"),
            Inst::PopJumpFwdIfTrue(o) => write!(f, "jmpift {o}"),
            Inst::YieldTop => write!(f, "yldtop"),
            Inst::Eval(p) => {
                if *p {
                    write!(f, "eval")
                } else {
                    write!(f, "peval")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Inst::*;
    use super::*;
    use crate::parse;
    use void::Void;

    type Val = super::Val<Void, Void>;
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
            Ok(vec![GetSym(SymbolId::from("x"))])
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
            Ok(vec![PushConst(Val::Int(5)), DefSym(SymbolId::from("x")),])
        );

        assert_eq!(
            compile(&f("(def (a (b c)) val)")),
            Ok(vec![
                GetSym(SymbolId::from("val")),
                PushConst(Val::List(vec![
                    Val::symbol("a"),
                    Val::List(vec![Val::symbol("b"), Val::symbol("c")])
                ])),
                DefBind,
            ])
        );
    }

    #[test]
    fn compile_lambda() {
        assert_eq!(
            compile(&f("(lambda (x) x)")),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
                MakeFunc
            ])
        );

        assert_eq!(
            compile(&f("(lambda (x) (lambda () x))")),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![
                    PushConst(Val::List(vec![])),
                    PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
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
                GetSym(SymbolId::from("echo")),
                PushConst(Val::string("Hello world")),
                CallFunc(1)
            ])
        );

        assert_eq!(
            compile(&f("(+ 1 2 3 4 5)")),
            Ok(vec![
                GetSym(SymbolId::from("+")),
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
                GetSym(SymbolId::from("one")),
                GetSym(SymbolId::from("two")),
                PushConst(Val::Int(3)),
                GetSym(SymbolId::from("four")),
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
                PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x")),])),
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
                    PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
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
                    PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
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
                PushConst(Val::Int(1)),
                PopTop,
                PushConst(Val::Int(2)),
                PopTop,
                PushConst(Val::Int(3)),
                PopTop,
                PushConst(Val::Int(4)),
                PopTop,
                PushConst(Val::Int(5)),
            ])
        )
    }

    #[test]
    fn compile_quote() {
        assert_eq!(
            compile(&f("(quote (one :two three))")),
            Ok(vec![PushConst(Val::List(vec![
                Val::symbol("one"),
                Val::keyword("two"),
                Val::symbol("three"),
            ]))]),
            "functions and symbols should not be evaluated"
        );

        assert_eq!(
            compile(&f("'(one :two three)")),
            Ok(vec![PushConst(Val::List(vec![
                Val::symbol("one"),
                Val::keyword("two"),
                Val::symbol("three"),
            ]))]),
        );

        assert_eq!(
            compile(&f("(quote (lambda (x) x))")),
            Ok(vec![PushConst(Val::List(vec![
                Val::symbol("lambda"),
                Val::List(vec![Val::symbol("x")]),
                Val::symbol("x"),
            ]))]),
        );
    }

    #[test]
    fn compile_if() {
        assert_eq!(
            compile(&f("(if true \"true\" \"false\")")),
            Ok(vec![
                PushConst(Val::Bool(true)),
                PopJumpFwdIfTrue(2),
                PushConst(Val::string("false")),
                JumpFwd(1),
                PushConst(Val::string("true")),
            ])
        )
    }

    #[test]
    fn compile_yield() {
        assert_eq!(
            compile(&f("(yield)")),
            Ok(vec![PushConst(Val::Nil), YieldTop,])
        );

        assert_eq!(
            compile(&f("(yield 10)")),
            Ok(vec![PushConst(Val::Int(10)), YieldTop,])
        );

        assert_eq!(
            compile(&f("(yield ((lambda () 10)))")),
            Ok(vec![
                PushConst(Val::List(vec![])),
                PushConst(Val::Bytecode(vec![PushConst(Val::Int(10))])),
                MakeFunc,
                CallFunc(0),
                YieldTop,
            ])
        );
    }

    #[test]
    fn compile_let() {
        assert_eq!(
            compile(&f("(let () 10)")),
            Ok(vec![
                PushConst(Val::List(vec![])),
                PushConst(Val::Bytecode(vec![PushConst(Val::Int(10))])),
                MakeFunc,
                CallFunc(0)
            ])
        );

        let prog = r#"
            (let ((a 10)
                  (b (+ 1 2)))
                 (+ a b)
                 :ok)
        "#;
        assert_eq!(
            compile(&f(prog)),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("a"), Val::symbol("b")])),
                PushConst(Val::Bytecode(vec![
                    GetSym(SymbolId::from("+")),
                    GetSym(SymbolId::from("a")),
                    GetSym(SymbolId::from("b")),
                    CallFunc(2),
                    PopTop,
                    PushConst(Val::keyword("ok"))
                ])),
                MakeFunc,
                PushConst(Val::Int(10)),
                GetSym(SymbolId::from("+")),
                PushConst(Val::Int(1)),
                PushConst(Val::Int(2)),
                CallFunc(2),
                CallFunc(2),
            ])
        )
    }

    #[test]
    fn compile_eval() {
        assert_eq!(
            compile(&f("(eval 42)")),
            Ok(vec![PushConst(Val::Int(42)), Eval(false),])
        );
        assert_eq!(
            compile(&f("(eval (+ 1 2))")),
            Ok(vec![
                GetSym(SymbolId::from("+")),
                PushConst(Val::Int(1)),
                PushConst(Val::Int(2)),
                CallFunc(2),
                Eval(false),
            ])
        );
        assert_eq!(
            compile(&f("(eval '(+ 1 2))")),
            Ok(vec![
                PushConst(Val::List(vec![Val::symbol("+"), Val::Int(1), Val::Int(2),])),
                Eval(false),
            ])
        );
    }

    #[test]
    fn compile_loop() {
        assert_eq!(
            compile(&f("(loop (+ 1 2))")),
            Ok(vec![
                GetSym(SymbolId::from("+")),
                PushConst(Val::Int(1)),
                PushConst(Val::Int(2)),
                CallFunc(2),
                PopTop,
                JumpBck(6),
            ])
        );
    }

    /// Convenience for creating Val from expressions
    fn f(expr: &str) -> Val {
        parse(expr).expect("expr should be valid form").into()
    }
}
