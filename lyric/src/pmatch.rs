//! Pattern Matching
use crate::{Extern, Locals, SymbolId, Val};
use std::collections::HashMap;

/// Pattern matching predicate
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern<T: Extern, L: Locals> {
    inner: Val<T, L>,
}

/// Result of pattern match
#[derive(Debug, PartialEq)]
pub struct Matches<T: Extern, L: Locals> {
    bindings: HashMap<SymbolId, Val<T, L>>,
}

impl<T, L> Pattern<T, L>
where
    T: Extern,
    L: Locals,
{
    pub fn from_val(inner: Val<T, L>) -> Self {
        Self { inner }
    }

    /// Check if pattern matches given value
    pub fn is_match(&self, val: &Val<T, L>) -> bool {
        self.matches(val).is_some()
    }

    /// Extract matches
    pub fn matches(&self, val: &Val<T, L>) -> Option<Matches<T, L>> {
        let mut matches = Matches::new();
        if Self::matches_inner(&self.inner, val, &mut matches) {
            Some(matches)
        } else {
            None
        }
    }

    fn matches_inner(pat: &Val<T, L>, val: &Val<T, L>, matches: &mut Matches<T, L>) -> bool {
        use Val::*;

        match pat {
            Symbol(s) => {
                matches.bindings.insert(s.clone(), val.clone());
                true
            }
            List(pat) => match val {
                List(val) if pat.len() == val.len() => pat
                    .iter()
                    .zip(val.iter())
                    .all(|(lhs, rhs)| Self::matches_inner(lhs, rhs, matches)),
                _ => false,
            },
            Nil | Bool(_) | Int(_) | String(_) | Keyword(_) | Lambda(_) | NativeFn(_)
            | Bytecode(_) | Error(_) | Ref(_) | Extern(_) => pat == val,
        }
    }
}

impl<T: Extern, L: Locals> Matches<T, L> {
    pub fn new() -> Self {
        Self {
            bindings: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse, Ref, SymbolId};
    use void::Void;

    type Val = crate::Val<Void, ()>;
    type Pattern = crate::Pattern<Void, ()>;

    #[test]
    fn values_matches_itself() {
        {
            assert!(Pattern::from_val(v("nil")).is_match(&v("nil")));
            assert!(!Pattern::from_val(v("nil")).is_match(&v("0")));
            assert!(!Pattern::from_val(v("nil")).is_match(&v("\"hello\"")));
            assert!(!Pattern::from_val(v("nil")).is_match(&v("true")));
        }
        {
            assert!(Pattern::from_val(v("true")).is_match(&v("true")));
            assert!(Pattern::from_val(v("false")).is_match(&v("false")));
            assert!(!Pattern::from_val(v("true")).is_match(&v("false")));
            assert!(!Pattern::from_val(v("false")).is_match(&v("true")));
        }
        {
            assert!(Pattern::from_val(v("0")).is_match(&v("0")));
            assert!(Pattern::from_val(v("9999")).is_match(&v("9999")));
            assert!(!Pattern::from_val(v("10")).is_match(&v("1")));
            assert!(!Pattern::from_val(v("99")).is_match(&v("0")));
            assert!(!Pattern::from_val(v("9")).is_match(&v("99")));
        }
        {
            assert!(Pattern::from_val(v("\"hello\"")).is_match(&v("\"hello\"")));
            assert!(Pattern::from_val(v("\"hello world\"")).is_match(&v("\"hello world\"")));
            assert!(!Pattern::from_val(v("\"hello\"")).is_match(&v("\"goodbye\"")));
            assert!(!Pattern::from_val(v("\"hello world\"")).is_match(&v("\"hello\"")));
            assert!(!Pattern::from_val(v("\"hello world\"")).is_match(&v("\"world\"")));
        }
        {
            assert!(Pattern::from_val(v(":hello")).is_match(&v(":hello")));
            assert!(Pattern::from_val(v(":hello_world")).is_match(&v(":hello_world")));
            assert!(!Pattern::from_val(v(":hello")).is_match(&v(":goodbye")));
            assert!(!Pattern::from_val(v(":hello_world")).is_match(&v(":hello")));
            assert!(!Pattern::from_val(v(":hello_world")).is_match(&v(":world")));
        }
        {
            let ref1 = Val::Ref(Ref("abc".to_string()));
            let ref2 = Val::Ref(Ref("ABC".to_string()));
            let ref3 = Val::Ref(Ref("abcde".to_string()));

            assert!(Pattern::from_val(ref1.clone()).is_match(&ref1));
            assert!(!Pattern::from_val(ref1.clone()).is_match(&ref2));
            assert!(!Pattern::from_val(ref1.clone()).is_match(&ref3));
            assert!(!Pattern::from_val(ref3).is_match(&ref2));
        }
    }

    #[test]
    fn symbols_matches_all() {
        let pat = Pattern::from_val(v("a"));

        {
            let m = pat.matches(&v("hello")).expect("should match");
            assert_eq!(m.bindings.len(), 1,);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::symbol("hello"))
            );
        }

        {
            let m = pat.matches(&v("5")).expect("should match");
            assert_eq!(m.bindings.len(), 1,);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&Val::Int(5)));
        }

        {
            let m = pat.matches(&v("\"hello\"")).expect("should match");
            assert_eq!(m.bindings.len(), 1,);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::string("hello"))
            );
        }
        {
            let m = pat.matches(&v(":hello")).expect("should match");
            assert_eq!(m.bindings.len(), 1,);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::keyword("hello"))
            );
        }
        {
            let m = pat.matches(&v("()")).expect("should match");
            assert_eq!(m.bindings.len(), 1,);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::List(vec![]))
            );
        }
        {
            let m = pat.matches(&v("(1 2 3)")).expect("should match");
            assert_eq!(m.bindings.len(), 1,);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&v("(1 2 3)")));
        }
    }

    #[test]
    fn list_empty() {
        let pat = Pattern::from_val(v("()"));
        assert!(pat.is_match(&v("()")));
        assert!(!pat.is_match(&v("(1)")));
        assert!(!pat.is_match(&v("\"hello\"")));
    }

    #[test]
    fn list_nonempty() {
        let pat = Pattern::from_val(v("(a b c)"));

        {
            let m = pat.matches(&v("(1 2 3)")).expect("should match");
            assert_eq!(m.bindings.len(), 3,);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&Val::Int(1)));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&Val::Int(2)));
            assert_eq!(m.bindings.get(&SymbolId::from("c")), Some(&Val::Int(3)));
        }

        {
            let m = pat
                .matches(&v("(:one :two \"three\")"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 3);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::keyword("one"))
            );
            assert_eq!(
                m.bindings.get(&SymbolId::from("b")),
                Some(&Val::keyword("two"))
            );
            assert_eq!(
                m.bindings.get(&SymbolId::from("c")),
                Some(&Val::string("three"))
            );
        }

        assert_eq!(pat.matches(&v("\"hello\"")), None);
        assert_eq!(pat.matches(&v("()")), None);
        assert_eq!(pat.matches(&v("(1 :two)")), None)
    }

    #[test]
    fn list_nested() {
        let pat = Pattern::from_val(v("(a b (c d))"));

        {
            let m = pat.matches(&v("(1 2 (3 4))")).expect("should match");
            assert_eq!(m.bindings.len(), 4);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&Val::Int(1)));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&Val::Int(2)));
            assert_eq!(m.bindings.get(&SymbolId::from("c")), Some(&Val::Int(3)));
            assert_eq!(m.bindings.get(&SymbolId::from("d")), Some(&Val::Int(4)));
        }

        {
            let m = pat
                .matches(&v("(:one :two (\"three\" 4))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 4);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::keyword("one"))
            );
            assert_eq!(
                m.bindings.get(&SymbolId::from("b")),
                Some(&Val::keyword("two"))
            );
            assert_eq!(
                m.bindings.get(&SymbolId::from("c")),
                Some(&Val::string("three"))
            );
            assert_eq!(m.bindings.get(&SymbolId::from("d")), Some(&Val::Int(4)));
        }

        assert_eq!(pat.matches(&v("(1 2 3 4)")), None);
        assert_eq!(pat.matches(&v("(1 (2 3) 4)")), None);
        assert_eq!(pat.matches(&v("\"1234\"")), None);
        assert_eq!(pat.matches(&v("()")), None);
    }

    #[test]
    fn list_symbols_and_constants() {
        let pat = Pattern::from_val(v("(a 2 (:three d))"));

        {
            let m = pat
                .matches(&v("(:one 2 (:three 4))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::keyword("one"))
            );
            assert_eq!(m.bindings.get(&SymbolId::from("d")), Some(&Val::Int(4)));
        }

        {
            let m = pat
                .matches(&v("(:one 2 (:three :four))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::keyword("one"))
            );
            assert_eq!(
                m.bindings.get(&SymbolId::from("d")),
                Some(&Val::keyword("four"))
            );
        }

        assert!(!pat.is_match(&v("(:one :two (\"three\" 4))")));
        assert!(!pat.is_match(&v("(1 2 (3 4))")));
        assert!(!pat.is_match(&v("()")));
    }

    fn v(expr: &str) -> Val {
        parse(expr).unwrap().into()
    }
}
