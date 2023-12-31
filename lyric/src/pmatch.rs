//! Pattern Matching
use crate::{Extern, Locals, Result, SymbolId, Val};
use std::collections::HashMap;

/// Pattern matching predicate
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern<T: Extern, L: Locals> {
    inner: Pat<T, L>,
}

/// Internal implementation of pattern
#[derive(Debug, Clone, PartialEq)]
enum Pat<T: Extern, L: Locals> {
    One(Val<T, L>),
    Multi(Vec<Val<T, L>>),
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
    pub fn from_val(pat: Val<T, L>) -> Self {
        Self {
            inner: Pat::One(pat),
        }
    }

    pub fn from_vals(patterns: Vec<Val<T, L>>) -> Self {
        Self {
            inner: Pat::Multi(patterns),
        }
    }

    pub fn from_expr(expr: &str) -> Result<Self> {
        Ok(Self {
            inner: Pat::One(Val::from_expr(expr)?),
        })
    }

    pub fn from_exprs(exprs: &[&str]) -> Result<Self> {
        Ok(Self {
            inner: Pat::Multi(
                exprs
                    .iter()
                    .map(|p| Val::from_expr(p))
                    .collect::<Result<_>>()?,
            ),
        })
    }

    /// Check if pattern matches given value
    pub fn is_match(&self, val: &Val<T, L>) -> bool {
        self.matches(val).is_some()
    }

    /// Extract matches
    pub fn matches(&self, val: &Val<T, L>) -> Option<Matches<T, L>> {
        match &self.inner {
            Pat::One(pat) => {
                let mut matches = Matches::default();
                if Self::matches_inner(pat, val, &mut matches) {
                    Some(matches)
                } else {
                    None
                }
            }
            Pat::Multi(patterns) => {
                for pat in patterns {
                    let mut matches = Matches::default();
                    if Self::matches_inner(pat, val, &mut matches) {
                        return Some(matches);
                    }
                }

                None
            }
        }
    }

    fn matches_inner(pat: &Val<T, L>, val: &Val<T, L>, matches: &mut Matches<T, L>) -> bool {
        use Val::*;
        match pat {
            Symbol(s) if s.as_str() == "_" => true,
            Symbol(s) => match matches.bindings.get(s) {
                Some(seen) if val == seen => true,
                None => {
                    matches.bindings.insert(s.clone(), val.clone());
                    true
                }
                _ => false,
            },
            List(pat) => match val {
                List(val) if pat.len() == val.len() => pat
                    .iter()
                    .zip(val.iter())
                    .all(|(lhs, rhs)| Self::matches_inner(lhs, rhs, matches)),
                _ => false,
            },
            Nil | Bool(_) | Int(_) | String(_) | Keyword(_) | Lambda(_) | NativeFn(_)
            | NativeAsyncFn(_) | Bytecode(_) | Error(_) | Ref(_) | Extern(_) => pat == val,
        }
    }
}

impl<T: Extern, L: Locals> IntoIterator for Matches<T, L> {
    type Item = (SymbolId, Val<T, L>);
    type IntoIter = std::collections::hash_map::IntoIter<SymbolId, Val<T, L>>;
    fn into_iter(self) -> Self::IntoIter {
        self.bindings.into_iter()
    }
}

impl<T: Extern, L: Locals> std::default::Default for Matches<T, L> {
    fn default() -> Self {
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

    #[test]
    fn repeated_symbols() {
        let pat = Pattern::from_val(v("(a b a)"));

        {
            let m = pat.matches(&v("(1 2 1)")).expect("should match");
            assert_eq!(m.bindings.len(), 2,);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&Val::Int(1)));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&Val::Int(2)));
        }

        assert!(
            !pat.is_match(&v("(1 2 3)")),
            "(1 2 3) does not match (a b a)"
        );
    }

    #[test]
    fn repeated_symbols_nested() {
        let pat = Pattern::from_val(v("(a b (a b))"));

        {
            let m = pat.matches(&v("(1 1 (1 1))")).expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&Val::Int(1)));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&Val::Int(1)));
        }

        {
            let m = pat
                .matches(&v("(:one \"two\" (:one \"two\"))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                Some(&Val::keyword("one"))
            );
            assert_eq!(
                m.bindings.get(&SymbolId::from("b")),
                Some(&Val::string("two"))
            );
        }

        {
            let m = pat
                .matches(&v("((:a :b :c) :d ((:a :b :c) :d))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&v("(:a :b :c)")));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&v(":d")));
        }

        {
            let m = pat
                .matches(&v("(1 (:d :e) (1 (:d :e)))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&Val::Int(1)));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&v("(:d :e)")));
        }

        {
            let m = pat
                .matches(&v("((:a :b :c) (:d :e) ((:a :b :c) (:d :e)))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&v("(:a :b :c)")));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&v("(:d :e)")));
        }

        assert!(!pat.is_match(&v("(1 2 (3 4))")));
    }

    #[test]
    fn underscore_simple() {
        let pat = Pattern::from_val(v("_"));
        {
            let m = pat.matches(&v("0")).expect("should match");
            assert!(m.bindings.is_empty(), "_ should not capture new bindings");
        }
        {
            let m = pat.matches(&v("true")).expect("should match");
            assert!(m.bindings.is_empty(), "_ should not capture new bindings");
        }
        {
            let m = pat.matches(&v("\"hello\"")).expect("should match");
            assert!(m.bindings.is_empty(), "_ should not capture new bindings");
        }
        {
            let m = pat.matches(&v(":hello")).expect("should match");
            assert!(m.bindings.is_empty(), "_ should not capture new bindings");
        }
    }

    #[test]
    fn underscore_nested() {
        let pat = Pattern::from_val(v("(a _ (a _ a))"));
        {
            let m = pat
                .matches(&v("(1 :hello (1 :boop 1))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 1, "should have one binding for `a`");
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&v("1")));
        }
    }

    #[test]
    fn multi_pattern() {
        let pat = Pattern::from_exprs(&["(:one a)", "(:two a b)", "(:three b c)"]).unwrap();

        {
            let m = pat.matches(&v("(:one 1)")).expect("should match");
            assert_eq!(m.bindings.len(), 1);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&v("1")));
        }

        {
            let m = pat.matches(&v("(:two 2 3)")).expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&v("2")));
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&v("3")));
        }

        {
            let m = pat
                .matches(&v("(:three (:one 1) (:two 2 3))"))
                .expect("should match");
            assert_eq!(m.bindings.len(), 2);
            assert_eq!(
                m.bindings.get(&SymbolId::from("a")),
                None,
                "Variables mentioned in other patterns should not be extracted"
            );
            assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&v("(:one 1)")));
            assert_eq!(m.bindings.get(&SymbolId::from("c")), Some(&v("(:two 2 3)")));
        }

        {
            assert!(!pat.is_match(&v("(:four :not :match)")));
            assert!(!pat.is_match(&v("(:one :not :match)")));
            assert!(!pat.is_match(&v("(:two :not :match :either)")));
        }
    }

    #[test]
    fn multi_pattern_earlier_patterns_match_first() {
        let pat = Pattern::from_exprs(&["(a b)", "(a a)"]).unwrap();

        let m = pat.matches(&v("(1 1)")).expect("should match (a b)");
        assert_eq!(
            m.bindings.len(),
            2,
            "Should have two bindings for (a b) - earlier patterns match first"
        );
        assert_eq!(m.bindings.get(&SymbolId::from("a")), Some(&v("1")),);
        assert_eq!(m.bindings.get(&SymbolId::from("b")), Some(&v("1")),);
    }

    fn v(expr: &str) -> Val {
        parse(expr).unwrap().into()
    }
}
