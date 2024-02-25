use lyric::{Error, Inst};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher, Utf32Str,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::rt::program::{NativeFn, NativeFnOp, Val};

// TODO: Revisit whether fuzzy matching belongs in a Lyric library or extension
// rather than a native binding. For now reuse the GUI's existing matcher.
pub(crate) fn fuzzy_match_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "(fuzzy_match QUERY ITEMS KEY) - Rank original items by KEY's text or list of text fields; exact fields first"
            .into(),
        func: |_, args| match args {
            [Val::String(_), Val::List(_), key] => Ok(NativeFnOp::Exec(vec![
                Inst::PushConst(Val::NativeFn(sort_matches_fn())),
                Inst::PushConst(args[0].clone()),
                Inst::PushConst(args[1].clone()),
                Inst::PushConst(Val::NativeFn(lyric::builtin::list::map_fn())),
                Inst::PushConst(args[1].clone()),
                Inst::PushConst(key.clone()),
                Inst::CallFunc(2),
                Inst::CallFunc(3),
            ])),
            _ => Err(Error::UnexpectedArguments(
                "fuzzy_match expects text, items, and a key function".into(),
            )),
        },
    }
}

fn sort_matches_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "Internal fuzzy matching of evaluated keys".into(),
        func: |_, args| {
            let [Val::String(query), Val::List(items), Val::List(keys)] = args else {
                return Err(Error::UnexpectedArguments(
                    "invalid fuzzy matching keys".into(),
                ));
            };
            let mut fields = vec![];
            for (index, value) in keys.iter().enumerate() {
                let values = match value {
                    Val::List(values) => values.as_slice(),
                    _ => std::slice::from_ref(value),
                };
                for value in values {
                    fields.push(Key {
                        index,
                        text: value.as_string()?.clone(),
                    });
                }
            }
            let matches = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)
                .match_list(fields, &mut Matcher::default());
            let mut scores = vec![None; items.len()];
            for (key, score) in matches {
                let exact = !query.trim().is_empty() && key.text.eq_ignore_ascii_case(query.trim());
                let rank = (exact, score);
                scores[key.index] =
                    Some(scores[key.index].map_or(rank, |old| std::cmp::max(old, rank)));
            }
            let mut ranked = scores
                .into_iter()
                .enumerate()
                .filter_map(|(index, rank)| rank.map(|rank| (index, rank)))
                .collect::<Vec<_>>();
            ranked.sort_by_key(|(_, rank)| std::cmp::Reverse(*rank));
            Ok(NativeFnOp::Return(Val::List(
                ranked
                    .into_iter()
                    .map(|(index, _)| items[index].clone())
                    .collect(),
            )))
        },
    }
}

struct Key {
    index: usize,
    text: String,
}

impl AsRef<str> for Key {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

// Like fuzzy_match, keep this native implementation provisional. Return data,
// not HTML or byte offsets: clients need not share Rust's string indexing.
pub(crate) fn match_excerpt_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "(match_excerpt QUERY TEXT) - Short matching excerpt as text spans and (:match TEXT), or nil".into(),
        func: |_, args| {
            let [Val::String(query), Val::String(text)] = args else {
                return Err(Error::UnexpectedArguments(
                    "match_excerpt expects query and text".into(),
                ));
            };
            if query.trim().is_empty() {
                return Ok(NativeFnOp::Return(Val::Nil));
            }
            let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let graphemes = compact.graphemes(true).collect::<Vec<_>>();
            // Match the same grapheme-leading characters as nucleo's Utf32Str
            // conversion, but keep the complete graphemes for display.
            let chars = graphemes.iter()
                .map(|g| g.chars().next().unwrap())
                .collect::<Vec<_>>();
            let mut indices = vec![];
            let score = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)
                .indices(Utf32Str::Unicode(&chars), &mut Matcher::default(), &mut indices);
            if score.is_none() || indices.is_empty() {
                return Ok(NativeFnOp::Return(Val::Nil));
            }
            indices.sort_unstable();
            indices.dedup();
            let start = (indices[0] as usize).saturating_sub(32);
            let end = (start + 160).min(graphemes.len());
            let mut spans = vec![];
            if start > 0 {
                spans.push(Val::string("…"));
            }
            let mut index = start;
            while index < end {
                let matched = indices.binary_search(&(index as u32)).is_ok();
                let mut next = index + 1;
                while next < end && indices.binary_search(&(next as u32)).is_ok() == matched {
                    next += 1;
                }
                let text = Val::string(&graphemes[index..next].concat());
                spans.push(if matched {
                    Val::List(vec![Val::keyword("match"), text])
                } else {
                    text
                });
                index = next;
            }
            if end < graphemes.len() {
                spans.push(Val::string("…"));
            }
            Ok(NativeFnOp::Return(Val::List(spans)))
        },
    }
}
