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


fn parse_words<'a>(words: Vec<(CharRange, &'a str)>) -> Vec<Node<'a>> {
    let mut enclosure_stack: BeginEnclosureStack = BeginEnclosureStack::new();
    enclosure_stack.push_front(("[root]", CharIndex::zero(), Default::default()));
    let mut skip_next  = false;
    let mut skip_to: Option<usize> = None;
    for word_pos in 0..words.len() {
        if skip_next {
            skip_next = false;
            continue;
        }
        // GET LEFT - TODO REMOVE
        let left = None;
        // GET RIGHT
        let right = words.get(word_pos + 1);
        // GET CURRENT
        let current = words.get(word_pos).unwrap();
        // INIT ZIPPER
        let zipper = Zipper {left, current, right};
        // GO!
        let (mode, consumed) = parse_word(zipper);
        let range = {
            match consumed {
                ZipperConsumed::Right if right.is_some() => {
                    let start = current.0.start;
                    let end = right.unwrap().0.end;
                    CharRange {start, end}
                }
                _ => current.0,
            }
        };
        match mode {
            Mode::BeginEnclosure {kind} => {
                let start_pos = current.0.start;
                enclosure_stack.push_back(
                    (kind, start_pos, Default::default())
                );
            }
            Mode::EndEnclosure {kind} => {
                let mut last = enclosure_stack.pop_back().unwrap();
                let mut parent = enclosure_stack.back_mut().unwrap();
                let start = last.1;
                let end = range.end;
                let end_pos = range.end;
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
                let CharRange{start, end} = range;
                parent.2.push_back(Node::Ident(Ann {
                    start,
                    end,
                    data: Atom::Borrowed(ident)
                }));
            }
            Mode::NoOP => {
                let mut parent = enclosure_stack.back_mut().unwrap();
                let CharRange{start, end} = range;
                let node = Node::String(Ann {
                    start,
                    end,
                    data: Cow::Borrowed(current.1)
                });
                parent.2.push_back(node);
            }
        }
        // FINALIZE
        match consumed {
            ZipperConsumed::Right => {skip_next = true}
            ZipperConsumed::Current => {}
        }
    }
    enclosure_stack
        .into_iter()
        .flat_map(|(_, _, xs)| xs)
        .collect()
}

fn parse_word<'a>(
    // source: &'a str,
    zipper: Zipper<&(CharRange, &'a str)>
) -> (Mode<'a>, ZipperConsumed) {
    let pattern = (
        zipper.left.map(|(_, x)| x),
        zipper.current.1,
        zipper.right.map(|(_, x)| x),
    );
    match pattern {
        (_, "\\", Some(next)) if next == &"{"  => (
            Mode::Ident(backend::data::INLINE_MATH_TAG),
            ZipperConsumed::Current,
        ),
        (_, "\\", Some(ident)) if !is_token(ident) && ident != &" " => (
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
        _ => (Mode::NoOP, ZipperConsumed::Current),
    }
}


// MAIN ENTRYPOINT FOR STRING TO PARSER AST 
pub fn parse_source<'a>(source: &'a str) -> Vec<Node<'a>> {
    use itertools::Itertools;
    let words: Vec<(CharRange, &str)> = source
        .char_indices() // BYTE POSITION
        .enumerate()
        .map(|(cix, (bix, ch))| {
            let char_index = CharIndex {
                byte_index: bix,
                char_index: cix,
            };
            (char_index, ch)
        })
        .group_by(|(_, c)| c.is_whitespace())
        .into_iter()
        .flat_map(|(key, values)| {
            let values = values
                .group_by(|(_, char)| {
                    match char {
                        '\\' => true,
                        '{' => true,
                        '}' => true,
                        '[' => true,
                        ']' => true,
                        '(' => true,
                        ')' => true,
                        '=' => true,
                        '>' => true,
                        '_' => true,
                        '^' => true,
                        _ => false
                    }
                })
                .into_iter()
                .flat_map(|(key, group)| -> Vec<Vec<(CharIndex, char)>> {
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
                .collect_vec();
            // DONE
            values
        })
        .filter(|xs| xs.len() != 0)
        .filter_map(|xs| {
            let range: (CharIndex, CharIndex) = match xs.len() {
                0 => unimplemented!(),
                1 => (xs[0].0, xs[0].0),
                _ => {
                    let start = xs.first().unwrap().0;
                    let end = xs.last().unwrap().0;
                    (start, end)
                }
            };
            let range = CharRange{
                start: range.0,
                end: range.1,
            };
            // println!("{:#?}", xs);
            let word = range.substrng(source)?;
            Some((range, word))
        })
        .collect_vec();
    // println!("{:#?}", words);
    parse_words(words)
}


///////////////////////////////////////////////////////////////////////////////
// PARSER ENTRYPOINT
///////////////////////////////////////////////////////////////////////////////



///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////



pub(crate) fn dev() {
    // let source = include_str!("../../source.txt");
    // let result = run_parser(source);
    // for node in result {
    //     println!("{:#?}", node);
    // }
}

