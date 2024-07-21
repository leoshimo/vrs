//! List builtins
use crate::{
    compile, kwargs, parse, Error, Extern, Inst, Lambda, Locals, NativeFn, NativeFnOp, SymbolId,
    Val,
};

/// Language bindng for `list`
pub fn list_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(list ELEM_1 ELEM_2 .. ELEM_N) - Creates a new list containing arguments of form. Each argument ELEM is evaluated.\
              Arguments are optional.".to_string(),
        func: |_, args| Ok(NativeFnOp::Return(Val::List(args.to_vec()))),
    }
}

/// Language bindng for `push`
pub fn push_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(push LIST ELEM) - Creates a new list containing elements of LIST with ELEM appended at end".to_string(),
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
        doc: "(get LIST ATTR) - Returns element within LIST for given ATTR, which can be 0-indexed position in list, or keywords for association lists. Negative indexes return from end of list.".to_string(),
        func: |_, x| match x {
            [Val::List(l), Val::Int(idx)] => {
                let index = if *idx >= 0 {
                    *idx as usize
                } else {
                    l.len() + *idx as usize
                };

                let elem = match l.get(index) {
                    Some(elem) => elem.clone(),
                    None => Val::Nil,
                };
                Ok(NativeFnOp::Return(elem))
            }
            [Val::List(l), Val::Keyword(target)] => Ok(NativeFnOp::Return(
                kwargs::get(l, target).unwrap_or(Val::Nil),
            )),
            _ => Err(Error::UnexpectedArguments(
                "get expects a list and indexing argument".to_string(),
            )),
        },
    }
}

/// Language binding for `len`
pub fn len_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(len LIST) - Returns number of elements in LIST".to_string(),
        func: |_, x| match x {
            [Val::List(l)] => Ok(NativeFnOp::Return(Val::Int(l.len().try_into().unwrap()))),
            _ => Err(Error::UnexpectedArguments(
                "list expects a list argument".to_string(),
            )),
        },
    }
}

// TODO: Revisit this map impl.
/// Language binding for `map`
pub(crate) fn map_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(map LIST CALLABLE) - Creates a new list containing elements of LIST transformed by CALLABLE".to_string(),
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

/// Language binding for `filter`
pub(crate) fn filter_fn<T: Extern, L: Locals>() -> Lambda<T, L> {
    Lambda {
        doc: Some("(filter LIST CALLABLE) - Creates a new list containing elements of LIST filtered by CALLABLE".to_string()),
        params: vec![SymbolId::from("lst"), SymbolId::from("callable")],
        code: compile(
            &parse(
                r#"
            (begin 
                (def filter_result '())
                (map lst (fn (it)
                    (if (callable it) (set filter_result (push filter_result it)))))
                filter_result)
        "#,
            )
            .unwrap()
            .into(),
        )
        .unwrap(),
        parent: None,
    }
}

// TODO: Write lang.ts tests for list bindings?
// TODO: Write lang.ts tests for map
// TODO: Write tests for filter
