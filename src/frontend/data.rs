//! Common data types
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Eq)]
pub struct Symbol<'a>(pub Cow<'a, str>);

impl<'a> Symbol<'a> {
    pub fn token_set() -> HashSet<&'static str> {
        HashSet::from_iter(TOKEN_SET.to_owned())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub static TOKEN_SET: &'static [&'static str] = &["\\", "[", "]", "{", "}", "(", ")", "=>", "_", "^"];

fn get_end_kind_for(begin_kind: &str) -> &str {
    match begin_kind {
        "{" => "}",
        "[" => "]",
        "(" => ")",
        _ => unreachable!()
    }
}

fn get_begin_kind_for(end_kind: &str) -> &str {
    match end_kind {
        "}" => "{",
        "]" => "[",
        ")" => "(",
        _ => unreachable!()
    }
}

pub fn is_token<'a>(value: &'a str) -> bool {
    for tk in TOKEN_SET {
        if *tk == value {
            return true;
        }
    }
    false
}
