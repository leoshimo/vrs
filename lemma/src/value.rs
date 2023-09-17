//! Values in Lemma
//! A value is the result of evaluating an [Form](crate::Form)

use crate::{form, Env, Form, Result, SymbolId};

// TODO: Reevaluate Form / Value split w.r.t. Quoting, Forms, and Value::Vec
/// A value from evaluating a [Form](crate::Form).
///
/// # Difference between [Form](crate::Form) and [Value]
/// All forms can be values, but not all values are forms due to function
/// bindings, special forms, and macro expansions.
/// [Value] is not serializable, but [Form](crate::Form) is.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Form value
    Form(Form),
    /// Callable function value
    Lambda(Lambda),
    /// Callable special form
    SpecialForm(SpecialForm),
    // TODO: Think - Should Vec be considered a "foreign" value created via `vec` keyword?
    /// Vector of values
    /// This is distinct from a [Value::Form] wrapping [Form::List], which represents pre-evaluated form
    Vec(Vec<Value>),
}

/// A function as a value
#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    pub params: Vec<SymbolId>,
    pub body: Vec<Form>,
}

/// A function that evaluates special forms
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialForm {
    pub symbol: SymbolId,
    pub func: fn(&[Form], &mut Env) -> Result<Value>,
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Form(Form::Bool(value))
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Form(Form::Int(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Form(Form::string(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Form(Form::String(value))
    }
}

impl From<form::Form> for Value {
    fn from(value: form::Form) -> Self {
        Self::Form(value)
    }
}

impl From<Vec<form::Form>> for Value {
    fn from(value: Vec<form::Form>) -> Self {
        Self::Form(Form::List(value))
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::Vec(value)
    }
}

// Conversion from value to form
impl From<Value> for Form {
    fn from(value: Value) -> Self {
        match value {
            Value::Form(f) => f,
            Value::Vec(values) => {
                let mut inner = vec![];
                for v in values {
                    inner.push(Form::from(v));
                }
                Form::List(inner)
            }
            Value::Lambda(l) => Form::List(
                [
                    vec![
                        Form::symbol("lambda"),
                        Form::List(l.params.into_iter().map(Form::Symbol).collect()),
                    ],
                    l.body,
                ]
                .concat(),
            ),
            Value::SpecialForm(sp_form) => Form::Symbol(sp_form.symbol),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Form(form) => write!(f, "{}", form),
            Value::Lambda(lambda) => write!(
                f,
                "<lambda ({})>",
                lambda
                    .params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Value::SpecialForm(s) => write!(f, "<spform {}>", s.symbol),
            Value::Vec(elems) => write!(
                f,
                "<vec ({})>",
                elems
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_form_to_string() {
        assert_eq!(Value::from(true).to_string(), "true");
        assert_eq!(Value::from(false).to_string(), "false");
    }

    #[test]
    fn int_form_to_string() {
        assert_eq!(Value::from(32).to_string(), "32");
    }

    #[test]
    fn string_form_to_string() {
        assert_eq!(Value::from("Hello world").to_string(), "\"Hello world\"");
    }

    #[test]
    fn keyword_form_to_string() {
        assert_eq!(
            Value::from(Form::keyword("a_keyword")).to_string(),
            ":a_keyword"
        );
    }

    #[test]
    fn form_to_string() {
        assert_eq!(Value::Form(Form::symbol("add")).to_string(), "add");

        assert_eq!(Value::Form(Form::keyword("add")).to_string(), ":add");

        assert_eq!(
            Value::Form(Form::List(vec![
                Form::symbol("add"),
                Form::Int(10),
                Form::string("ten"),
            ]))
            .to_string(),
            "(add 10 \"ten\")"
        )
    }

    #[test]
    fn lambda_to_string() {
        assert_eq!(
            Value::Lambda(Lambda {
                params: vec![],
                body: vec![Form::string("Hello world")],
            })
            .to_string(),
            "<lambda ()>",
        );

        assert_eq!(
            Value::Lambda(Lambda {
                params: vec![SymbolId::from("a"), SymbolId::from("b")],
                body: vec![Form::string("Hello world")],
            })
            .to_string(),
            "<lambda (a b)>",
        );
    }

    #[test]
    fn special_form_to_string() {
        assert_eq!(
            Value::SpecialForm(SpecialForm {
                symbol: SymbolId::from("hello"),
                func: |_, _| { Ok(Value::from("value")) },
            })
            .to_string(),
            "<spform hello>"
        );
    }

    #[test]
    fn vec_to_string() {
        assert_eq!(Value::Vec(vec![]).to_string(), "<vec ()>");

        assert_eq!(
            Value::Vec(vec![Value::from(Form::symbol("a_symbol"))]).to_string(),
            "<vec (a_symbol)>"
        );

        assert_eq!(
            Value::Vec(vec![
                Value::from(Form::symbol("one")),
                Value::from(Form::keyword("two")),
                Value::from(Form::Int(3)),
                Value::from(Form::string("four")),
            ])
            .to_string(),
            "<vec (one :two 3 \"four\")>"
        );
    }
}
