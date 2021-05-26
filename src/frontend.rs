use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;

///////////////////////////////////////////////////////////////////////////////
// BASICS
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Eq)]
pub struct Symbol<'a>(Cow<'a, str>);

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

impl<'a> Symbol<'a> {
    pub fn token_set() -> HashSet<&'static str> {
        HashSet::from_iter(TOKEN_SET.to_owned())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
pub enum EnclosureKind<'a> {
    CurlyBrace,
    SquareParens,
    Parens,
    Error {
        open: &'a str,
        close: &'a str,
    },
}

impl<'a> EnclosureKind<'a> {
    pub fn new(open: &'a str, close: &'a str) -> EnclosureKind<'a> {
        match (open, close) {
            ("{", "}") => EnclosureKind::CurlyBrace,
            ("[", "]") => EnclosureKind::SquareParens,
            ("(", ")") => EnclosureKind::Parens,
            (open, close) => EnclosureKind::Error {open, close},
        }
    }
}

#[derive(Debug, Clone)]
pub struct Enclosure<'a> {
    kind: EnclosureKind<'a>,
    children: Vec<Node<'a>>,
}

#[derive(Debug, Clone)]
pub enum Node<'a> {
    Ident(Symbol<'a>),
    Token(Symbol<'a>),
    Enclosure(Enclosure<'a>),
    String(Cow<'a, str>),
}

#[derive(Debug, Clone)]
struct State<'a> {
    index: usize,
    word: &'a str,
}

struct Zipper<T> {
    left: Option<T>,
    current: T,
    right: Option<T>,
}

enum ZipperConsumed {
    Nothing,
    Current,
    Right,
}

#[derive(Debug, Clone)]
pub enum Mode<'a> {
    BeginEnclosure {
        kind: &'a str,
    },
    EndEnclosure {
        kind: &'a str,
    },
    Ident(&'a str),
    NoOP,
}


fn tick<'a>(
    // source: &'a str,
    zipper: Zipper<&(usize, &'a str)>
) -> (Mode<'a>, ZipperConsumed) {
    let pattern = (
        zipper.left.map(|(_, x)| x),
        zipper.current.1,
        zipper.right.map(|(_, x)| x),
    );
    match pattern {
        (_, "\\", Some(ident)) if !is_token(ident) => (
            Mode::Ident(ident),
            ZipperConsumed::Right
        ),
        (_, tk @ "{", _) => (
            Mode::BeginEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (_, tk @ "[", _) => (
            Mode::BeginEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (_, tk @ "(", _) => (
            Mode::BeginEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (_, tk @ "}", _) => (
            Mode::EndEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (_, tk @ "]", _) => (
            Mode::EndEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (_, tk @ ")", _) => (
            Mode::EndEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        _ => (
            Mode::NoOP,
            ZipperConsumed::Nothing,
        ),
    }
}

type BeginEnclosureStack<'a> = VecDeque<(&'a str, LinkedList<Node<'a>>)>;

fn feed_processor<'a>(words: Vec<(usize, &'a str)>) -> Vec<Node<'a>> {
    let mut enclosure_stack = BeginEnclosureStack::new();
    enclosure_stack.push_front(("[root]", Default::default()));
    let mut skip_next = false;
    for word_pos in 0..words.len() {
        if skip_next {
            skip_next = false;
            continue;
        }
        let left = {
            if word_pos == 0 {
                None
            } else {
                words.get(word_pos - 1)
            }
        };
        let current = words.get(word_pos).unwrap();
        let right = words.get(word_pos + 1);
        let zipper = Zipper {left, current, right};
        let out = tick(zipper);
        match out {
            (Mode::BeginEnclosure {kind}, _) => {
                enclosure_stack.push_back((kind, Default::default()));
            }
            (Mode::EndEnclosure {kind}, _) => {
                let mut last = enclosure_stack.pop_back().unwrap();
                let mut parent = enclosure_stack.back_mut().unwrap();
                let node = Node::Enclosure(Enclosure {
                    kind: EnclosureKind::new(
                        last.0,
                        kind
                    ),
                    children: {
                        last.1
                        .into_iter()
                        .collect()
                    },
                });
                parent.1.push_back(node);
            }
            (Mode::Ident(ident), _) => {
                let mut parent = enclosure_stack.back_mut().unwrap();
                parent.1.push_back(Node::Ident(Symbol(Cow::Borrowed(ident))));
            }
            (Mode::NoOP, _) => {
                let mut parent = enclosure_stack.back_mut().unwrap();
                let node = Node::String(Cow::Borrowed(current.1));
                parent.1.push_back(node);
            }
        }
        match out.1 {
            ZipperConsumed::Right => {
                skip_next = true;
            }
            ZipperConsumed::Current => {}
            ZipperConsumed::Nothing => {}
        }
    }
    enclosure_stack
        .into_iter()
        .flat_map(|(_, xs)| xs)
        .collect()
}

///////////////////////////////////////////////////////////////////////////////
// INTERNAL HELPERS
///////////////////////////////////////////////////////////////////////////////

fn break_up<'a>(word: &'a str) -> Vec<&'a str> {
    let break_up = TOKEN_SET
        .into_iter()
        .any(|tk| word.contains(tk));
    let mut sub_strings = Vec::new();
    let mut start_pos = 0;
    let mut skip_to: Option<usize> = None;
    for ix in 0..word.len() {
        if let Some(pos) = skip_to {
            if ix < pos {
                continue;
            } else {
                skip_to = None;
            }
        }
        let sub_string = &word[ix..];
        let break_point = TOKEN_SET
            .into_iter()
            .find_map(|tk| {
                if sub_string.starts_with(tk) {
                    Some(tk.clone())
                } else {
                    None
                }
            });
        if let Some(break_point) = break_point {
            let end_pos = ix + break_point.len();
            let left = &word[start_pos..ix];
            let current = &word[ix..end_pos];
            skip_to = Some(ix + break_point.len());
            start_pos = end_pos;
            if !left.is_empty() {
                sub_strings.push(left);
            }
            assert!(!current.is_empty());
            sub_strings.push(current);
        }
    }
    sub_strings.push(&word[start_pos..]);
    sub_strings
}

///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn run() {
    let source = include_str!("../source.txt");
    let words = {
        let mut index = 0;
        source
            .split_whitespace()
            .flat_map(|word| -> Vec<(usize, &str)> {
                break_up(word)
                    .into_iter()
                    .map(|sub| {
                        let ix = index;
                        let current = sub;
                        index = index + sub.len();
                        (ix, sub)
                    })
                    .collect()
            })
            .filter(|x| !x.1.is_empty())
            .collect::<Vec<_>>()
    };
    let top_level = feed_processor(words);
    for node in top_level {
        println!("{:#?}", node);
    }
}


