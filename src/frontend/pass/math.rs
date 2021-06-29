//! Compile math mode to the given target.
//!
//! Eventually the given options will be
//! * LaTeX math (for some external compiler such as MathJax when using the HTML target). 
//! * Native typesetter. 
use lazy_static::lazy_static;
use std::iter::FromIterator;
use std::collections::HashSet;
use std::rc::Rc;
use std::borrow::Cow;
use crate::frontend::data::{
    Atom,
    Text,
    Enclosure,
    EnclosureKind,
    INLINE_MATH_TAG,
};
use crate::frontend::ast::{Ann, Node, NodeEnvironment, Tag};

pub static LATEX_ENVIRONMENT_NAME_LIST: &'static [&'static str] = &[
    "equation",
    "split",
];

lazy_static! {
    pub static ref LATEX_ENV_NAMES: HashSet<&'static str> = {
        HashSet::from_iter(
            LATEX_ENVIRONMENT_NAME_LIST.to_vec()
        )
    };
}


/// Converts math nodes to a valid latex code within the AST data model.
fn to_valid_latex_math<'a>(node: Node<'a>) -> Node<'a> {
    // HELPERS
    fn init_env<'a>(env_name: &'a str, children: Vec<Node<'a>>) -> Node<'a> {
        Node::new_fragment(vec![
            Node::unannotated_tag_(
                "begin",
                Node::Enclosure(Ann::unannotated(
                    Enclosure::new_curly_brace_(Node::unannotated_str(env_name))
                ))
            ),
            Node::new_fragment(children),
            Node::unannotated_tag_(
                "end",
                Node::Enclosure(Ann::unannotated(
                    Enclosure::new_curly_brace_(Node::unannotated_str(env_name))
                ))
            ),
        ])
    }
    // FUNCTION
    fn f<'a>(env: NodeEnvironment, x: Node<'a>) -> Node<'a> {
        match x {
            Node::Tag(tag) if LATEX_ENV_NAMES.contains(tag.name()) => {
                let env_name = *LATEX_ENV_NAMES.get(tag.name()).unwrap();
                init_env(
                    env_name,
                    tag.children
                )
            }
            Node::Tag(mut tag) => {
                tag.children = tag.children
                    .into_iter()
                    // .flat_map(Node::unblock)
                    .collect();
                Node::Tag(tag)
            }
            x => x,
        }
    }
    // GO! (BOTTOM UP)
    node.transform(NodeEnvironment::default(), Rc::new(f))
}


/// Entrypoint.
pub fn latex_pass<'a>(node: Node<'a>) -> Node<'a> {
    match node {
        Node::Tag(tag) if tag.has_name("equation") => {
            let node = tag.children
                .into_iter()
                .flat_map(Node::unblock)
                .map(to_valid_latex_math)
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("");
            let start = "\\begin{equation}\\begin{split}";
            let end = "\\end{split}\\end{equation}";
            Node::String(Ann::unannotated(Cow::Owned(format!(
                "\\[{}{}{}\\]",
                start,
                node,
                end,
            ))))
        }
        Node::Tag(tag) if tag.has_name(INLINE_MATH_TAG) => {
            let node = tag.children
                .into_iter()
                .flat_map(Node::unblock)
                .map(to_valid_latex_math)
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("");
            Node::String(Ann::unannotated(Cow::Owned(format!(
                "\\({}\\)",
                node,
            ))))
        }
        Node::Tag(mut tag) => {
            tag.children = tag.children
                .into_iter()
                .map(latex_pass)
                .collect();
            Node::Tag(tag)
        }
        Node::Enclosure(mut block) => {
            block.data.children = block.data.children
                .into_iter()
                .map(latex_pass)
                .collect();
            Node::Enclosure(block)
        }
        node @ Node::Ident(_) => node,
        node @ Node::String(_) => node,
        node @ Node::InvalidToken(_) => node,
    }
}

