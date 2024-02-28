//! Process-local, data-in/data-out source expansion. Runtime environments never
//! enter the phase VM; captures in the separate phase environment are frozen.
use crate::{env::EnvRef, Bytecode, Env, Error, Extern, Fiber, Form, Inst, Lambda, Locals, NativeFn, NativeFnOp, Result, Signal, SymbolId, Val};
use std::{collections::HashMap, sync::{Arc, Mutex}};

#[derive(Debug, Clone, PartialEq)]
pub enum PhaseExtern {}
impl std::fmt::Display for PhaseExtern {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match *self {} }
}
type PhaseVal = Val<PhaseExtern, ()>;

#[derive(Clone, Debug, Default)]
pub struct MacroEnv { state: Option<Arc<State>> }
#[derive(Debug)]
struct State {
    definitions: HashMap<String, Definition>,
    helpers: EnvRef<PhaseExtern, ()>,
}
#[derive(Debug, Clone)]
struct Definition {
    required: usize,
    rest: bool,
    function: Lambda<PhaseExtern, ()>,
}

#[derive(Debug)]
pub(crate) struct Budget { fuel: usize, calls: usize, nodes: usize }
pub(crate) type BudgetRef = Arc<Mutex<Budget>>;
pub(crate) fn budget() -> BudgetRef { Arc::new(Mutex::new(Budget { fuel: 1_000_000, calls: 10_000, nodes: 1_000_000 })) }
fn fail(message: impl Into<String>) -> Error { Error::Macro(message.into()) }
pub(crate) fn tick(budget: &BudgetRef) -> Result<()> {
    let mut b = budget.lock().unwrap();
    b.fuel = b.fuel.checked_sub(1).ok_or_else(|| fail("phase instruction limit exceeded"))?;
    Ok(())
}

const PHASE_NAMES: &[&str] = &[
    "contains?", "eq?", "+", "-", "list", "list?", "error", "push", "get", "map", "apply", "len", "filter",
    "not?", "ok?", "empty?", "keyword?", "err?", "str", "join", "split", "format", "read", "meta",
    "concat", "slice", "symbol", "symbol?", "keyword", "lambda?", "gensym", "macroexpand", "macroexpand_1",
];

impl MacroEnv {
    fn state(&mut self) -> Arc<State> {
        if self.state.is_none() {
            let mut helpers = Env::standard();
            helpers.retain_names(PHASE_NAMES);
            let helpers = Arc::new(Mutex::new(helpers));
            Env::freeze_graph(&helpers);
            self.state = Some(Arc::new(State { definitions: HashMap::new(), helpers }));
            // No quasiquote bootstrap dependency.
            let when = crate::parse("(defmacro when (test & body) \"Evaluate BODY when TEST is true.\" (list 'if test (concat '(begin) body) nil))").unwrap();
            self.define::<PhaseExtern, ()>(&when.into(), &budget()).expect("standard macro should compile");
        }
        self.state.as_ref().unwrap().clone()
    }

    /// Load definitions without executing runtime code. Intended for host macro libraries.
    pub fn load(&mut self, source: &str) -> Result<()> {
        let mut next = self.clone();
        let b = budget();
        for form in crate::parse_script(source)? {
            let value: PhaseVal = form.into();
            match head(&value) {
                Some("defmacro") => { next.define(&value, &b)?; }
                Some("for_syntax") => { next.helpers(&value, &b)?; }
                _ => return Err(fail("macro libraries accept only defmacro and for_syntax")),
            }
        }
        *self = next;
        Ok(())
    }

    fn define<T: Extern, L: Locals>(&mut self, source: &Val<T, L>, b: &BudgetRef) -> Result<Val<T, L>> {
        let source = source_form(source)?;
        let source: PhaseVal = source.into();
        let args = &source.as_list()?[1..];
        let [Val::Symbol(name), Val::List(params), body @ ..] = args else { return Err(fail("defmacro expects a name, parameter list and body")); };
        if name.as_str().is_empty() || name.as_str().ends_with('!') { return Err(fail("defmacro name must not end in !")); }
        let mut names = vec![];
        let mut rest = false;
        let mut iter = params.iter();
        while let Some(param) = iter.next() {
            let param = param.as_symbol().map_err(|_| fail("macro parameters must be symbols"))?;
            if param.as_str() == "&" {
                let next = iter.next().ok_or_else(|| fail("& needs a final rest parameter"))?.as_symbol()?;
                if next.as_str() == "&" || iter.next().is_some() { return Err(fail("& needs exactly one final rest parameter")); }
                names.push(next.clone()); rest = true; break;
            }
            names.push(param.clone());
        }
        let mut seen = std::collections::HashSet::new();
        if names.iter().any(|name| !seen.insert(name)) { return Err(fail("duplicate macro parameter")); }
        let (doc, body) = match body { [Val::String(doc), body @ ..] => (Some(doc.clone()), body), body => (None, body) };
        if body.is_empty() { return Err(fail("defmacro requires a body")); }
        let state = self.state();
        let body = Val::List(std::iter::once(Val::symbol("begin")).chain(body.iter().cloned()).collect());
        let body = self.expression(&body, b, 0)?;
        let function = Lambda { metadata: vec![], doc, params: names.clone(), code: crate::compile(&body)?, parent: Some(state.helpers.clone()) };
        let mut definitions = state.definitions.clone();
        definitions.insert(name.as_str().into(), Definition { required: names.len() - usize::from(rest), rest, function });
        self.state = Some(Arc::new(State { definitions, helpers: state.helpers.clone() }));
        Ok(Val::Symbol(name.clone()))
    }

    fn helpers<T: Extern, L: Locals>(&mut self, value: &Val<T, L>, b: &BudgetRef) -> Result<()> {
        let source: PhaseVal = source_form(value)?.into();
        let body = Val::List(std::iter::once(Val::symbol("begin")).chain(source.as_list()?[1..].iter().cloned()).collect());
        let state = self.state();
        let body = self.expression(&body, b, 0)?;
        let mut env = Env::extend(&state.helpers);
        env.macros = Some(self.clone());
        let mut fiber = Fiber::from_bytecode(crate::compile(&body)?, env, ());
        fiber.set_phase_budget(b.clone());
        match fiber.start()? { Signal::Done(_) => (), _ => return Err(fail("phase code cannot suspend")) }
        let root = fiber.global_env().clone();
        // Do not retain a phase namespace through a captured helper root: the
        // active phase evaluator supplies its own snapshot for deferred eval.
        root.lock().unwrap().macros = None;
        Env::freeze_graph(&root);
        self.state = Some(Arc::new(State { definitions: state.definitions.clone(), helpers: root }));
        Ok(())
    }

    pub(crate) fn prepare<T: Extern, L: Locals>(&mut self, value: &Val<T,L>, b: &BudgetRef, phase: bool) -> Result<Bytecode<T,L>> {
        self.state();
        let value = self.outer(value, b, false)?;
        match head(&value) {
            Some("begin") => {
                let mut code = vec![];
                for item in &value.as_list()?[1..] {
                    if !code.is_empty() { code.push(Inst::PopTop); }
                    code.push(Inst::Prepare(item.clone()));
                }
                if code.is_empty() { code.push(Inst::PushConst(Val::Nil)); }
                Ok(code)
            }
            Some("defmacro" | "for_syntax") if phase => Err(fail("phase execution cannot register definitions")),
            Some("defmacro") => Ok(vec![Inst::PushConst(self.define(&value, b)?)]),
            Some("for_syntax") => { self.helpers(&value, b)?; Ok(vec![Inst::PushConst(Val::Nil)]) }
            _ => crate::compile(&self.expression(&value, b, 0)?),
        }
    }

    pub(crate) fn outer<T: Extern, L: Locals>(&mut self, value: &Val<T,L>, b: &BudgetRef, once: bool) -> Result<Val<T,L>> {
        self.state();
        let mut result = value.clone();
        let mut trace = vec![];
        loop {
            let Some(name) = head(&result).filter(|name| name.ends_with('!')).map(str::to_owned) else { return Ok(result); };
            if name == "!" || name.ends_with("!!") { return Err(fail(format!("invalid macro invocation {name}"))); }
            trace.push(name.clone());
            if trace.len() > 256 { return Err(fail(format!("macro expansion depth exceeded: {}", trace.join(" -> ")))); }
            let expanded = self.invoke(&name[..name.len()-1], &result, b)
                .map_err(|error| fail(format!("{}: {error}", trace.join(" -> "))))?;
            result = expanded;
            if once { return Ok(result); }
        }
    }

    fn invoke<T: Extern, L: Locals>(&mut self, name: &str, call: &Val<T,L>, b: &BudgetRef) -> Result<Val<T,L>> {
        { let mut budget = b.lock().unwrap(); budget.calls = budget.calls.checked_sub(1).ok_or_else(|| fail("macro invocation limit exceeded"))?; }
        let state = self.state();
        let definition = state.definitions.get(name).ok_or_else(|| fail(format!("undefined macro {name}!")))?;
        let source: PhaseVal = source_form(call)?.into();
        let args = &source.as_list()?[1..];
        if args.len() < definition.required || (!definition.rest && args.len() != definition.required) {
            return Err(fail(format!("{name}! expects {}{} arguments, got {} in {call}", definition.required, if definition.rest { " or more" } else { "" }, args.len())));
        }
        let mut values = args[..definition.required].to_vec();
        if definition.rest { values.push(Val::List(args[definition.required..].to_vec())); }
        let mut code = vec![Inst::PushConst(Val::Lambda(definition.function.clone()))];
        code.extend(values.into_iter().map(Inst::PushConst));
        code.push(Inst::CallFunc(definition.function.params.len()));
        let mut env = Env::extend(&state.helpers);
        env.macros = Some(self.clone());
        let mut fiber = Fiber::from_bytecode(code, env, ());
        fiber.set_phase_budget(b.clone());
        let result = match fiber.start()? { Signal::Done(value) => value, _ => return Err(fail("macro transformer cannot suspend")) };
        let form = source_form_counted(&result, 0, &mut b.lock().unwrap().nodes, "result")?;
        Ok(form.into())
    }

    fn expression<T: Extern, L: Locals>(&mut self, value: &Val<T,L>, b: &BudgetRef, depth: usize) -> Result<Val<T,L>> {
        if depth > 256 { return Err(fail("source expression depth exceeded")); }
        let value = self.outer(value, b, false)?;
        let Val::List(mut items) = value else { return Ok(value); };
        let name = items.first().and_then(|v| if let Val::Symbol(s)=v {Some(s.as_str())} else {None}).unwrap_or("").to_owned();
        match name.as_str() {
            "quote" | "try" => (),
            "quasiquote" => { if items.len() == 2 { items[1] = self.template(&items[1], b, 1, depth+1)?; } }
            "defmacro" | "for_syntax" => return Err(fail(format!("{name} is only allowed at the top level of an evaluation unit"))),
            "def" | "set" => { if items.len() == 3 { items[2] = self.expression(&items[2], b, depth+1)?; } }
            "lambda" | "fn" | "defn" => {
                let mut start = if name == "defn" {3} else {2};
                if matches!(items.get(start), Some(Val::String(_))) { start += 1; }
                if name != "lambda" && items.get(start).and_then(head) == Some("interactive") { start += 1; }
                for item in items.iter_mut().skip(start) { *item = self.expression(item,b,depth+1)?; }
            }
            "let" => {
                if let Some(Val::List(bindings)) = items.get_mut(1) {
                    for binding in bindings {
                        if let Val::List(pair) = binding { if pair.len()==2 { pair[1]=self.expression(&pair[1],b,depth+1)?; } }
                    }
                }
                for item in items.iter_mut().skip(2) { *item=self.expression(item,b,depth+1)?; }
            }
            "match" => {
                if let Some(expr) = items.get_mut(1) { *expr=self.expression(expr,b,depth+1)?; }
                for clause in items.iter_mut().skip(2) {
                    if let Val::List(pair)=clause { if pair.len()==2 { pair[1]=self.expression(&pair[1],b,depth+1)?; } }
                }
            }
            "cond" => {
                for clause in items.iter_mut().skip(1) {
                    if let Val::List(pair)=clause { for item in pair { *item=self.expression(item,b,depth+1)?; } }
                }
            }
            _ => { for item in &mut items { *item = self.expression(item,b,depth+1)?; } }
        }
        Ok(Val::List(items))
    }

    fn template<T: Extern,L:Locals>(&mut self,value:&Val<T,L>, b:&BudgetRef, quote_depth:usize, depth:usize)->Result<Val<T,L>> {
        if depth > 256 { return Err(fail("quasiquote source depth exceeded")); }
        let Val::List(items)=value else {return Ok(value.clone())};
        if items.len()==2 {
            match head(value) {
                Some("quasiquote") => return Ok(Val::List(vec![items[0].clone(),self.template(&items[1],b,quote_depth+1,depth+1)?])),
                Some("unquote" | "unquote-splicing") => {
                    let inner=if quote_depth==1 {self.expression(&items[1],b,depth+1)?} else {self.template(&items[1],b,quote_depth-1,depth+1)?};
                    return Ok(Val::List(vec![items[0].clone(),inner]));
                }
                _=>(),
            }
        }
        Ok(Val::List(items.iter().map(|x|self.template(x,b,quote_depth,depth+1)).collect::<Result<_>>()?))
    }
}

fn head<T:Extern,L:Locals>(value:&Val<T,L>)->Option<&str> {
    match value { Val::List(items) => match items.first() { Some(Val::Symbol(s))=>Some(s.as_str()),_=>None },_=>None }
}
pub fn source_form<T:Extern,L:Locals>(value:&Val<T,L>)->Result<Form> {
    source_form_counted(value,0,&mut 1_000_000,"source")
}
fn source_form_counted<T:Extern,L:Locals>(value:&Val<T,L>,depth:usize,nodes:&mut usize,path:&str)->Result<Form> {
    if depth>256 {return Err(fail(format!("source depth exceeded at {path}")))}
    *nodes=nodes.checked_sub(1).ok_or_else(||fail("source node limit exceeded"))?;
    Ok(match value {
        Val::Nil=>Form::Nil,Val::Bool(x)=>Form::Bool(*x),Val::Int(x)=>Form::Int(*x),Val::String(x)=>Form::String(x.clone()),
        Val::Symbol(x)=>Form::Symbol(x.clone()),Val::Keyword(x)=>Form::Keyword(x.clone()),
        Val::List(items)=>Form::List(items.iter().enumerate().map(|(i,x)|source_form_counted(x,depth+1,nodes,&format!("{path}[{i}]"))).collect::<Result<_>>()?),
        _=>return Err(fail(format!("expected source data at {path}, got {}", value))),
    })
}

pub(crate) fn phase_natives<T:Extern,L:Locals>()->std::collections::HashSet<usize> {
    let mut env: Env<T,L>=Env::standard();
    env.retain_names(PHASE_NAMES);
    let mut allowed:std::collections::HashSet<usize>=env.iter().filter_map(|(_,value)|match value {Val::NativeFn(n)=>Some(n.func as usize),_=>None}).collect();
    allowed.insert(crate::builtin::metadata::annotate_fn::<T,L>().func as usize);
    allowed
}

fn native<T:Extern,L:Locals>(doc:&str,func:crate::types::NativeFnSig<T,L>)->NativeFn<T,L> {
    NativeFn {metadata:vec![],doc:doc.into(),func}
}
pub(crate) fn bind_builtins<T:Extern,L:Locals>(env:&mut Env<T,L>) {
    env.bind_native(SymbolId::from("macroexpand_1"),native("(macroexpand_1 FORM) - Expand one outer macro call as data; quote the call",|f,args|inspect(f,args,true)))
        .bind_native(SymbolId::from("macroexpand"),native("(macroexpand FORM) - Expand outer macro calls until the head is ordinary code; does not walk nested forms",|f,args|inspect(f,args,false)))
        .bind_native(SymbolId::from("gensym"),native("(gensym [HINT]) - Fresh printable source symbol",|_,args| {
            let hint=match args {[]=>"tmp",[Val::String(s)]=>s,_=>return Err(fail("gensym expects an optional string hint"))};
            let hint:String=hint.chars().filter(|c|c.is_ascii_alphanumeric()||*c=='_').take(32).collect();
            Ok(NativeFnOp::Return(Val::symbol(&format!("__lyric_g_{}_{}",nanoid::nanoid!(26),hint))))
        }))
        .bind_native(SymbolId::from("symbol?"),native("(symbol? VALUE)",|_,args|one(args,|v|Ok(Val::Bool(matches!(v,Val::Symbol(_)))))))
        .bind_native(SymbolId::from("lambda?"),native("(lambda? VALUE)",|_,args|one(args,|v|Ok(Val::Bool(matches!(v,Val::Lambda(_)))))))
        .bind_native(SymbolId::from("symbol"),native("(symbol STRING-OR-KEYWORD)",|_,args|one(args,|v|match v {Val::String(s)=>Ok(Val::symbol(s)),Val::Keyword(k)=>Ok(Val::Symbol(k.clone().to_symbol())),_=>Err(fail("symbol expects a string or keyword"))})))
        .bind_native(SymbolId::from("keyword"),native("(keyword STRING-OR-SYMBOL)",|_,args|one(args,|v|match v {Val::String(s)=>Ok(Val::keyword(s)),Val::Symbol(s)=>Ok(Val::Keyword(s.clone().to_keyword())),_=>Err(fail("keyword expects a string or symbol"))})))
        .bind_native(SymbolId::from("concat"),native("(concat LIST ...) - Concatenate lists",|_,args| {
            let mut result=vec![];for arg in args {result.extend_from_slice(arg.as_list()?);if result.len()>1_000_000{return Err(fail("list size limit exceeded"));}}
            Ok(NativeFnOp::Return(Val::List(result)))
        }))
        .bind_native(SymbolId::from("slice"),native("(slice LIST START) - List suffix at nonnegative index",|_,args| {
            match args {[Val::List(items),Val::Int(start)] if *start>=0=>Ok(NativeFnOp::Return(Val::List(items.get(*start as usize..).unwrap_or(&[]).to_vec()))),_=>Err(fail("slice expects a list and nonnegative index"))}
        }));
}
fn one<T:Extern,L:Locals>(args:&[Val<T,L>],f:impl FnOnce(&Val<T,L>)->Result<Val<T,L>>)->Result<NativeFnOp<T,L>> {
    match args {[v]=>Ok(NativeFnOp::Return(f(v)?)),_=>Err(fail("expected one argument"))}
}
fn inspect<T:Extern,L:Locals>(fiber:&mut Fiber<T,L>,args:&[Val<T,L>],once:bool)->Result<NativeFnOp<T,L>> {
    let [value]=args else {return Err(fail("macroexpand expects one source-data argument"))};
    source_form(value)?;
    let mut env=fiber.global_env().lock().unwrap().macro_env();
    let result=env.outer(value,&fiber.expansion_budget(),once)?;
    Ok(NativeFnOp::Return(result))
}
