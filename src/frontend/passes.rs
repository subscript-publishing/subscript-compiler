//! AST transformations.
use std::rc::Rc;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use itertools::Itertools;

use crate::backend::{self, Ast};
use crate::compiler::data::*;
use crate::frontend::ast::*;


///////////////////////////////////////////////////////////////////////////////
// PARSER AST TO BACKEND AST & NORMALIZATION
///////////////////////////////////////////////////////////////////////////////


pub fn to_unnormalized_backend_ir<'a>(children: Vec<Node<'a>>) -> Vec<crate::backend::ast::Ast<'a>> {
    use crate::backend;
    let mut results: Vec<backend::Ast> = Default::default();
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
            Node::Enclosure(node) if last_is_ident && node.data.is_square_parens() => {
                let last = last.unwrap();
                let mut name = last.clone().into_ident().unwrap();
                let parameters = to_unnormalized_backend_ir(node.data.children);
                let new_node = backend::Ast::Tag(backend::Tag {
                    name,
                    parameters: Some(parameters),
                    children: Vec::new(),
                    rewrite_rules: Vec::new(),
                });
                *last = new_node;
                None
            }
            Node::Enclosure(node) if last_is_ident && node.data.is_curly_brace() => {
                let last = last.unwrap();
                let mut name = last.clone().into_ident().unwrap();
                let children = to_unnormalized_backend_ir(node.data.children);
                let new_node = backend::Ast::Tag(backend::Tag {
                    name,
                    parameters: None,
                    children: vec![
                        backend::ast::Ast::Enclosure(
                            backend::Enclosure {
                                kind: backend::EnclosureKind::CurlyBrace,
                                children,
                            }
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
                tag.unpack_tag_mut()
                    .unwrap()
                    .children
                    .push(backend::ast::Ast::Enclosure(
                        backend::Enclosure {
                            kind: backend::EnclosureKind::CurlyBrace,
                            children,
                        }
                    ));
                None
            }
            Node::Enclosure(node) => {
                let children = to_unnormalized_backend_ir(node.data.children);
                let new_node = backend::Ast::Enclosure(Enclosure{
                    kind: node.data.kind,
                    children,
                });
                Some(new_node)
            }
            Node::Ident(node) => {
                let new_node = backend::Ast::Ident(node.data);
                Some(new_node)
            }
            Node::InvalidToken(node) => {
                let new_node = backend::Ast::Token(node.data);
                Some(new_node)
            }
            Node::String(node) => {
                let mut is_token = false;
                for sym in crate::compiler::data::TOKEN_SET {
                    if *sym == &node.data {
                        is_token = true;
                        break;
                    }
                }
                if is_token {
                    Some(backend::Ast::Token(node.data))
                } else {
                    Some(backend::Ast::Content(node.data))
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
    children: Vec<backend::Ast<'a>>
) -> Vec<backend::RewriteRule<backend::Ast<'a>>> {
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
                results.push(backend::RewriteRule {
                    from: left.clone(),
                    to: right.clone(),
                })
            }
            _ => ()
        }
    }
    results
}

pub fn block_level_normalize<'a>(children: Vec<backend::Ast<'a>>) -> Vec<backend::Ast<'a>> {
    let mut results = Vec::new();
    for child in children {
        if child.is_named_block("!where") {
            let child = child.into_tag().unwrap();
            let last = results
                .last_mut()
                .and_then(backend::Ast::unpack_tag_mut);
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

pub fn parameter_level_normalize(parameters: Vec<backend::Ast>) -> Vec<backend::Ast> {
    parameters
        .iter()
        .filter_map(Ast::get_string)
        .collect::<Vec<_>>()
        .join("")
        .split_whitespace()
        .map(ToOwned::to_owned)
        .map(|x| backend::Ast::Content(Cow::Owned(x)))
        .collect::<Vec<_>>()
}
