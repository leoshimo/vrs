//! Keyword list utilities

use crate::{Extern, KeywordId, Locals, Val};

/// Get the value associated with `keyword` from `lst`, if any.
pub fn get<T: Extern, L: Locals>(lst: &[Val<T, L>], target: &KeywordId) -> Option<Val<T, L>> {
    lst.windows(2)
        .find(|w| matches!(&w[0], Val::Keyword(key) if key == target))
        .map(|w| w[1].clone())
}

// TODO: Tests for `kwargs::get`
