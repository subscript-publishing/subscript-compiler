//! Common data types
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;

pub type Atom<'a> = Cow<'a, str>;

#[derive(Debug, Clone)]
pub struct Text<'a>(pub Cow<'a, str>);

impl<'a> PartialEq for Text<'a> {
    fn eq(&self, other: &Self) -> bool {
        let left = &*self.0;
        let right = &*other.0;
        left == right
    }
}

impl<'a> PartialEq<str> for Text<'a> {
    fn eq(&self, other: &str) -> bool {
        let left = &*self.0;
        let right = other;
        left == right
    }
}



impl<'a> Text<'a> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn append(self, other: Text<'a>) -> Self {
        Text(self.0 + other.0)
    }
}

impl<'a> std::fmt::Display for Text<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

#[derive(Debug, Clone, PartialEq)]
pub struct RewriteRule<T> {
    pub from: T,
    pub to: T,
}

#[derive(Debug, Clone)]
pub struct CurlyBrace<T>(pub Vec<T>);

#[derive(Debug, Clone)]
pub struct SquareParen<T>(pub Vec<T>);

#[derive(Debug, Clone, PartialEq)]
pub enum EnclosureKind<'a> {
    CurlyBrace,
    SquareParen,
    Parens,
    /// Intenral
    Module,
    Error {
        open: &'a str,
        close: &'a str,
    },
}

impl<'a> EnclosureKind<'a> {
    pub fn new(open: &'a str, close: &'a str) -> EnclosureKind<'a> {
        match (open, close) {
            ("{", "}") => EnclosureKind::CurlyBrace,
            ("[", "]") => EnclosureKind::SquareParen,
            ("(", ")") => EnclosureKind::Parens,
            (open, close) => EnclosureKind::Error {open, close},
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enclosure<'a, T> {
    pub kind: EnclosureKind<'a>,
    pub children: Vec<T>,
}

impl<'a, T> Enclosure<'a, T> {
    pub fn is_curly_brace(&self) -> bool {
        match self.kind {
            EnclosureKind::CurlyBrace => true,
            _ => false,
        }
    }
    pub fn is_square_parens(&self) -> bool {
        match self.kind {
            EnclosureKind::SquareParen => true,
            _ => false,
        }
    }
    pub fn is_parens(&self) -> bool {
        match self.kind {
            EnclosureKind::Parens => true,
            _ => false,
        }
    }
    pub fn is_error(&self) -> bool {
        match self.kind {
            EnclosureKind::Error{..} => true,
            _ => false,
        }
    }
}

pub fn identity<T>(x: T) -> T {x}