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
use crate::frontend::{Ast, Tag};

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
fn to_valid_latex_math<'a>(node: Ast<'a>) -> Ast<'a> {
    // HELPERS
    fn init_env<'a>(env_name: &'a str, tag_name: Cow<'a, str>, children: Vec<Ast<'a>>) -> Ast<'a> {
        Ast::new_fragment(vec![
            Ast::Tag(Tag::new("begin", vec![
                Ast::Enclosure(Enclosure::new_curly_brace(vec![
                    Ast::Content(Cow::Borrowed(env_name))
                ]))
            ])),
            Ast::new_fragment(children),
            Ast::Tag(Tag::new("end", vec![
                Ast::Enclosure(Enclosure::new_curly_brace(vec![
                    Ast::Content(Cow::Borrowed(env_name))
                ]))
            ])),
        ])
    }
    // FUNCTION
    fn f<'a>(x: Ast<'a>) -> Ast<'a> {
        match x {
            Ast::Tag(tag) if LATEX_ENV_NAMES.contains(tag.name()) => {
                let env_name = *LATEX_ENV_NAMES.get(tag.name()).unwrap();
                init_env(
                    env_name,
                    tag.name.clone(),
                    tag.children
                )
            }
            Ast::Tag(mut tag) => {
                tag.children = tag.children
                    .into_iter()
                    // .flat_map(Ast::unblock)
                    .collect();
                Ast::Tag(tag)
            }
            x => x,
        }
    }
    // GO! (BOTTOM UP)
    node.transform(Rc::new(f))
}


/// Entrypoint.
pub fn latex_pass<'a>(node: Ast<'a>) -> Ast<'a> {
    match node {
        Ast::Tag(tag) if tag.has_name("equation") => {
            let node = tag.children
                .into_iter()
                .flat_map(Ast::unblock)
                .map(to_valid_latex_math)
                .map(Ast::to_string)
                .collect::<Vec<_>>()
                .join("");
            let start = "\\begin{equation}\\begin{split}";
            let end = "\\end{split}\\end{equation}";
            Ast::Content(Cow::Owned(format!(
                "\\[{}{}{}\\]",
                start,
                node,
                end,
            )))
        }
        Ast::Tag(tag) if tag.has_name(INLINE_MATH_TAG) => {
            let node = tag.children
                .into_iter()
                // .flat_map(Ast::unblock)
                .map(to_valid_latex_math)
                .map(Ast::to_string)
                .collect::<Vec<_>>()
                .join("");
            Ast::Content(Cow::Owned(format!(
                "\\({}\\)",
                node,
            )))
        }
        Ast::Tag(mut tag) => {
            tag.children = tag.children
                .into_iter()
                .map(latex_pass)
                .collect();
            Ast::Tag(tag)
        }
        Ast::Enclosure(mut block) => {
            block.children = block.children
                .into_iter()
                .map(latex_pass)
                .collect();
            Ast::Enclosure(block)
        }
        node @ Ast::Ident(_) => node,
        node @ Ast::Content(_) => node,
        node @ Ast::Token(_) => node,
    }
}

