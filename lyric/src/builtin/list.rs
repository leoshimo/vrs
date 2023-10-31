//! List builtins
use crate::{Error, Extern, Inst, Locals, NativeFn, NativeFnOp, SymbolId, Val};

/// Language bindng for `list`
pub fn list_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        symbol: SymbolId::from("list"),
        func: |_, args| Ok(NativeFnOp::Return(Val::List(args.to_vec()))),
    }
}

/// Language bindng for `push`
pub fn push_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        symbol: SymbolId::from("push"),
        func: |_, args| match args {
            [Val::List(l), elem] => {
                let mut l = l.to_vec();
                l.push(elem.clone());
                Ok(NativeFnOp::Return(Val::List(l)))
            }
            _ => Err(Error::UnexpectedArguments(
                "push expects a list and element argument".to_string(),
            )),
        },
    }
}

/// Language bindng for `get`
pub fn get_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        symbol: SymbolId::from("get"),
        func: |_, x| match x {
            [Val::List(l), Val::Int(idx)] => {
                let elem = match l.get(*idx as usize) {
                    Some(elem) => elem.clone(),
                    None => Val::Nil,
                };
                Ok(NativeFnOp::Return(elem))
            }
            [Val::List(l), Val::Keyword(target)] => {
                let result = l
                    .windows(2)
                    .find(|w| matches!(&w[0], Val::Keyword(key) if key == target))
                    .map(|w| w[1].clone())
                    .unwrap_or(Val::Nil);

                Ok(NativeFnOp::Return(result))
            }
            _ => Err(Error::UnexpectedArguments(
                "get expects a list and indexing argument".to_string(),
            )),
        },
    }
}

// TODO: Revisit this map impl.
/// Language binding for `map`
pub(crate) fn map_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        symbol: SymbolId::from("map"),
        func: |_, args| match args {
            [Val::List(l), val] if val.is_callable() => {
                let mut bc = vec![Inst::GetSym(SymbolId::from("list"))];
                for elem in l {
                    bc.extend([
                        Inst::PushConst(val.clone()),
                        Inst::PushConst(elem.clone()),
                        Inst::CallFunc(1),
                    ]);
                }
                bc.push(Inst::CallFunc(l.len()));
                Ok(NativeFnOp::Exec(bc))
            }
            _ => Err(Error::UnexpectedArguments(
                "map expects a list and mapping operation".to_string(),
            )),
        },
    }
}

// TODO: Write lang.ts tests for list bindings?
// TODO: Write lang.ts tests for map
