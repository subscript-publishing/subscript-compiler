//! The parser herein is supposed to meet the following criteria:
//! * real-time parsing (suitable for IDE syntax highlighting).
//! * zero-copy parsing (only copying pointers).
//! * fault tolerant parsing; again, so it can be used in IDE/text editors.
//! Eventually I’d like to support incremental parsing as well. 
use std::rc::Rc;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
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
    let mut results: LinkedList<frontend::Ast> = LinkedList::new();
    for child in children {
        let mut last = results.back_mut();
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
                    children: children,
                    rewrite_rules: Vec::new(),
                });
                *last = new_node;
                None
            }
            Node::Enclosure(node) if last_is_tag && node.is_curly_brace() => {
                let tag = last.unwrap();
                let children = to_frontend_ir(node.children);
                tag.unpack_tag_mut().unwrap().children.extend(children);
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
            results.push_back(new_child);
        }
    }
    results.into_iter().collect()
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
// PARSER DATA TYPES
///////////////////////////////////////////////////////////////////////////////

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
        (_, "\\", Some(next)) if next == &"{"  => (
            Mode::Ident("[math]"),
            ZipperConsumed::Current,
        ),
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
                parent.1.push_back(Node::Ident(Atom::Borrowed(ident)));
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
// PARSER ENTRYPOINT
///////////////////////////////////////////////////////////////////////////////

fn run_parser_internal_ast<'a>(source: &'a str) -> Vec<Node<'a>> {
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
    feed_processor(words)
}

pub fn run_parser<'a>(source: &'a str) -> Vec<crate::frontend::Ast<'a>> {
    let children = run_parser_internal_ast(source);
    let children = to_frontend_ir(children);
    let transfomer = frontend::ast::ChildListTransformer {
        parameters: Rc::new(std::convert::identity),
        block: Rc::new(normalize_ir),
        rewrite_rules: Rc::new(std::convert::identity),
        marker: std::marker::PhantomData
    };
    let node = frontend::Ast::Enclosure(frontend::Enclosure {
        kind: frontend::EnclosureKind::Module,
        children,
    });
    let node = node.child_list_transformer(Rc::new(transfomer));
    match node {
        frontend::Ast::Enclosure(frontend::Enclosure{kind: _, children}) => {
            children
        }
        x => vec![x]
    }
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn dev() {
    let source = include_str!("../../source.txt");
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


