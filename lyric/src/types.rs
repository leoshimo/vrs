//! Types in Lisp virtual machine
use crate::codegen::Inst;
use crate::{Env, Error, Fiber, Ref, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// All values in VM that can be manipulated
#[derive(Debug, Clone, PartialEq)]
pub enum Val<T: Extern, L: Locals> {
    /// No value
    Nil,
    /// True or false
    Bool(bool),
    /// Integers
    Int(i32),
    /// Strings
    String(String),
    /// Named slots for values
    Symbol(SymbolId),
    /// Unique strings
    Keyword(KeywordId),
    /// Lists
    List(Vec<Val<T, L>>),
    /// A callable function object
    Lambda(Lambda<T, L>),
    /// A callable native function object
    NativeFn(NativeFn<T, L>),
    /// Compiled bytecode sequence
    Bytecode(Vec<Inst<T, L>>),
    /// Error as a value
    Error(Error),
    /// References as a value
    Ref(Ref),
    /// Externally defined type as Val
    Extern(T),
}

/// Forms are parsed expression that can be evaluated or be converted to [Val]
/// A serializable subset of [Val]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Form {
    Nil,
    Bool(bool),
    Int(i32),
    String(String),
    RawString(String), // TODO: Remove this w/ new client API
    Symbol(SymbolId),
    Keyword(KeywordId),
    List(Vec<Form>),
}

/// A function object that closes over environment it was created in
#[derive(Clone)]
pub struct Lambda<T: Extern, L: Locals> {
    pub params: Vec<SymbolId>,
    pub code: Vec<Inst<T, L>>,
    pub parent: Option<Arc<Mutex<Env<T, L>>>>,
}

/// A native founction bound to given symbol
#[allow(clippy::type_complexity)]
#[derive(Debug, Clone, PartialEq)]
pub struct NativeFn<T: Extern, L: Locals> {
    pub func: fn(&mut Fiber<T, L>, &[Val<T, L>]) -> Result<NativeFnOp<T, L>>,
}

/// Result of executing native function value
#[derive(Debug, Clone, PartialEq)]
pub enum NativeFnOp<T: Extern, L: Locals> {
    /// Return a value
    Return(Val<T, L>),
    /// Yield a value
    Yield(Val<T, L>),
    /// Execute bytecode-level instructions
    Exec(Vec<Inst<T, L>>),
}

/// Identifier for Symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(String);

/// Identifier for Keywords
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeywordId(String);

/// Trait alias for host defined type in Val (until unstable trait_alias)
pub trait Extern:
    std::fmt::Display + std::fmt::Debug + std::cmp::PartialEq + std::clone::Clone
{
}

/// Trait alias impl for [Extern]
impl<T> Extern for T where
    T: std::fmt::Display + std::fmt::Debug + std::cmp::PartialEq + std::clone::Clone
{
}

/// Trait alias for fiber local storage
pub trait Locals: std::fmt::Debug + std::cmp::PartialEq + std::clone::Clone {}

/// Trait alias impl for [Locals]
impl<T> Locals for T where T: std::fmt::Debug + std::cmp::PartialEq + std::clone::Clone {}

impl Form {
    /// Shorhand for constructing [Form::String]
    pub fn string(s: &str) -> Self {
        Self::String(String::from(s))
    }

    /// Shorthand for constructing [Form::Symbol]
    pub fn symbol(id: &str) -> Self {
        Self::Symbol(SymbolId::from(id))
    }

    /// Shorthand for creating [Form::Keyword]
    pub fn keyword(id: &str) -> Self {
        Self::Keyword(KeywordId::from(id))
    }
}

impl<T: Extern, L: Locals> Val<T, L> {
    /// Shorhand for constructing [Val::String]
    pub fn string(s: &str) -> Self {
        Self::String(String::from(s))
    }

    /// Shorthand for constructing [Val::Symbol]
    pub fn symbol(id: &str) -> Self {
        Self::Symbol(SymbolId::from(id))
    }

    /// Shorthand for creating [Val::Keyword]
    pub fn keyword(id: &str) -> Self {
        Self::Keyword(KeywordId::from(id))
    }

    /// Whether or not val is a callable function
    pub fn is_callable(&self) -> bool {
        matches!(self, Val::Lambda(_) | Val::NativeFn(_))
    }
}

impl SymbolId {
    /// Returns inner ID as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Extern, L: Locals> PartialEq for Lambda<T, L> {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params
            && self.code == other.code
            && ((self.parent.is_none() && other.parent.is_none())
                || Arc::ptr_eq(
                    self.parent.as_ref().unwrap(),
                    other.parent.as_ref().unwrap(),
                ))
    }
}

impl<T: Extern, L: Locals> std::fmt::Display for Val<T, L>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::Nil => write!(f, "nil"),
            Val::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Val::Int(i) => write!(f, "{}", i),
            Val::String(s) => write!(f, "\"{}\"", s),
            Val::Keyword(k) => write!(f, "{}", k),
            Val::Symbol(s) => write!(f, "{}", s),
            Val::List(l) => match &l[..] {
                [quote, form] if quote == &Val::symbol("quote") => {
                    write!(f, "'{}", form)
                }
                _ => write!(
                    f,
                    "({})",
                    l.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            },
            Val::Lambda(l) => write!(
                f,
                "<lambda ({})>",
                l.params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Val::NativeFn(_) => write!(f, "<nativefn>"),
            Val::Bytecode(_) => write!(f, "<bytecode>"),
            Val::Error(e) => write!(f, "<error {e}>"),
            Val::Ref(r) => write!(f, "<ref {}>", r.0),
            Val::Extern(e) => write!(f, "{e}"),
        }
    }
}

impl std::fmt::Display for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Form::Nil => write!(f, "nil"),
            Form::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Form::Int(i) => write!(f, "{}", i),
            Form::String(s) => write!(f, "\"{}\"", s),
            Form::Keyword(k) => write!(f, "{}", k),
            Form::Symbol(s) => write!(f, "{}", s),
            Form::List(l) => match &l[..] {
                [quote, form] if quote == &Form::Symbol(SymbolId::from("quote")) => {
                    write!(f, "'{}", form)
                }
                _ => write!(
                    f,
                    "({})",
                    l.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            },
            Form::RawString(s) => write!(f, "{}", s),
        }
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for KeywordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ":{}", self.0)
    }
}

impl<T: Extern, L: Locals> std::fmt::Debug for Lambda<T, L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // don't blow the stack via env
        let params = self
            .params
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        write!(f, "Form::Lambda({} ...)", params)
    }
}

impl<T: Extern, L: Locals> From<Form> for Val<T, L> {
    fn from(value: Form) -> Self {
        match value {
            Form::Nil => Val::Nil,
            Form::Bool(b) => Val::Bool(b),
            Form::Int(i) => Val::Int(i),
            Form::String(s) => Val::String(s),
            Form::Symbol(s) => Val::Symbol(s),
            Form::Keyword(k) => Val::Keyword(k),
            Form::List(l) => Val::List(l.into_iter().map(|e| e.into()).collect()),
            Form::RawString(s) => Val::String(s),
        }
    }
}

impl<T: Extern, L: Locals> TryFrom<Val<T, L>> for Form {
    type Error = Error;

    fn try_from(value: Val<T, L>) -> std::result::Result<Self, Self::Error> {
        match value {
            Val::Nil => Ok(Form::Nil),
            Val::Bool(b) => Ok(Form::Bool(b)),
            Val::Int(i) => Ok(Form::Int(i)),
            Val::String(s) => Ok(Form::String(s)),
            Val::Symbol(s) => Ok(Form::Symbol(s)),
            Val::Keyword(k) => Ok(Form::Keyword(k)),
            Val::List(l) => Ok(Form::List(
                l.into_iter()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<_>>>()?,
            )),
            Val::Ref(_)
            | Val::Error(_)
            | Val::Bytecode(_)
            | Val::Lambda(_)
            | Val::NativeFn(_)
            | Val::Extern(_) => Ok(Form::RawString(value.to_string())),
        }
    }
}

impl From<String> for SymbolId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SymbolId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for KeywordId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for KeywordId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use void::Void;

    type Val = super::Val<Void, Void>;

    #[test]
    fn nil_to_string() {
        assert_eq!(Val::Nil.to_string(), "nil");
    }

    #[test]
    fn bool_to_string() {
        assert_eq!(Val::Bool(true).to_string(), "true");
        assert_eq!(Val::Bool(false).to_string(), "false");
    }

    #[test]
    fn int_to_string() {
        assert_eq!(Val::Int(5).to_string(), "5");
        assert_eq!(Val::Int(0).to_string(), "0");
        assert_eq!(Val::Int(-99).to_string(), "-99");
    }

    #[test]
    fn string_to_string() {
        assert_eq!(Val::string("hello").to_string(), "\"hello\"");
        assert_eq!(
            Val::string("  hello  world  ").to_string(),
            "\"  hello  world  \"",
        );
    }

    #[test]
    fn symbol_to_string() {
        assert_eq!(Val::symbol("hello").to_string(), "hello");
    }

    #[test]
    fn keyword_to_string() {
        assert_eq!(Val::keyword("hello").to_string(), ":hello");
    }

    #[test]
    fn list_to_string() {
        assert_eq!(
            Val::List(vec![
                Val::symbol("my-func"),
                Val::Int(5),
                Val::string("string"),
            ])
            .to_string(),
            "(my-func 5 \"string\")"
        );
        assert_eq!(
            Val::List(vec![
                Val::symbol("hello"),
                Val::List(vec![
                    Val::symbol("world"),
                    Val::List(vec![Val::keyword("a_keyword"),])
                ]),
                Val::string("string"),
                Val::Int(10),
                Val::Int(-99),
            ])
            .to_string(),
            "(hello (world (:a_keyword)) \"string\" 10 -99)"
        );
    }

    #[test]
    fn quoted_to_string() {
        assert_eq!(
            Val::List(vec![Val::symbol("quote"), Val::symbol("hello")]).to_string(),
            "'hello"
        );
        assert_eq!(
            Val::List(vec![Val::symbol("quote"), Val::List(vec![])]).to_string(),
            "'()"
        );
        assert_eq!(
            Val::List(vec![
                Val::symbol("quote"),
                Val::List(vec![Val::Int(1), Val::Int(2), Val::Int(3),])
            ])
            .to_string(),
            "'(1 2 3)"
        );
        assert_eq!(
            Val::List(vec![
                Val::symbol("quote"),
                Val::List(vec![
                    Val::Int(1),
                    Val::Int(2),
                    Val::Int(3),
                    Val::List(vec![
                        Val::symbol("quote"),
                        Val::List(vec![Val::Int(4), Val::Int(5), Val::Int(6),])
                    ])
                ]),
            ])
            .to_string(),
            "'(1 2 3 '(4 5 6))"
        );
    }
}
