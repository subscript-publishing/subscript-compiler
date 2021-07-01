//! AST HTML canonicalization. 
//! 
//! This should really be implemented using the HTML AST; but it’s easier using
//! the frontend AST instead, since we don’t need to replicate AST utilities.

use std::iter::FromIterator;
use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::Cow;
use std::convert::TryFrom;
use either::Either;
use crate::frontend::data::*;
use crate::frontend::ast::*;

///////////////////////////////////////////////////////////////////////////////
// TABLE OF CONTENTS
///////////////////////////////////////////////////////////////////////////////

fn generate_toc_heading_id_from_child_nodes<'a>(children: &Vec<Node<'a>>) -> String {
    use pct_str::PctStr;
    let contents = generate_toc_heading_title_from_child_nodes(children);
    let pct_str = PctStr::new(&contents).unwrap();
    pct_str.as_str().to_owned()
}

fn generate_toc_heading_title_from_child_nodes<'a>(children: &Vec<Node<'a>>) -> String {
    children.iter()
        .map(Node::to_string)
        .collect::<Vec<_>>()
        .join("")
}

fn get_headings<'a>(node: &Node<'a>) -> Vec<(Vec<String>, String, String, String)> {
    let headings = Rc::new(RefCell::new(Vec::new()));
    let f = {
        let headings = headings.clone();
        move |env: NodeEnvironment<'a>, node: Node<'a>| {
            match &node {
                Node::Tag(tag) if tag.is_heading_node() => {
                    let id = generate_toc_heading_id_from_child_nodes(&tag.children);
                    let parents = env.parents
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .clone();
                    let name = tag.name.data.to_string();
                    let text = tag.children
                        .iter()
                        .flat_map(|x| x.clone().unblock())
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("");
                    headings.borrow_mut().push((parents, name, text, id));
                }
                _ => ()
            }
            node
        }
    };
    let _ = node.clone().transform(
        NodeEnvironment::default(),
        Rc::new(f),
    );
    let info = headings
        .clone()
        .borrow()
        .iter()
        .map(|x| {
            x.clone()
        })
        .collect::<Vec<_>>();
    info
}

pub(crate) fn generate_table_of_contents_tree<'a>(input: &Node<'a>) -> Node<'a> {
    let children = get_headings(input)
        .into_iter()
        .map(|(parents, ty, contents, id)| {
            let mut a = Tag::new(
                Ann::unannotated("a"),
                vec![Node::unannotated_string(contents)]
            );
            a.insert_unannotated_parameter(
                &format!("href=#{}", id)
            );
            let a = Node::Tag(a);
            let mut li = Tag::new(
                Ann::unannotated("li"),
                vec![a]
            );
            li.insert_unannotated_parameter(
                &format!("type={}", ty)
            );
            let li = Node::Tag(li);
            li
        })
        .collect::<Vec<_>>();
    let mut tag = Tag::new(
        Ann::unannotated("ul"),
        children
    );
    tag.insert_unannotated_parameter("id=toc");
    let node = Node::Tag(tag);
    node
}

pub fn annotate_heading_nodes<'a>(input: Node<'a>) -> Node<'a> {
    let f = |env: NodeEnvironment, node: Node<'a>| -> Node<'a> {
        match node {
            Node::Tag(mut tag) if tag.is_heading_node() => {
                let id = generate_toc_heading_id_from_child_nodes(&tag.children);
                tag.insert_unannotated_parameter(
                    &format!("id={}", id)
                );
                Node::Tag(tag)
            }
            x => x,
        }
    };
    input.transform(NodeEnvironment::default(), Rc::new(f))
}


///////////////////////////////////////////////////////////////////////////////
// WHERE TAG PROCESSING
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


///////////////////////////////////////////////////////////////////////////////
// AST-TO-AST PASSES
///////////////////////////////////////////////////////////////////////////////

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
        }
        else if name == "img" {
            let value = tag
                .get_parameter("width")
                .map(|x| x.data)
                .and_then(|x| {
                    let x: &str = &x;
                    x.parse::<f32>().ok()
                });
            if let Some(width) = value {
                tag.insert_unannotated_parameter(&format!(
                    "style='width:{};'",
                    width
                ));
            }
        }
        else if name == "layout" {
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

