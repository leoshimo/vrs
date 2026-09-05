use crate::{Error, Extern, Form, Locals, NativeFn, NativeFnOp, Val};

pub fn field<'a>(record: &'a [Form], name: &str) -> Option<&'a Form> {
    record
        .chunks_exact(2)
        .find(|pair| pair[0] == Form::keyword(name))
        .map(|pair| &pair[1])
}

pub fn set_field(record: &mut Vec<Form>, name: &str, value: Form) {
    if let Some(index) = record
        .chunks_exact(2)
        .position(|pair| pair[0] == Form::keyword(name))
    {
        record[index * 2 + 1] = value;
    } else {
        record.extend([Form::keyword(name), value]);
    }
}

// Compiler-only attachment for the validated leading `interactive` declaration.
// No public metadata editing API until a concrete command needs one.
pub(crate) fn annotate_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: "Attach interactive argument metadata".into(),
        func: |_, args| {
            let [Val::Lambda(lambda), Val::List(metadata)] = args else {
                return Err(Error::UnexpectedArguments(
                    "expected a lambda and compiler-generated metadata".into(),
                ));
            };
            let mut lambda = lambda.clone();
            lambda.metadata = metadata
                .iter()
                .cloned()
                .map(Form::try_from)
                .collect::<crate::Result<_>>()?;
            Ok(NativeFnOp::Return(Val::Lambda(lambda)))
        },
    }
}

pub(crate) fn meta_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: "(meta VALUE) - Inspect function metadata and its real signature; other values return ()".into(),
        func: |_, args| {
            let [value] = args else { return Err(Error::UnexpectedArguments("meta expects one value".into())); };
            let (mut metadata, doc) = match value {
                Val::Lambda(f) => {
                    let mut metadata = f.metadata.clone();
                    let annotations = match field(&metadata, "args") { Some(Form::List(args)) => args.clone(), _ => vec![] };
                    let args = f.params.iter().enumerate().map(|(i, name)| {
                        let mut arg = match annotations.get(i) { Some(Form::List(fields)) => fields.clone(), _ => vec![] };
                        set_field(&mut arg, "name", Form::Symbol(name.clone()));
                        Form::List(arg)
                    }).collect();
                    set_field(&mut metadata, "args", Form::List(args));
                    (metadata, f.doc.clone())
                }
                Val::NativeFn(f) => (f.metadata.clone(), Some(f.doc.clone())),
                Val::NativeAsyncFn(f) => (f.metadata.clone(), Some(f.doc.clone())),
                _ => return Ok(NativeFnOp::Return(Val::List(vec![]))),
            };
            if field(&metadata, "doc").is_none() {
                if let Some(doc) = doc { set_field(&mut metadata, "doc", Form::String(doc)); }
            }
            Ok(NativeFnOp::Return(Val::from(Form::List(metadata))))
        },
    }
}
