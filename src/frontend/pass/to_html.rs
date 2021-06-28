//! Frontend AST to HTML AST conversion.
use std::iter::FromIterator;
use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::borrow::Cow;
use crate::frontend::data::*;
use crate::frontend::ast::*;
use crate::frontend::pass;

use crate::codegen::html;

/// Ensure that `Node` is first canonicalized!
/// - I.e. make sure the inputs have been passes through the `html_canonicalization` function.
fn node_to_html<'a>(node: Node<'a>) -> html::Node<'a> {
    fn enclosure<'a>(
        start: &'a str,
        children: Vec<Node<'a>>,
        end: &'a str,
    ) -> html::Node<'a> {
        html::Node::Fragment(
            vec![
                vec![html::Node::new_text(start)],
                children.into_iter().map(node_to_html).collect::<Vec<_>>(),
                vec![html::Node::new_text(end)],
            ].concat()
        )
    }
    fn enclosure_cow<'a>(
        start: Atom<'a>,
        children: Vec<Node<'a>>,
        end: Option<Atom<'a>>,
    ) -> html::Node<'a> {
        let end = match end {
            Some(x) => x,
            None => Cow::Owned(String::new()),
        };
        html::Node::Fragment(
            vec![
                vec![html::Node::Text(Text(start))],
                children.into_iter().map(node_to_html).collect::<Vec<_>>(),
                vec![html::Node::Text(Text(end))],
            ].concat()
        )
    }
    fn map_children<'a>(children: Vec<Node<'a>>) -> Vec<html::Node<'a>> {
        children.into_iter().map(node_to_html).collect::<Vec<_>>()
    }
    fn to_html_attributes<'a>(parameters: Vec<Node<'a>>) -> HashMap<Text<'a>, Text<'a>> {
        parameters
            .into_iter()
            .filter_map(|node| -> Option<Text<'a>> {
                match node {
                    Node::String(Ann{data: txt, ..}) if !txt.trim().is_empty() => {
                        Some(Text(txt))
                    }
                    _ => None
                }
            })
            .map(|x| -> (Text<'a>, Text<'a>) {
                if let Some((l, r)) = x.0.split_once("=") {
                    (Text(Cow::Owned(l.to_owned())), Text(Cow::Owned(r.to_owned())))
                } else {
                    (x, Text(Cow::Borrowed("")))
                }
            })
            .collect::<HashMap<_, _>>()
    }
    match node {
        Node::Tag(node) => {
            html::Node::Element(html::Element {
                name: Text(node.name.data),
                attributes: node.parameters
                    .map(to_html_attributes)
                    .unwrap_or_default(),
                children: map_children(node.children),
            })
        },
        Node::Enclosure(Ann{data: Enclosure {
            kind: EnclosureKind::CurlyBrace,
            children
        }, ..}) => {
            enclosure(
                "{",
                children,
                "}"
            )
        },
        Node::Enclosure(Ann{data: Enclosure {
            kind: EnclosureKind::Parens,
            children
        }, ..}) => {
            enclosure(
                "(",
                children,
                ")"
            )
        },
        Node::Enclosure(Ann{data: Enclosure {
            kind: EnclosureKind::Fragment,
            children
        }, ..}) => {
            html::Node::Fragment(map_children(children))
        },
        Node::Enclosure(Ann{data: Enclosure {
            kind: EnclosureKind::SquareParen,
            children
        }, ..}) => {
            enclosure(
                "[",
                children,
                "]"
            )
        },
        Node::Enclosure(Ann{data: Enclosure {
            kind: EnclosureKind::Error{open, close},
            children
        }, ..}) => {
            enclosure_cow(
                open,
                children,
                close
            )
        },
        Node::Ident(Ann{data, ..}) => {
            html::Node::Text(Text::new("\\").append(Text(data)))
        },
        Node::String(Ann{data, ..}) => {
            html::Node::Text(Text(data))
        },
        Node::InvalidToken(Ann{data, ..}) => {
            html::Node::Text(Text(data))
        }
    }
}

pub fn compile_to_html(source: &str) -> String {
    let nodes = crate::frontend::pass::pp_normalize::run_compiler_frontend(source);
    let nodes = pass::html_normalize::html_canonicalization(nodes);
    let nodes = nodes
        .into_iter()
        .map(pass::math::latex_pass)
        .map(node_to_html)
        .collect::<Vec<_>>();
    crate::codegen::html::render_document(nodes)
}

