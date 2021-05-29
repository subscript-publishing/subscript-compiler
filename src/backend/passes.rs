use std::iter::FromIterator;
use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::borrow::Cow;
use crate::backend::data::{
    Atom,
    Text,
    Enclosure,
    EnclosureKind,
    INLINE_MATH_TAG,
    RewriteRule,
};
use crate::backend::{Ast, Tag};
use crate::backend::ast::ChildListTransformer;

///////////////////////////////////////////////////////////////////////////////
// AST-TO-AST PASSES
///////////////////////////////////////////////////////////////////////////////


pub fn match_and_apply_rewrite_rule<'a>(
    pattern: Vec<Ast<'a>>,
    target: Vec<Ast<'a>>,
    children: Vec<Ast<'a>>,
) -> Vec<Ast<'a>> {
    let mut left: Vec<Ast<'a>> = Vec::<Ast>::new();
    let mut current = children;
    while current.len() > 0 && current.len() >= pattern.len() {
        fn is_eq<T: PartialEq>((l, r): (T, T)) -> bool {l == r}
        let matches = current
            .iter()
            .zip(pattern.iter())
            .all(is_eq);
        if matches {
            // ADD NEW PATTENR TO LEFT
            left.extend(target.clone());
            let _ = current
                .drain(..pattern.len())
                .collect::<Vec<_>>();
            continue;
        }
        left.push(current.remove(0));
    }
    left.extend(current);
    left
}


pub fn child_list_passes<'a>(children: Vec<Ast<'a>>) -> Vec<Ast<'a>> {
    // APPLY AFTER REMOVING ALL TOKENS
    fn merge_text_content<'a>(xs: Vec<Ast<'a>>) -> Vec<Ast<'a>> {
        let mut results = Vec::new();
        for current in xs.into_iter() {
            assert!(!current.is_token());
            let left = results
                .last_mut()
                .and_then(Ast::unpack_content_mut);
            if let Some(left) = left {
                if let Some(txt) = current.unpack_content() {
                    *left = left.to_owned() + txt.to_owned();
                    continue;
                }
            }
            results.push(current);
        }
        results
    }
    fn block_passes<'a>(xs: Vec<Ast<'a>>) -> Vec<Ast<'a>> {
        merge_text_content(xs)
    }
    let transfomer = ChildListTransformer {
        parameters: Rc::new(std::convert::identity),
        block: Rc::new(block_passes),
        rewrite_rules: Rc::new(std::convert::identity),
        marker: std::marker::PhantomData
    };
    let node = Ast::Enclosure(Enclosure {
        kind: EnclosureKind::Fragment,
        children,
    });
    let node = node.child_list_transformer(Rc::new(transfomer));
    match node {
        Ast::Enclosure(Enclosure{kind: _, children}) => {
            children
        }
        x => vec![x]
    }
}

pub fn node_passes<'a>(node: Ast<'a>) -> Ast<'a> {
    fn apply_rewrite_rules<'a>(tag: Tag<'a>) -> Tag<'a> {
        let mut children = tag.children;
        for RewriteRule{from, to} in tag.rewrite_rules {
            let from = from
                .into_enclosure_children(EnclosureKind::CurlyBrace);
            let to = to
                .into_enclosure_children(EnclosureKind::CurlyBrace);
            match (from, to) {
                (Some(from), Some(to)) => {
                    children = match_and_apply_rewrite_rule(
                        from,
                        to,
                        children,
                    );
                }
                _ => ()
            }
        }
        Tag {
            name: tag.name,
            parameters: tag.parameters,
            children,
            rewrite_rules: Vec::new(),
        }
    }
    fn process_tags<'a>(mut tag: Tag<'a>) -> Tag<'a> {
        let extract_tags = HashSet::<&str>::from_iter(vec![
            "note",
            "layout",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "li",
            "ul",
            "ol",
            "table",
            "tr",
            "td",
            "p",
            "u",
            "b",
        ]);
        fn unblock_children<'a>(children: Vec<Ast<'a>>) -> Vec<Ast<'a>> {
            children
                .into_iter()
                .flat_map(|child| {
                    match child {
                        Ast::Enclosure(
                            block
                        ) if block.kind == EnclosureKind::CurlyBrace => {
                            block.children
                        }
                        _ => vec![child]
                    }
                })
                .collect()
        }
        let name: &str = &(tag.name);
        if extract_tags.contains(name) {
            tag.children = unblock_children(tag.children);
        }
        // REWRITE SUBSCRIPT ONLY TAGS INTO VALID HTML
        if name == "note" {
            tag.name = Cow::Borrowed("div");
            tag.insert_parameter("macro=note");
        } else if name == "layout" {
            tag.name = Cow::Borrowed("div");
            tag.insert_parameter("macro=layout");
        }
        tag
    }
    let f = |node: Ast<'a>| -> Ast<'a> {
        match node {
            Ast::Tag(tag) => {
                let tag = apply_rewrite_rules(tag);
                let tag = process_tags(tag);
                Ast::Tag(tag)
            }
            Ast::Token(tk) => Ast::Content(tk),
            node @ Ast::Enclosure(_) => node,
            node @ Ast::Content(_) => node,
            node @ Ast::Ident(_) => node,
        }
    };
    node.transform(Rc::new(f))
}


pub fn passes<'a>(children: Vec<Ast<'a>>) -> Vec<Ast<'a>> {
    let children = children
        .into_iter()
        .map(node_passes)
        .collect();
    child_list_passes(children)
}

///////////////////////////////////////////////////////////////////////////////
// AST-TO-CODEGEN PASSES
///////////////////////////////////////////////////////////////////////////////

use crate::codegen::html;

pub fn node_to_html<'a>(node: Ast<'a>) -> html::Node<'a> {
    fn enclosure<'a>(
        start: &'a str,
        children: Vec<Ast<'a>>,
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
    fn map_children<'a>(children: Vec<Ast<'a>>) -> Vec<html::Node<'a>> {
        children.into_iter().map(node_to_html).collect::<Vec<_>>()
    }
    fn to_html_attributes<'a>(parameters: Vec<Ast<'a>>) -> HashMap<Text<'a>, Text<'a>> {
        parameters
            .into_iter()
            .filter_map(|node| -> Option<Text<'a>> {
                match node {
                    Ast::Content(txt) if !txt.trim().is_empty() => {
                        Some(Text(txt))
                    }
                    Ast::Token(txt) if !txt.trim().is_empty() => {
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
        Ast::Tag(node) => {
            html::Node::Element(html::Element {
                name: Text(node.name),
                attributes: node.parameters
                    .map(to_html_attributes)
                    .unwrap_or_default(),
                children: map_children(node.children),
            })
        },
        Ast::Enclosure(Enclosure {
            kind: EnclosureKind::CurlyBrace,
            children
        }) => {
            enclosure(
                "{",
                children,
                "}"
            )
        },
        Ast::Enclosure(Enclosure {
            kind: EnclosureKind::Parens,
            children
        }) => {
            enclosure(
                "(",
                children,
                ")"
            )
        },
        Ast::Enclosure(Enclosure {
            kind: EnclosureKind::Fragment,
            children
        }) => {
            html::Node::Fragment(map_children(children))
        },
        Ast::Enclosure(Enclosure {
            kind: EnclosureKind::SquareParen,
            children
        }) => {
            enclosure(
                "[",
                children,
                "]"
            )
        },
        Ast::Enclosure(Enclosure {
            kind: EnclosureKind::Error{open, close},
            children
        }) => {
            enclosure(
                open,
                children,
                close
            )
        },
        Ast::Ident(node) => {
            let node = Text::new("\\").append(Text(node));
            html::Node::Text(node)
        },
        Ast::Content(node) => {
            html::Node::Text(Text(node))
        },
        Ast::Token(node) => {
            html::Node::Text(Text(node))
        },
    }
}

///////////////////////////////////////////////////////////////////////////////
// AST TO CODEGEN
///////////////////////////////////////////////////////////////////////////////

pub fn to_html_pipeline<'a>(nodes: Vec<Ast<'a>>) -> Vec<crate::codegen::html::Node<'a>> {
    let result = passes(nodes);
    let result = result
        .into_iter()
        .map(crate::backend::math::latex_pass)
        .map(node_to_html)
        .collect::<Vec<_>>();
    result
}
