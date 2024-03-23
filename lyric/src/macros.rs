//! Macros are ordinary lexical functions invoked with unevaluated source at runtime.
//! Expansion runs on the calling fiber, so ordinary helpers and async calls work.
use crate::{
    env::EnvRef, Env, Error, Extern, Fiber, Form, Inst, Lambda, Locals, NativeFn, NativeFnOp,
    Result, SymbolId, Val,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
pub struct MacroEnv<T: Extern, L: Locals> {
    definitions: HashMap<String, Definition<T, L>>,
}
#[derive(Clone, Debug)]
pub(crate) struct Definition<T: Extern, L: Locals> {
    pub(crate) rest: bool,
    pub(crate) function: Lambda<T, L>,
}
impl<T: Extern, L: Locals> Default for MacroEnv<T, L> {
    fn default() -> Self {
        let mut env = Self {
            definitions: HashMap::new(),
        };
        let source = crate::parse("(defmacro when (test & body) \"Evaluate BODY when TEST is true.\" (list 'if test (concat '(begin) body) nil))").unwrap();
        env.define(&Val::from(source), None)
            .expect("standard macro must compile");
        env
    }
}
impl<T: Extern, L: Locals> MacroEnv<T, L> {
    pub(crate) fn define(
        &mut self,
        source: &Val<T, L>,
        parent: Option<EnvRef<T, L>>,
    ) -> Result<SymbolId> {
        validate_source(source)?;
        let [_, Val::Symbol(name), Val::List(params), body @ ..] = source.as_list()?.as_slice()
        else {
            return Err(fail("defmacro expects a name, parameter list and body"));
        };
        if name.as_str().is_empty() || name.as_str().ends_with('!') {
            return Err(fail("defmacro name must not end in !"));
        }
        let mut names = vec![];
        let mut rest = false;
        let mut iter = params.iter();
        while let Some(param) = iter.next() {
            let param = param
                .as_symbol()
                .map_err(|_| fail("macro parameters must be symbols"))?;
            if param.as_str() == "&" {
                let next = iter
                    .next()
                    .ok_or_else(|| fail("& needs a final rest parameter"))?
                    .as_symbol()?;
                if next.as_str() == "&" || iter.next().is_some() {
                    return Err(fail("& needs exactly one final rest parameter"));
                }
                names.push(next.clone());
                rest = true;
                break;
            }
            names.push(param.clone());
        }
        let mut seen = std::collections::HashSet::new();
        if names.iter().any(|name| !seen.insert(name)) {
            return Err(fail("duplicate macro parameter"));
        }
        let (doc, body) = match body {
            [Val::String(doc), body @ ..] => (Some(doc.clone()), body),
            body => (None, body),
        };
        if body.is_empty() {
            return Err(fail("defmacro requires a body"));
        }
        let body = Val::List(
            std::iter::once(Val::symbol("begin"))
                .chain(body.iter().cloned())
                .collect(),
        );
        let function = Lambda {
            metadata: vec![],
            doc,
            params: names,
            code: crate::compile(&body)?,
            parent,
        };
        self.definitions
            .insert(name.as_str().into(), Definition { rest, function });
        Ok(name.clone())
    }
    pub(crate) fn get(&self, name: &str) -> Result<Definition<T, L>> {
        if name == "!" || name.ends_with("!!") {
            return Err(fail(format!("invalid macro invocation {name}")));
        }
        self.definitions
            .get(&name[..name.len() - 1])
            .cloned()
            .ok_or_else(|| fail(format!("undefined macro {name}")))
    }
    /// Host libraries use the destination process's globals, like native library lambdas.
    pub fn use_global_scope(&mut self) {
        for definition in self.definitions.values_mut() {
            definition.function.parent = None;
        }
    }
}

#[derive(Debug)]
pub(crate) struct Budget {
    fuel: usize,
    calls: usize,
    nodes: usize,
}
pub(crate) type BudgetRef = Arc<Mutex<Budget>>;
pub(crate) fn budget() -> BudgetRef {
    Arc::new(Mutex::new(Budget {
        fuel: 1_000_000,
        calls: 10_000,
        nodes: 1_000_000,
    }))
}
fn fail(message: impl Into<String>) -> Error {
    Error::Macro(message.into())
}
pub(crate) fn tick(budget: &BudgetRef) -> Result<()> {
    let mut b = budget.lock().unwrap();
    b.fuel = b
        .fuel
        .checked_sub(1)
        .ok_or_else(|| fail("macro instruction limit exceeded"))?;
    Ok(())
}
pub(crate) fn invocation(budget: &BudgetRef) -> Result<()> {
    let mut b = budget.lock().unwrap();
    b.calls = b
        .calls
        .checked_sub(1)
        .ok_or_else(|| fail("macro invocation limit exceeded"))?;
    Ok(())
}
pub(crate) fn validate_result<T: Extern, L: Locals>(
    value: &Val<T, L>,
    budget: &BudgetRef,
) -> Result<()> {
    check_expansion_value(value)?;
    validate_source_counted(
        value,
        &mut vec![],
        &mut budget.lock().unwrap().nodes,
        "macro result",
    )?;
    Ok(())
}
pub(crate) fn head<T: Extern, L: Locals>(value: &Val<T, L>) -> Option<&str> {
    match value {
        Val::List(items) => match items.first() {
            Some(Val::Symbol(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

pub fn source_form<T: Extern, L: Locals>(value: &Val<T, L>) -> Result<Form> {
    validate_source(value)?;
    Form::try_from(value.clone())
}

pub(crate) fn validate_source<T: Extern, L: Locals>(value: &Val<T, L>) -> Result<()> {
    check_expansion_value(value)?;
    validate_source_counted(value, &mut vec![], &mut 1_000_000, "source")
}

/// Bound intermediate transformer data, before it can be repeatedly copied into a
/// growing expansion. Ordinary evaluation is unaffected by these expansion limits.
pub(crate) fn check_expansion_value<T: Extern, L: Locals>(value: &Val<T, L>) -> Result<()> {
    check_expansion_values(std::slice::from_ref(value))
}

pub(crate) fn check_expansion_values<T: Extern, L: Locals>(values: &[Val<T, L>]) -> Result<()> {
    let mut pending: Vec<_> = values.iter().map(|value| (value, 0)).collect();
    let mut nodes = 0usize;
    let mut bytes = 0usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if nodes > 1_000_000 || depth > 256 {
            return Err(fail("expansion value size/depth limit exceeded"));
        }
        match value {
            Val::String(s) => bytes = bytes.saturating_add(s.len()),
            Val::Symbol(s) => bytes = bytes.saturating_add(s.as_str().len()),
            Val::Keyword(k) => bytes = bytes.saturating_add(k.as_str().len()),
            Val::List(items) => pending.extend(items.iter().map(|v| (v, depth + 1))),
            _ => (),
        }
        if bytes > 1_000_000 {
            return Err(fail("expansion string size limit exceeded"));
        }
    }
    Ok(())
}

fn validate_source_counted<T: Extern, L: Locals>(
    value: &Val<T, L>,
    indices: &mut Vec<usize>,
    nodes: &mut usize,
    root: &str,
) -> Result<()> {
    // check_expansion_value has already bounded depth and string sizes.
    *nodes = nodes
        .checked_sub(1)
        .ok_or_else(|| fail("source node limit exceeded"))?;
    match value {
        Val::Nil
        | Val::Bool(_)
        | Val::Int(_)
        | Val::String(_)
        | Val::Symbol(_)
        | Val::Keyword(_) => (),
        Val::List(items) => {
            for (i, item) in items.iter().enumerate() {
                indices.push(i);
                validate_source_counted(item, indices, nodes, root)?;
                indices.pop();
            }
        }
        _ => {
            let path = indices
                .iter()
                .fold(root.to_owned(), |path, i| format!("{path}[{i}]"));
            return Err(fail(format!("expected source data at {path}, got {value}")));
        }
    }
    Ok(())
}

fn native<T: Extern, L: Locals>(
    doc: &str,
    func: crate::types::NativeFnSig<T, L>,
) -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: doc.into(),
        func,
    }
}
pub(crate) fn bind_builtins<T: Extern, L: Locals>(env: &mut Env<T, L>) {
    env.bind_native(SymbolId::from("eval_caller"), native(
        "(eval_caller FORM) - During macro expansion, evaluate source in the macro call's scope",
        |_, args| match args {
            [value] => Ok(NativeFnOp::Exec(vec![Inst::PushConst(value.clone()), Inst::EvalCaller])),
            _ => Err(fail("eval_caller expects one source form")),
        },
    ));
    env.bind_native(
        SymbolId::from("eval_global"),
        native(
            "(eval_global FORM) - Evaluate source in this process's global environment",
            |_, args| match args {
                [value] => Ok(NativeFnOp::EvalGlobal(value.clone())),
                _ => Err(fail("eval_global expects one source form")),
            },
        ),
    )
    .bind_native(
        SymbolId::from("matches?"),
        native(
            "(matches? PATTERN VALUE) - Test a pattern supplied as data",
            |_, args| match args {
                [pattern, value] => Ok(NativeFnOp::Return(Val::Bool(
                    crate::Pattern::from_val(pattern.clone()).is_match(value),
                ))),
                _ => Err(fail("matches? expects pattern and value")),
            },
        ),
    )
    .bind_native(
        SymbolId::from("with_meta"),
        crate::builtin::metadata::with_meta_fn(),
    );
    env.bind_native(SymbolId::from("macroexpand_1"),native("(macroexpand_1 FORM) - Expand one outer macro call as data; quote the call",|f,args|inspect(f,args,true)))
        .bind_native(SymbolId::from("macroexpand"),native("(macroexpand FORM) - Expand outer macro calls until the head is ordinary code; does not walk nested forms",|f,args|inspect(f,args,false)))
        .bind_native(SymbolId::from("gensym"),native("(gensym [HINT]) - Fresh printable source symbol",|_,args| {
            let hint=match args {[]=>"tmp",[Val::String(s)]=>s,_=>return Err(fail("gensym expects an optional string hint"))};
            let mut hint:String=hint.chars().filter(|c|c.is_ascii_alphanumeric()||*c=='_').take(32).collect();
            if hint.is_empty() {
                hint.push_str("tmp");
            } else if hint.as_bytes()[0].is_ascii_digit() {
                // The reader treats a token starting with a digit as an integer.
                hint.insert(0, '_');
            }
            // Keep 96 random bits in the actual symbol, including across processes
            // and printed/read-back expansions; display aliases would lose identity.
            Ok(NativeFnOp::Return(Val::symbol(&format!("{}__{}",hint,nanoid::nanoid!(16)))))
        }))
        .bind_native(SymbolId::from("symbol?"),native("(symbol? VALUE)",|_,args|one(args,|v|Ok(Val::Bool(matches!(v,Val::Symbol(_)))))))
        .bind_native(SymbolId::from("lambda?"),native("(lambda? VALUE)",|_,args|one(args,|v|Ok(Val::Bool(matches!(v,Val::Lambda(_)))))))
        .bind_native(SymbolId::from("symbol"),native("(symbol STRING-OR-KEYWORD)",|_,args|one(args,|v|match v {Val::String(s)=>Ok(Val::symbol(s)),Val::Keyword(k)=>Ok(Val::Symbol(k.clone().to_symbol())),_=>Err(fail("symbol expects a string or keyword"))})))
        .bind_native(SymbolId::from("keyword"),native("(keyword STRING-OR-SYMBOL)",|_,args|one(args,|v|match v {Val::String(s)=>Ok(Val::keyword(s)),Val::Symbol(s)=>Ok(Val::Keyword(s.clone().to_keyword())),_=>Err(fail("keyword expects a string or symbol"))})))
        .bind_native(SymbolId::from("concat"),native("(concat LIST ...) - Concatenate lists",|_,args| {
            let mut result=vec![];for arg in args {result.extend_from_slice(arg.as_list()?);}
            Ok(NativeFnOp::Return(Val::List(result)))
        }))
        .bind_native(SymbolId::from("slice"),native("(slice LIST START) - List suffix at nonnegative index",|_,args| {
            match args {[Val::List(items),Val::Int(start)] if *start>=0=>Ok(NativeFnOp::Return(Val::List(items.get(*start as usize..).unwrap_or(&[]).to_vec()))),_=>Err(fail("slice expects a list and nonnegative index"))}
        }));
}
fn one<T: Extern, L: Locals>(
    args: &[Val<T, L>],
    f: impl FnOnce(&Val<T, L>) -> Result<Val<T, L>>,
) -> Result<NativeFnOp<T, L>> {
    match args {
        [v] => Ok(NativeFnOp::Return(f(v)?)),
        _ => Err(fail("expected one argument")),
    }
}
fn inspect<T: Extern, L: Locals>(
    _fiber: &mut Fiber<T, L>,
    args: &[Val<T, L>],
    once: bool,
) -> Result<NativeFnOp<T, L>> {
    let [value] = args else {
        return Err(fail("macroexpand expects one source-data argument"));
    };
    validate_source(value)?;
    Ok(NativeFnOp::Exec(vec![
        Inst::PushConst(value.clone()),
        Inst::Expand(once),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    type Value = Val<void::Void, ()>;

    #[test]
    fn result_node_budget_is_shared_across_expansions() {
        let budget = budget();
        budget.lock().unwrap().nodes = 5;
        let value = Value::from_expr("(1 2)").unwrap();
        validate_result(&value, &budget).unwrap();
        assert!(validate_result(&value, &budget)
            .unwrap_err()
            .to_string()
            .contains("source node limit exceeded"));
    }

    #[test]
    fn validation_retains_nested_error_paths_and_readable_conversion() {
        let value = Value::List(vec![Value::List(vec![Value::NativeFn(
            crate::builtin::dbg_fn(),
        )])]);
        for result in [validate_source(&value), source_form(&value).map(|_| ())] {
            assert!(result.unwrap_err().to_string().contains("source[0][0]"));
        }
        let value = Value::from_expr("(nil true 42 \"hello\" name :key (nested))").unwrap();
        assert_eq!(Value::from(source_form(&value).unwrap()), value);
    }
}
