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
use crate::frontend;
use crate::frontend::data::*;


///////////////////////////////////////////////////////////////////////////////
// PARSER AST
///////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone)]
pub enum Node<'a> {
    Ident(Atom<'a>),
    Enclosure(Enclosure<'a, Node<'a>>),
    String(Atom<'a>),
}

///////////////////////////////////////////////////////////////////////////////
// PARSER IR TO FRONTEND AST NORMALIZATION
///////////////////////////////////////////////////////////////////////////////

fn to_frontend_ir<'a>(children: Vec<Node<'a>>) -> Vec<crate::frontend::ast::Ast<'a>> {
    use crate::frontend;
    let mut results: Vec<frontend::Ast> = Default::default();
    for child in children {
        let last = {
            let mut valid_left_pos = None;
            // let cursor = results.cursor_back_mut();
            for ix in (0..results.len()).rev() {
                let leftward = results
                    .get(ix)
                    .filter(|x| !x.is_whitespace());
                if valid_left_pos.is_none() && leftward.is_some() {
                    valid_left_pos = Some(ix);
                    break;
                }
            }
            // results.back_mut()
            // unimplemented!()
            if let Some(ix) = valid_left_pos {
                results.get_mut(ix)
            } else {
                None
            }
        };
        let last_is_ident = last.as_ref().map(|x| x.is_ident()).unwrap_or(false);
        let last_is_tag = last.as_ref().map(|x| x.is_tag()).unwrap_or(false);
        // RETURN NONE IF CHILD IS ADDED TO SOME EXISTING NODE
        let new_child = match child {
            Node::Enclosure(node) if last_is_ident && node.is_square_parens() => {
                let last = last.unwrap();
                let mut name = last.clone().into_ident().unwrap();
                let parameters = to_frontend_ir(node.children);
                let new_node = frontend::Ast::Tag(frontend::Tag {
                    name,
                    parameters: Some(parameters),
                    children: Vec::new(),
                    rewrite_rules: Vec::new(),
                });
                *last = new_node;
                None
            }
            Node::Enclosure(node) if last_is_ident && node.is_curly_brace() => {
                let last = last.unwrap();
                let mut name = last.clone().into_ident().unwrap();
                let children = to_frontend_ir(node.children);
                let new_node = frontend::Ast::Tag(frontend::Tag {
                    name,
                    parameters: None,
                    children: vec![
                        frontend::ast::Ast::Enclosure(
                            frontend::Enclosure {
                                kind: frontend::EnclosureKind::CurlyBrace,
                                children,
                            }
                        )
                    ],
                    rewrite_rules: Vec::new(),
                });
                *last = new_node;
                None
            }
            Node::Enclosure(node) if last_is_tag && node.is_curly_brace() => {
                let tag = last.unwrap();
                let children = to_frontend_ir(node.children);
                tag.unpack_tag_mut()
                    .unwrap()
                    .children.push(frontend::ast::Ast::Enclosure(
                        frontend::Enclosure {
                            kind: frontend::EnclosureKind::CurlyBrace,
                            children,
                        }
                    ));
                None
            }
            Node::Enclosure(node) => {
                let children = to_frontend_ir(node.children);
                let new_node = frontend::Ast::Enclosure(Enclosure{
                    kind: node.kind,
                    children,
                });
                Some(new_node)
            }
            Node::Ident(node) => {
                let new_node = frontend::Ast::Ident(node);
                Some(new_node)
            }
            Node::String(node) => {
                let mut is_token = false;
                for sym in frontend::data::TOKEN_SET {
                    if *sym == &node {
                        is_token = true;
                        break;
                    }
                }
                if is_token {
                    Some(frontend::Ast::Token(node))
                } else {
                    Some(frontend::Ast::Content(node))
                }
            }
        };
        if let Some(new_child) = new_child {
            results.push(new_child);
        }
    }
    results
}


fn into_rewrite_rules<'a>(
    children: Vec<frontend::Ast<'a>>
) -> Vec<frontend::RewriteRule<frontend::Ast<'a>>> {
    let mut results = Vec::new();
    for ix in 0..children.len() {
        if ix == 0 {
            continue;
        }
        let left = children.get(ix - 1);
        let current = children
            .get(ix)
            .and_then(|x| x.unpack_token())
            .filter(|x| *x == "=>");
        let right = children.get(ix + 1);
        match (left, current, right) {
            (Some(left), Some(_), Some(right)) => {
                results.push(frontend::RewriteRule {
                    from: left.clone(),
                    to: right.clone(),
                })
            }
            _ => ()
        }
    }
    results
}

fn normalize_ir<'a>(children: Vec<frontend::Ast<'a>>) -> Vec<frontend::Ast<'a>> {
    let mut results = Vec::new();
    for child in children {
        if child.is_named_block("!where") {
            let child = child.into_tag().unwrap();
            let last = results  
                .last_mut()
                .and_then(frontend::Ast::unpack_tag_mut);
            if let Some(last) = last {
                let rewrite_rule = into_rewrite_rules(
                    child.children,
                );
                last.rewrite_rules.extend(rewrite_rule);
                continue;
            }
        } else {
            results.push(child);
        }
    }
    results
}


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
                let end_pos = range.end;
                let node = Node::Enclosure(Enclosure {
                    kind: EnclosureKind::new(
                        last.0,
                        kind
                    ),
                    children: last.2.into_iter().collect(),
                });
                parent.2.push_back(node);
            }
            Mode::Ident(ident) => {
                let range = range;
                let mut parent = enclosure_stack.back_mut().unwrap();
                parent.2.push_back(Node::Ident(Atom::Borrowed(ident)));
            }
            Mode::NoOP => {
                let range = range;
                let mut parent = enclosure_stack.back_mut().unwrap();
                let node = Node::String(Cow::Borrowed(current.1));
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
            Mode::Ident(frontend::data::INLINE_MATH_TAG),
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
fn run_parser_internal_ast<'a>(source: &'a str) -> Vec<Node<'a>> {
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


pub fn run_parser<'a>(source: &'a str) -> Vec<crate::frontend::Ast<'a>> {
    let children = run_parser_internal_ast(source);
    let children = to_frontend_ir(children);
    let transfomer = frontend::ast::ChildListTransformer {
        parameters: Rc::new(std::convert::identity),
        block: Rc::new(normalize_ir),
        rewrite_rules: Rc::new(std::convert::identity),
        marker: std::marker::PhantomData
    };
    let node = frontend::Ast::new_fragment(children);
    let node = node.child_list_transformer(Rc::new(transfomer));
    node.into_fragment()
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////



pub(crate) fn dev() {
    let source = include_str!("../../source.txt");
    let result = run_parser(source);
    for node in result {
        println!("{:#?}", node);
    }
}

