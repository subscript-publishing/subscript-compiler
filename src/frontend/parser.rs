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
use crate::backend::data::*;
use crate::frontend::ast::*;



///////////////////////////////////////////////////////////////////////////////
// PARSER BASICS
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

type BeginEnclosureStack<'a> = VecDeque<(&'a str, CharIndex, LinkedList<Node<'a>>)>;



// MAIN ENTRYPOINT FOR STRING TO PARSER AST 
pub fn parse_source<'a>(source: &'a str) -> Vec<Node<'a>> {
    into_ast(init_words(source, init_characters(source)))
}


///////////////////////////////////////////////////////////////////////////////
// PARSER ENTRYPOINT
///////////////////////////////////////////////////////////////////////////////



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
struct Character<'a> {
    range: CharRange,
    char: &'a str,
}

impl<'a> Character<'a> {
    pub fn is_whitespace(&self) -> bool {
        self.char.chars().any(|x| x.is_whitespace())
    }
}

fn init_characters<'a>(source: &'a str) -> Vec<Character<'a>> {
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
                    char_index: pos
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
struct Word<'a> {
    range: CharRange,
    word: &'a str,
}

impl<'a> Word<'a> {
    pub fn is_whitespace(&self) -> bool {
        self.word.trim().is_empty()
    }
}

fn init_words<'a>(source: &'a str, chars: Vec<Character<'a>>) -> Vec<Word<'a>> {
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
            Mode::Ident(backend::data::INLINE_MATH_TAG),
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

fn into_ast<'a>(words: Vec<Word<'a>>) -> Vec<Node<'a>> {
    let mut enclosure_stack: BeginEnclosureStack = BeginEnclosureStack::new();
    enclosure_stack.push_front(("[root]", CharIndex::zero(), Default::default()));
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
                enclosure_stack.push_back(
                    (kind, start_pos, Default::default())
                );
            }
            Mode::EndEnclosure {kind} => {
                let mut last = enclosure_stack.pop_back().unwrap();
                let mut parent = enclosure_stack.back_mut().unwrap();
                let start = last.1;
                let end = current.range.end;
                let node = Node::Enclosure(Ann {
                    start,
                    end,
                    data: Enclosure {
                        kind: EnclosureKind::new(
                            last.0,
                            kind
                        ),
                        children: last.2.into_iter().collect(),
                    }
                });
                parent.2.push_back(node);
            }
            Mode::Ident(ident) => {
                let mut parent = enclosure_stack.back_mut().unwrap();
                let start = current.range.start;
                let end = next
                    .map(|x| x.1.range.end)
                    .unwrap_or(current.range.end);
                parent.2.push_back(Node::Ident(Ann {
                    start,
                    end,
                    data: Atom::Borrowed(ident)
                }));
            }
            Mode::NoOP => {
                let mut parent = enclosure_stack.back_mut().unwrap();
                let start = current.range.start;
                let end = current.range.end;
                let node = Node::String(Ann {
                    start,
                    end,
                    data: Cow::Borrowed(current.word)
                });
                parent.2.push_back(node);
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
    enclosure_stack
        .into_iter()
        .flat_map(|(_, _, xs)| xs)
        .collect::<Vec<_>>()
}


pub(crate) fn dev() {
    let source = include_str!("../../source.txt");
    // use itertools::Itertools;
    // let words = source
    //     .grapheme_indices(true)
    //     .enumerate()
    //     .map(|(cix, (bix, x))| {
    //         let index = CharIndex {
    //             byte_index: bix,
    //             char_index: cix,
    //         };
    //         (index, x)
    //     })
    //     .collect_vec();
    // // let mut output = 
    // for pos in 0..words.len() {
    //     let current = words[pos];
    //     let right = words.get(pos + 1);
    // }
    let results = into_ast(init_words(source, init_characters(source)));
    for entry in results {
        println!("{:#?}", entry);
    }
}

