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
        doc: "(fuzzy_match QUERY ITEMS KEY) - Return original items ranked by matching (KEY item)"
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
            let keys = keys
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    Ok(Key {
                        index,
                        text: value.as_string()?.clone(),
                    })
                })
                .collect::<lyric::Result<Vec<_>>>()?;
            let matches = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)
                .match_list(keys, &mut Matcher::default());
            Ok(NativeFnOp::Return(Val::List(
                matches
                    .into_iter()
                    .map(|(key, _)| items[key.index].clone())
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
