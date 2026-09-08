//! Source provenance travels with syntax symbols, independently of symbol identity.
//! It survives ordinary macro list operations without changing quoted values or
//! the serialized Form protocol. Compiled call sites keep their own descriptors.
use crate::{Extern, Form, Locals, Val};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSite {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub expression: String,
    pub form: String,
    /// Generated syntax has an origin, not an exact literal call location.
    pub generated: bool,
}

impl SourceSite {
    pub fn synthetic(expression: String) -> Self {
        Self {
            file: "<generated>".into(),
            line: 0,
            column: 0,
            form: expression.clone(),
            expression,
            generated: true,
        }
    }
}

pub(crate) fn attach(form: &mut Form, site: SourceSite) {
    match form {
        Form::Symbol(s) => s.sources.push(Arc::new(site)),
        Form::List(items) => {
            if let Some(first) = items.first_mut() {
                attach(first, site);
            }
        }
        _ => (),
    }
}

pub fn site<T: Extern, L: Locals>(value: &Val<T, L>) -> Option<SourceSite> {
    fn head<T: Extern, L: Locals>(v: &Val<T, L>) -> Option<&crate::SymbolId> {
        match v {
            Val::Symbol(s) => Some(s),
            Val::List(items) => items.first().and_then(head),
            _ => None,
        }
    }
    let name = head(value)?;
    let form = value.to_string();
    name.sources
        .iter()
        .find(|s| s.form == form)
        .map(|s| (**s).clone())
        .or_else(|| {
            name.sources.iter().find(|s| s.generated).map(|s| {
                let mut site = (**s).clone();
                site.form = form.clone();
                site.expression = form;
                site
            })
        })
}

/// Give newly constructed macro syntax the invocation's origin. Existing syntax
/// retains its exact locations; generated calls are explicitly marked as such.
pub(crate) fn expansion_origin<T: Extern, L: Locals>(v: &mut Val<T, L>, origin: &SourceSite) {
    match v {
        Val::Symbol(s) if s.sources.is_empty() => {
            let mut site = origin.clone();
            site.generated = true;
            s.sources.push(Arc::new(site));
        }
        Val::List(items) => {
            for item in items {
                expansion_origin(item, origin);
            }
        }
        _ => (),
    }
}

/// Evaluate source text in the current lexical scope. Transporting the source as
/// text avoids changing the ordinary Form wire representation.
pub(crate) fn eval_source_fn<T: Extern, L: Locals>() -> crate::NativeFn<T, L> {
    crate::NativeFn {
        metadata: vec![],
        doc: "(eval_source SOURCE FILE LINE COLUMN) - Evaluate source in the caller scope, retaining source locations (one-based).".into(),
        func: |_, args| {
            let [Val::String(text), Val::String(file), Val::Int(line), Val::Int(column)] = args else {
                return Err(crate::Error::UnexpectedArguments("eval_source expects source, file, line, column".into()));
            };
            if *line < 1 || *column < 1 { return Err(crate::Error::UnexpectedArguments("source line and column must be positive".into())); }
            let forms = crate::parse_source(text, file, *line as usize, *column as usize)?;
            let body = Val::List(std::iter::once(Val::symbol("begin")).chain(forms.into_iter().map(Val::from)).collect());
            Ok(crate::NativeFnOp::Exec(vec![crate::Inst::Prepare(body)]))
        }
    }
}

/// Build a source-preserving request using the existing evaluation protocol.
pub fn request(text: &str, file: &str, line: usize, column: usize) -> Form {
    Form::List(vec![
        Form::symbol("eval_source"),
        Form::string(text),
        Form::string(file),
        Form::Int(line.min(i32::MAX as usize) as i32),
        Form::Int(column.min(i32::MAX as usize) as i32),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Env, Fiber, Inst, SymbolId};
    type Value = Val<void::Void, ()>;

    #[test]
    fn locations_preserve_unicode_comments_duplicate_forms_and_symbol_identity() {
        let source = "# 東京\n(begin (+ 1 2)\n  (+ 1 2) ((fn (x) x) 9) \"\"\"a (b)\"\"\")";
        let forms = crate::parse_source(source, "sample.ll", 20, 5).unwrap();
        assert_eq!(forms, crate::parse_script(source).unwrap());
        let val: Value = forms[0].clone().into();
        let code = crate::compile(&val).unwrap();
        let sites: Vec<_> = code
            .iter()
            .filter_map(|i| match i {
                Inst::CallAt(_, s) => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(
            sites.iter().map(|s| (s.line, s.column)).collect::<Vec<_>>(),
            vec![(21, 8), (22, 3), (22, 11)]
        );
        assert_eq!(sites[2].expression, "((fn (x) x) 9)");
        let wire = serde_json::to_string(&forms).unwrap();
        assert!(!wire.contains("sample.ll"));
        assert_eq!(serde_json::from_str::<Vec<Form>>(&wire).unwrap(), forms);
        let mut env = Env::<void::Void, ()>::standard();
        let Form::List(items) = &forms[0] else {
            panic!()
        };
        let Form::Symbol(s) = &items[0] else { panic!() };
        env.define(s.clone(), Value::Int(42));
        assert_eq!(env.get(&SymbolId::from("begin")), Some(Value::Int(42)));
    }

    #[tokio::test]
    async fn source_eval_keeps_scope_and_macro_body_locations() {
        let source = "(defn! twice (x) (+ x x))\n(def result (twice 21))";
        let request: Value = request(source, "functions.ll", 8, 1).into();
        let mut f = Fiber::from_val(&request, Env::standard(), ()).unwrap();
        assert_eq!(crate::run(&mut f).await.unwrap(), Value::Int(42));
        assert_eq!(
            f.global_env()
                .lock()
                .unwrap()
                .get(&SymbolId::from("result")),
            Some(Value::Int(42))
        );
        let Some(Value::Lambda(lambda)) =
            f.global_env().lock().unwrap().get(&SymbolId::from("twice"))
        else {
            panic!()
        };
        let site = lambda
            .code
            .iter()
            .find_map(|i| match i {
                Inst::CallAt(_, s) => Some(s),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            (&*site.file, site.line, site.column, &*site.expression),
            ("functions.ll", 8, 18, "(+ x x)")
        );
    }
}
