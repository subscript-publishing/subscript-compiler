//! AST HTML canonicalization. 
//! 
//! This should really be implemented using the HTML AST; but it’s easier using
//! the frontend AST instead, since we don’t need to replicate AST utilities.

use std::iter::FromIterator;
use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::borrow::Cow;
use crate::frontend::data::*;
use crate::frontend::ast::*;


///////////////////////////////////////////////////////////////////////////////
// AST-TO-AST PASSES
///////////////////////////////////////////////////////////////////////////////

/// This is where we expand the patterns defined in `\!where` tags.
fn match_and_apply_rewrite_rule<'a>(
    pattern: Vec<Node<'a>>,
    target: Vec<Node<'a>>,
    children: Vec<Node<'a>>,
) -> Vec<Node<'a>> {
    let mut left: Vec<Node<'a>> = Vec::<Node>::new();
    let mut current = children;
    while current.len() > 0 && current.len() >= pattern.len() {
        let matches = current
            .iter()
            .zip(pattern.iter())
            .all(|(x, y)| x.syntactically_equal(y));
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

/// All compiler passes for same scope children.
fn child_list_passes<'a>(children: Vec<Node<'a>>) -> Vec<Node<'a>> {
    // APPLY AFTER REMOVING ALL TOKENS
    fn merge_text_content<'a>(xs: Vec<Node<'a>>) -> Vec<Node<'a>> {
        let mut results = Vec::new();
        for current in xs.into_iter() {
            let left = results
                .last_mut()
                .and_then(Node::unwrap_string_mut);
            if let Some(left) = left {
                if let Some(txt) = current.unwrap_string() {
                    *left = Ann::unannotated(left.data.to_owned() + txt.data.to_owned());
                    continue;
                }
            }
            results.push(current);
        }
        results
    }
    fn block_passes<'a>(xs: Vec<Node<'a>>) -> Vec<Node<'a>> {
        /// Put all 'block passes' here
        merge_text_content(xs)
    }
    let node = Node::new_fragment(children);
    let node = node.transform_children(Rc::new(block_passes));
    node.into_fragment()
}

/// All node to node passes.
fn node_passes<'a>(node: Node<'a>) -> Node<'a> {
    fn apply_rewrite_rules<'a>(tag: Tag<'a>) -> Tag<'a> {
        let mut children = tag.children;
        for RewriteRule{from, to} in tag.rewrite_rules {
            let from = from.unwrap_curly_brace();
            let to = to.unwrap_curly_brace();
            match (from, to) {
                (Some(from), Some(to)) => {
                    children = match_and_apply_rewrite_rule(
                        from.clone(),
                        to.clone(),
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
    fn process_tags<'a>(env: NodeEnvironment, mut tag: Tag<'a>) -> Tag<'a> {
        let name: &str = &(tag.name.data);
        // DON'T DO THIS IN A MATH ENV
        if env.is_default_env() {
            // Apply this after any multi-argument specific tag processing.
            // Because e.g. `\h1{hello }{world}` will become `<h1>hello world</h1>`.
            // Really only a problem if we happen to be converting to LaTeX.
            tag.children = tag.children
                .into_iter()
                .flat_map(Node::unblock)
                .collect();
        }
        // REWRITE SUBSCRIPT TAGS INTO VALID HTML
        if name == "note" {
            tag.name = Ann::unannotated(Cow::Borrowed("div"));
            tag.insert_unannotated_parameter("macro=note");
        } else if name == "layout" {
            tag.name = Ann::unannotated(Cow::Borrowed("div"));
            tag.insert_unannotated_parameter("macro=layout");
        }
        tag
    }
    let f = |env: NodeEnvironment, node: Node<'a>| -> Node<'a> {
        match node {
            Node::Tag(tag) => {
                let tag = apply_rewrite_rules(tag);
                let tag = process_tags(env, tag);
                Node::Tag(tag)
            }
            node @ Node::Enclosure(_) => node,
            node @ Node::String(_) => node,
            node @ Node::Ident(_) => node,
            node @ Node::InvalidToken(_) => node,
        }
    };
    node.transform(NodeEnvironment::default(), Rc::new(f))
}


///////////////////////////////////////////////////////////////////////////////
// AST TO CODEGEN
///////////////////////////////////////////////////////////////////////////////

/// Internal
pub fn html_canonicalization<'a>(nodes: Vec<Node<'a>>) -> Vec<Node<'a>> {
    fn passes<'a>(children: Vec<Node<'a>>) -> Vec<Node<'a>> {
        let children = children
            .into_iter()
            .map(node_passes)
            .collect();
        child_list_passes(children)
    }
    let result = passes(nodes);
    let result = result
        .into_iter()
        .map(crate::frontend::pass::math::latex_pass)
        .collect::<Vec<_>>();
    result
}

