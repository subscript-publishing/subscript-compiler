//! AST post-parser canonicalization.
use std::rc::Rc;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use itertools::Itertools;

use crate::frontend::data::*;
use crate::frontend::ast::*;


///////////////////////////////////////////////////////////////////////////////
// PARSER AST TO BACKEND AST & NORMALIZATION
///////////////////////////////////////////////////////////////////////////////


pub fn to_unnormalized_backend_ir<'a>(children: Vec<Node<'a>>) -> Vec<Node<'a>> {
    let mut results: Vec<Node> = Default::default();
    for child in children {
        let last = {
            let mut valid_left_pos = None;
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
            Node::Tag(..) => unimplemented!(),
            Node::Enclosure(node) if last_is_ident && node.data.is_square_parens() => {
                let last = last.unwrap();
                let mut name = last
                    .unwrap_ident()
                    .unwrap()
                    .clone();
                let parameters = to_unnormalized_backend_ir(node.data.children);
                let new_node = Node::Tag(Tag {
                    name: name.clone(),
                    parameters: Some(parameters),
                    children: Vec::new(),
                    rewrite_rules: Vec::new(),
                });
                *last = new_node;
                None
            }
            Node::Enclosure(node) if last_is_ident && node.data.is_curly_brace() => {
                let last = last.unwrap();
                let mut name = last
                    .unwrap_ident()
                    .unwrap()
                    .clone();
                let children = to_unnormalized_backend_ir(node.data.children);
                let new_node = Node::Tag(Tag {
                    name,
                    parameters: None,
                    children: vec![
                        Node::unannotated_enclosure(
                            EnclosureKind::CurlyBrace,
                            children,
                        )
                    ],
                    rewrite_rules: Vec::new(),
                });
                *last = new_node;
                None
            }
            Node::Enclosure(node) if last_is_tag && node.data.is_curly_brace() => {
                let tag = last.unwrap();
                let children = to_unnormalized_backend_ir(node.data.children);
                tag.unwrap_tag_mut()
                    .unwrap()
                    .children
                    .push(Node::unannotated_enclosure(
                        EnclosureKind::CurlyBrace,
                        children,
                    ));
                None
            }
            Node::Enclosure(node) => {
                let children = to_unnormalized_backend_ir(node.data.children);
                let new_node = Node::unannotated_enclosure(
                    node.data.kind,
                    children,
                );
                Some(new_node)
            }
            Node::Ident(node) => {
                let new_node = Node::Ident(node);
                Some(new_node)
            }
            Node::InvalidToken(node) => {
                let new_node = Node::String(node);
                Some(new_node)
            }
            Node::String(node) => {
                let mut is_token = false;
                for sym in TOKEN_SET {
                    if *sym == &node.data {
                        is_token = true;
                        break;
                    }
                }
                if is_token {
                    Some(Node::String(node))
                } else {
                    Some(Node::String(node))
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
    children: Vec<Node<'a>>
) -> Vec<RewriteRule<Node<'a>>> {
    let mut results = Vec::new();
    for ix in 0..children.len() {
        if ix == 0 {
            continue;
        }
        let left = children.get(ix - 1);
        let current = children
            .get(ix)
            .and_then(|x| x.unwrap_string())
            .filter(|x| &x.data == "=>");
        let right = children.get(ix + 1);
        match (left, current, right) {
            (Some(left), Some(_), Some(right)) => {
                results.push(RewriteRule {
                    from: left.clone(),
                    to: right.clone(),
                })
            }
            _ => ()
        }
    }
    results
}

pub fn block_level_normalize<'a>(children: Vec<Node<'a>>) -> Vec<Node<'a>> {
    let mut results = Vec::new();
    for child in children {
        if child.is_named_block("!where") {
            let child = child.into_tag().unwrap();
            let last = results
                .last_mut()
                .and_then(Node::unwrap_tag_mut);
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

pub fn parameter_level_normalize_pass(node: Node) -> Node {
    fn go(parameters: Vec<Node>) -> Vec<Node> {
        parameters
            .iter()
            .filter_map(Node::get_string)
            .map(|x| x.data)
            .collect::<Vec<_>>()
            .join("")
            .split_whitespace()
            .map(ToOwned::to_owned)
            .map(|x| Node::String(Ann::unannotated(Cow::Owned(x))))
            .collect::<Vec<_>>()
    }
    match node {
        Node::Tag(mut tag) => {
            tag.parameters = tag.parameters.map(go);
            Node::Tag(tag)
        }
        x => x
    }
}


/// Parses the given source code and returns a normalized backend AST vector.
pub fn run_compiler_frontend<'a>(source: &'a str) -> Vec<Node<'a>> {
    // PARSE SOURCE CODE
    let children = crate::frontend::parser::parse_source(source);
    // NORMALIZE IR
    let children = to_unnormalized_backend_ir(children);
    // NORMALIZE IR
    let node = Node::new_fragment(children)
        .transform_children(Rc::new(block_level_normalize))
        .transform(
            NodeEnvironment::default(),
            Rc::new(|_, x| parameter_level_normalize_pass(x))
        );
    // DONE
    node.into_fragment()
}
