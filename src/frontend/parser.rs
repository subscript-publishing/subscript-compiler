//! The parser herein is supposed to meet the following criteria:
//! * real-time parsing (suitable for IDE syntax highlighting).
//! * zero-copy parsing (only copying pointers).
//! * fault tolerant parsing; again, so it can be used in IDE/text editors.
//! Eventually I’d like to support incremental parsing as well. 
use std::rc::Rc;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use serde::de::value;
use unicode_segmentation::UnicodeSegmentation;

use crate::backend;
use crate::compiler::data::*;
use crate::frontend::ast::*;



///////////////////////////////////////////////////////////////////////////////
// INTERNAL PARSER TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct Zipper<T> {
    left: Option<T>,
    current: T,
    right: Option<T>,
}

enum ZipperConsumed {
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

// type BeginEnclosureStack<'a> = VecDeque<(&'a str, CharIndex, LinkedList<Node<'a>>)>;

#[derive(Debug, Clone, PartialEq)]
pub enum OpenToken {
    CurlyBrace,
    SquareParen,
    Parens,
}

impl OpenToken {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenToken::CurlyBrace => "{",
            OpenToken::SquareParen => "[",
            OpenToken::Parens => "(",
        }
    }
    pub fn new<'a>(token: Atom<'a>) -> Option<OpenToken> {
        let token: &str = &token;
        match (token) {
            ("{") => Some(OpenToken::CurlyBrace),
            ("[") => Some(OpenToken::SquareParen),
            ("(") => Some(OpenToken::Parens),
            (_) => None,
        }
    }
}

#[derive(Debug, Clone)]
struct PartialBlock<'a> {
    open_token: Ann<OpenToken>,
    children: LinkedList<Node<'a>>,
}

#[derive(Debug, Clone)]
enum Branch<'a> {
    PartialBlock(PartialBlock<'a>),
    Node(Node<'a>),
}

#[derive(Debug, Default)]
pub struct ParseTree<'a> {
    scopes: VecDeque<PartialBlock<'a>>,
    finalized: LinkedList<Node<'a>>,
}

///////////////////////////////////////////////////////////////////////////////
// PARSE-TREE UTILS
///////////////////////////////////////////////////////////////////////////////

impl<'a> ParseTree<'a> {
    fn add_child_node(&mut self, new_node: Node<'a>) {
        match self.scopes.back_mut() {
            Some(scope) => {
                scope.children.push_back(new_node);
            }
            None => {
                self.finalized.push_back(new_node);
            }
        }
    }
    fn open_new_enclosure(&mut self, new_enclosure: PartialBlock<'a>) {
        self.scopes.push_back(new_enclosure);
    }
    fn close_last_enclosure(&mut self, close_word: &Word<'a>) {
        match self.scopes.pop_back() {
            Some(scope) => {
                let new_node = Enclosure {
                    kind: EnclosureKind::new(
                        Cow::Borrowed(scope.open_token.data.as_str()),
                        Cow::Borrowed(close_word.word),
                    ),
                    children: scope.children.into_iter().collect()
                };
                self.add_child_node(Node::Enclosure(Ann {
                    start: scope.open_token.start,
                    end: close_word.range.end,
                    data: new_node,
                }));
            }
            None => {
                let new_node = Node::InvalidToken(Ann{
                    start: close_word.range.start,
                    end: close_word.range.end,
                    data: Cow::Borrowed(close_word.word),
                });
                self.add_child_node(new_node);
            }
        }
    }
    pub fn finalize_all(self) -> Vec<Node<'a>> {
        let ParseTree { mut scopes, mut finalized } = self;
        let scopes = scopes.drain(..);
        let xs = scopes
            .map(|scope| {
                let enclosure = Enclosure{
                    kind: EnclosureKind::Error{
                        open: Cow::Borrowed(scope.open_token.data.as_str()),
                        close: None
                    },
                    children: scope.children.into_iter().collect()
                };
                Node::Enclosure(Ann {
                    start: scope.open_token.start,
                    end: scope.open_token.end,
                    data: enclosure,
                })
            });
        finalized.extend(xs);
        finalized.into_iter().collect()
    }
}

///////////////////////////////////////////////////////////////////////////////
// CORE PARSER ENGINE
///////////////////////////////////////////////////////////////////////////////


impl<'a> ParseTree<'a> {
    pub fn parse_words(words: Vec<Word<'a>>) -> Vec<Node<'a>> {
        let mut parse_tree = ParseTree::default();
        let mut skip_to: Option<usize> = None;
        for pos in 0..words.len() {
            if let Some(start_from) = skip_to {
                if pos <= start_from {
                    continue;
                } else {
                    skip_to = None;
                }
            }
            let forward = |by: usize| {
                words
                    .get(pos + by)
                    .filter(|w| !w.is_whitespace())
                    .map(|w| (by, w))
            };
            let current = &words[pos];
            let next = {
                let mut entry = None::<(usize, &Word)>;
                let words_left = words.len() - pos;
                for offset in 1..words_left {
                    assert!(entry.is_none());
                    entry = forward(offset);
                    if entry.is_some() {break}
                }
                entry
            };
            let (mode, consumed) = match_word(
                current.word,
                next.map(|(_, x)| x.word)
            );
            match mode {
                Mode::BeginEnclosure {kind} => {
                    let start_pos = current.range.start;
                    let new_stack = PartialBlock {
                        open_token: Ann {
                            start: current.range.start,
                            end: current.range.end,
                            data: OpenToken::new(Cow::Borrowed(kind)).unwrap()
                        },
                        children: Default::default(),
                    };
                    parse_tree.open_new_enclosure(new_stack);
                }
                Mode::EndEnclosure {kind: close_token} => {
                    parse_tree.close_last_enclosure(current);
                }
                Mode::Ident(ident) => {
                    let start = current.range.start;
                    let end = next
                        .map(|x| x.1.range.end)
                        .unwrap_or(current.range.end);
                    let new_node = Node::Ident(Ann {
                        start,
                        end,
                        data: Atom::Borrowed(ident)
                    });
                    parse_tree.add_child_node(new_node);
                }
                Mode::NoOP => {
                    let start = current.range.start;
                    let end = current.range.end;
                    let new_node = Node::String(Ann {
                        start,
                        end,
                        data: Cow::Borrowed(current.word)
                    });
                    parse_tree.add_child_node(new_node);
                }
            }
            // FINALIZE
            match consumed {
                ZipperConsumed::Current => (),
                ZipperConsumed::Right => {
                    assert!(next.is_some());
                    let offset = next.unwrap().0;
                    skip_to = Some(pos + offset);
                }
            }
        }
        parse_tree.finalize_all()
    }
}



///////////////////////////////////////////////////////////////////////////////
// PARSER ENTRYPOINT
///////////////////////////////////////////////////////////////////////////////


// MAIN ENTRYPOINT FOR STRING TO PARSER AST 
pub fn parse_source<'a>(source: &'a str) -> Vec<Node<'a>> {
    let words = init_words(source, init_characters(source));
    ParseTree::parse_words(words)
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

fn process_word<'a>(values: Vec<(CharIndex, &'a str)>) -> Vec<Vec<(CharIndex, &'a str)>> {
    use itertools::Itertools;
    values
        .into_iter()
        .group_by(|(_, char)| {
            let char: &str = char;
            match char {
                "\\" => true,
                "{" => true,
                "}" => true,
                "[" => true,
                "]" => true,
                "(" => true,
                ")" => true,
                "=" => true,
                ">" => true,
                "_" => true,
                "^" => true,
                _ => false
            }
        })
        .into_iter()
        .flat_map(|(key, group)| -> Vec<Vec<(CharIndex, &str)>> {
            if key == true {
                group
                    .into_iter()
                    .map(|(ix, ch)| {
                        vec![(ix, ch)]
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![group.into_iter().collect::<Vec<_>>()]
            }
        })
        .collect_vec()
}



#[derive(Debug, Clone)]
pub struct Character<'a> {
    range: CharRange,
    char: &'a str,
}

impl<'a> Character<'a> {
    pub fn is_whitespace(&self) -> bool {
        self.char.chars().any(|x| x.is_whitespace())
    }
}

pub fn init_characters<'a>(source: &'a str) -> Vec<Character<'a>> {
    use itertools::Itertools;
    let ending_byte_size = source.len();
    let words = source
        .grapheme_indices(true)
        .enumerate()
        .map(|(cix, (bix, x))| {
            let index = CharIndex {
                byte_index: bix,
                char_index: cix,
            };
            (index, x)
        })
        .collect_vec();
    let mut output = Vec::new();
    for pos in 0..words.len() {
        let (start, current) = words[pos];
        let end = words
            .get(pos + 1)
            .map(|(pos, _)| *pos)
            .unwrap_or_else(|| {
                CharIndex {
                    byte_index: ending_byte_size,
                    char_index: pos + 1
                }
            });
        output.push(Character{
            range: CharRange{ start, end},
            char: current
        });
    }
    output
}

#[derive(Debug, Clone)]
pub struct Word<'a> {
    range: CharRange,
    word: &'a str,
}

impl<'a> Word<'a> {
    pub fn is_whitespace(&self) -> bool {
        self.word.trim().is_empty()
    }
}

pub fn init_words<'a>(source: &'a str, chars: Vec<Character<'a>>) -> Vec<Word<'a>> {
    use itertools::Itertools;
    // let mut output = Vec::new();
    let mut current_word_start = 0usize;
    chars
        .into_iter()
        .group_by(|char| {
            if char.is_whitespace() {
                return true
            }
            match char.char {
                "\\" => true,
                "{" => true,
                "}" => true,
                "[" => true,
                "]" => true,
                "(" => true,
                ")" => true,
                "=" => true,
                ">" => true,
                "_" => true,
                "." => true,
                "^" => true,
                _ => false
            }
        })
        .into_iter()
        .flat_map(|(key, chars)| {
            let chars = chars.into_iter().collect_vec();
            if key || chars.len() < 2 {
                let chars = chars
                    .into_iter()
                    .map(|char| {
                        Word {
                            range: char.range,
                            word: char.char,
                        }
                    })
                    .collect_vec();
                return chars;
            }
            let start = {
                (&chars[0]).range.start
            };
            let end = {
                (&chars[chars.len() - 1]).range.end
            };
            let word = &source[start.byte_index..end.byte_index];
            let word = Word {
                range: CharRange{start, end},
                word,
            };
            vec![word]
        })
        .collect::<Vec<_>>()
}

fn match_word<'a>(current: &'a str, next: Option<&'a str>) -> (Mode<'a>, ZipperConsumed) {
    match (current, next) {
        ("\\", Some(next)) if next == "{"  => (
            Mode::Ident(crate::compiler::data::INLINE_MATH_TAG),
            ZipperConsumed::Current,
        ),
        ("\\", Some(ident)) if !is_token(ident) && ident != " " => (
            Mode::Ident(ident),
            ZipperConsumed::Right
        ),
        (tk @ "{", _) => (
            Mode::BeginEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (tk @ "[", _) => (
            Mode::BeginEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (tk @ "(", _) => (
            Mode::BeginEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (tk @ "}", _) => (
            Mode::EndEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (tk @ "]", _) => (
            Mode::EndEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        (tk @ ")", _) => (
            Mode::EndEnclosure{kind: tk},
            ZipperConsumed::Current
        ),
        _ => (Mode::NoOP, ZipperConsumed::Current),
    }
}




