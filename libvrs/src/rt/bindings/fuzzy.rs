use lyric::{Error, Inst};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher,
};

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
