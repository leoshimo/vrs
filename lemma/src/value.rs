//! Values in Lemma
//! A value is the result of evaluating an [Form]

use crate::form;

/// A value from evaluating a [Form].
/// All forms can be values, but not all values are forms due to function bindings.
/// [Value] is not serializable, but [Form] is.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    String(String),
    Form(form::Form),
}

impl From<form::Form> for Value {
    fn from(value: form::Form) -> Self {
        match value {
            form::Form::Int(i) => Self::Int(i),
            form::Form::String(s) => Self::String(s),
            _ => Self::Form(value),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{}", s),
            Value::Form(form) => write!(f, "{}", form),
        }
    }
}
