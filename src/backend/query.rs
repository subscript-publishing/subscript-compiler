//! AST transformations.
use std::iter::FromIterator;
use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::borrow::Cow;
use serde::{Serialize, Deserialize};
use crate::compiler::data::{
    Atom,
    Text,
    Enclosure,
    EnclosureKind,
    INLINE_MATH_TAG,
    RewriteRule,
};
use crate::backend::{Ast, Tag};
use crate::backend::ast::ChildListTransformer;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeadingKind {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl HeadingKind {
    pub fn from_str(x: &str) -> Option<Self> {
        match x {
            "h1" => Some(HeadingKind::H1),
            "h2" => Some(HeadingKind::H2),
            "h3" => Some(HeadingKind::H3),
            "h4" => Some(HeadingKind::H4),
            "h5" => Some(HeadingKind::H5),
            "h6" => Some(HeadingKind::H6),
            _ => None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    kind: HeadingKind,
    text: String,
}

pub fn query_heading_nodes<'a>(ast: &Ast<'a>) -> Vec<Heading> {
    pub fn go<'a>(ast: &Ast<'a>) -> Vec<Tag<'a>> {
        match ast {
            Ast::Tag(tag) if tag.has_name("h1") => vec![tag.clone()],
            Ast::Tag(tag) if tag.has_name("h2") => vec![tag.clone()],
            Ast::Tag(tag) if tag.has_name("h3") => vec![tag.clone()],
            Ast::Tag(tag) if tag.has_name("h4") => vec![tag.clone()],
            Ast::Tag(tag) if tag.has_name("h5") => vec![tag.clone()],
            Ast::Tag(tag) if tag.has_name("h6") => vec![tag.clone()],
            Ast::Tag(node) => {
                node.children
                    .iter()
                    .flat_map(go)
                    .collect::<Vec<_>>()
            }
            Ast::Enclosure(node) => {
                node.children
                    .iter()
                    .flat_map(go)
                    .collect::<Vec<_>>()
            }
            Ast::Ident(node) => Vec::new(),
            Ast::String(value) => Vec::new(),
        }
    }
    go(ast)
        .into_iter()
        .map(|tag| {
            let kind = HeadingKind::from_str(tag.name()).unwrap();
            let text = tag.children
                .iter()
                .map(Ast::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            Heading{kind, text}
        })
        .collect::<Vec<_>>()
}
