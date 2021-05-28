use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use crate::frontend::data::{
    Text,
    Enclosure,
    Atom,
    CurlyBrace,
    SquareParen,
    RewriteRule,
    EnclosureKind,
};

///////////////////////////////////////////////////////////////////////////////
// AST - BLOCK
///////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone, PartialEq)]
pub struct Tag<'a> {
    pub name: Atom<'a>,
    pub parameters: Option<Vec<Ast<'a>>>,
    pub children: Vec<Ast<'a>>,
    pub rewrite_rules: Vec<RewriteRule<Ast<'a>>>,
}

// #[derive(Debug, Clone)]
// pub struct Enclosure {
//     kind: 
// }

///////////////////////////////////////////////////////////////////////////////
// ROOT AST
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq)]
pub enum Ast<'a> {
    Tag(Tag<'a>),
    /// Some other enclosure, perhaps as a result of an error, such as in the
    /// case of curly braces or square parentheses thar Isn’t affiliated with a
    /// block node. Parentheses are fine though, since it's used in math mode.
    Enclosure(Enclosure<'a, Ast<'a>>),
    /// Some identifier not followed by a block. 
    Ident(Atom<'a>),
    Content(Atom<'a>),
    Token(Atom<'a>),
}

impl<'a> Ast<'a> {
    /// Bottom up transformation.
    pub fn transform<F: Fn(Ast<'a>) -> Ast<'a>>(self, f: Rc<F>) -> Self {
        match self {
            Ast::Tag(node) => {
                let children = node.children
                    .into_iter()
                    .map(|x| x.transform(f.clone()))
                    .collect();
                let rewrite_rules = node.rewrite_rules
                    .into_iter()
                    .map(|rule| -> RewriteRule<Ast<'a>> {
                        RewriteRule {
                            from: rule.from.transform(f.clone()),
                            to: rule.to.transform(f.clone()),
                        }
                    })
                    .collect();
                let node = Tag {
                    name: node.name,
                    parameters: node.parameters,
                    children,
                    rewrite_rules,
                };
                f(Ast::Tag(node))
            }
            Ast::Enclosure(node) => {
                let children = node.children
                    .into_iter()
                    .map(|x| x.transform(f.clone()))
                    .collect();
                let node = Ast::Enclosure(Enclosure{
                    kind: node.kind,
                    children,
                });
                f(node)
            }
            node @ Ast::Ident(_) => {
                f(node)
            }
            node @ Ast::Content(_) => {
                f(node)
            }
            node @ Ast::Token(_) => {
                f(node)
            }
        }
    }
    pub fn is_named_block(&self, name: &str) -> bool {
        self.unpack_tag()
            .map(|x| *x.name == *name)
            .unwrap_or(false)
    }
    pub fn is_tag(&self) -> bool {
        match self {
            Ast::Tag(_) => true,
            _ => false,
        }
    }
    pub fn is_ident(&self) -> bool {
        match self {
            Ast::Ident(_) => true,
            _ => false,
        }
    }
    pub fn is_enclosure(&self) -> bool {
        match self {
            Ast::Enclosure(_) => true,
            _ => false,
        }
    }
    pub fn is_content(&self) -> bool {
        match self {
            Ast::Content(_) => true,
            _ => false,
        }
    }
    pub fn is_token(&self) -> bool {
        match self {
            Ast::Token(_) => true,
            _ => false,
        }
    }
    pub fn unpack_tag(&self) -> Option<&Tag<'a>> {
        match self {
            Ast::Tag(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_tag_mut(&mut self) -> Option<&mut Tag<'a>> {
        match self {
            Ast::Tag(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_ident(&self) -> Option<&Atom<'a>> {
        match self {
            Ast::Ident(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_enclosure(&self) -> Option<&Enclosure<'a, Ast<'a>>> {
        match self {
            Ast::Enclosure(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_content(&self) -> Option<&Atom<'a>> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_token(&self) -> Option<&Atom<'a>> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_tag(self) -> Option<Tag<'a>> {
        match self {
            Ast::Tag(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_enclosure(self) -> Option<Enclosure<'a, Ast<'a>>> {
        match self {
            Ast::Enclosure(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_enclosure_children(self, kind: EnclosureKind) -> Option<Vec<Ast<'a>>> {
        match self {
            Ast::Enclosure(x) if x.kind == kind => Some(x.children),
            _ => None,
        }
    }
    pub fn into_ident(self) -> Option<Atom<'a>> {
        match self {
            Ast::Ident(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_parens(self) -> Option<Enclosure<'a, Ast<'a>>> {
        match self {
            Ast::Enclosure(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_content(self) -> Option<Atom<'a>> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_token(self) -> Option<Atom<'a>> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None,
        }
    }

    pub fn unsafe_unwrap_ident(&self) -> &Atom<'a> {
        self.unpack_ident().unwrap()
    }
}


///////////////////////////////////////////////////////////////////////////////
// ROOT AST HELPERS
///////////////////////////////////////////////////////////////////////////////

pub struct ChildListTransformer<'a, F, G, H>
where
    F: Fn(Vec<Ast<'a>>) -> Vec<Ast<'a>>,
    G: Fn(Vec<Ast<'a>>) -> Vec<Ast<'a>>,
    H: Fn(Vec<RewriteRule<Ast<'a>>>) -> Vec<RewriteRule<Ast<'a>>>,
{
    pub parameters: Rc<F>,
    /// Either a tag or some enclosure.
    pub block: Rc<G>,
    pub rewrite_rules: Rc<H>,
    pub marker: std::marker::PhantomData<Ast<'a>>,
}

impl<'a> Ast<'a> {
    pub fn child_list_transformer<F, G, H>(
        self,
        f: Rc<ChildListTransformer<'a, F, G, H>>
    ) -> Self where
    F: Fn(Vec<Ast<'a>>) -> Vec<Ast<'a>>,
    G: Fn(Vec<Ast<'a>>) -> Vec<Ast<'a>>,
    H: Fn(Vec<RewriteRule<Ast<'a>>>) -> Vec<RewriteRule<Ast<'a>>>,
    {
        match self {
            Ast::Tag(node) => {
                let children = node.children
                    .into_iter()
                    .map(|x| -> Ast {
                        x.child_list_transformer(f.clone())
                    })
                    .collect::<Vec<_>>();
                let children = (f.block)(children);
                let rewrite_rules = node.rewrite_rules
                    .into_iter()
                    .map(|rule| -> RewriteRule<Ast<'a>> {
                        RewriteRule {
                            from: rule.from.child_list_transformer(f.clone()),
                            to: rule.to.child_list_transformer(f.clone()),
                        }
                    })
                    .collect();
                let rewrite_rules = (f.rewrite_rules)(rewrite_rules);
                let node = Tag {
                    name: node.name,
                    parameters: node.parameters.map(|param| {
                        let param = param
                            .into_iter()
                            .map(|x| {
                                x.child_list_transformer(f.clone())
                            })
                            .collect();
                        (f.parameters)(param)
                    }),
                    children,
                    rewrite_rules,
                };
                Ast::Tag(node)
            }
            Ast::Enclosure(node) => {
                let children = node.children
                    .into_iter()
                    .map(|x| x.child_list_transformer(f.clone()))
                    .collect();
                let children = (f.block)(children);
                let node = Enclosure{
                    kind: node.kind,
                    children,
                };
                Ast::Enclosure(node)
            }
            node @ Ast::Ident(_) => {node}
            node @ Ast::Content(_) => {node}
            node @ Ast::Token(_) => {node}
        }
    }
}

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
    let parameters = |xs: Vec<Ast<'a>>| -> Vec<Ast<'a>> {
        xs
    };
    let block = |xs: Vec<Ast<'a>>| -> Vec<Ast<'a>> {
        xs
    };
    let rewrite_rules = |xs: Vec<RewriteRule<Ast<'a>>>| -> Vec<RewriteRule<Ast<'a>>> {
        xs
    };
    let transfomer = ChildListTransformer {
        parameters: Rc::new(parameters),
        block: Rc::new(block),
        rewrite_rules: Rc::new(rewrite_rules),
        marker: std::marker::PhantomData
    };
    let node = Ast::Enclosure(Enclosure {
        kind: EnclosureKind::Module,
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
    let f = |node: Ast<'a>| -> Ast<'a> {
        match node {
            Ast::Tag(block) => {
                let mut children = block.children;
                for RewriteRule{from, to} in block.rewrite_rules {
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
                Ast::Tag(Tag {
                    name: block.name,
                    parameters: block.parameters,
                    children,
                    rewrite_rules: Vec::new(),
                })
            }
            node @ Ast::Enclosure(_) => node,
            node @ Ast::Content(_) => node,
            node @ Ast::Token(_) => node,
            node @ Ast::Ident(_) => node,
        }
    };
    node.transform(Rc::new(f))
}


pub fn passes<'a>(children: Vec<Ast<'a>>) -> Vec<Ast<'a>> {
    children
        .into_iter()
        .map(node_passes)
        .collect()
}

///////////////////////////////////////////////////////////////////////////////
// AST-TO-CODEGEN PASSES
///////////////////////////////////////////////////////////////////////////////

use crate::codegen::html;

pub fn to_html<'a>(node: Ast<'a>) -> html::Node<'a> {
    fn enclosure<'a>(
        start: &'a str,
        children: Vec<Ast<'a>>,
        end: &'a str,
    ) -> html::Node<'a> {
        html::Node::Fragment(
            vec![
                vec![html::Node::new_text("{")],
                children.into_iter().map(to_html).collect::<Vec<_>>(),
                vec![html::Node::new_text("}")],
            ].concat()
        )
    }
    fn map_children<'a>(children: Vec<Ast<'a>>) -> Vec<html::Node<'a>> {
        children.into_iter().map(to_html).collect::<Vec<_>>()
    }
    fn to_html_attributes<'a>(children: Vec<Ast<'a>>) -> HashMap<Text<'a>, Text<'a>> {
        children
            .into_iter()
            .filter_map(|node| -> Option<Text<'a>> {
                match node {
                    Ast::Content(txt) => Some(Text(txt)),
                    Ast::Token(txt) => Some(Text(txt)),
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
            kind: EnclosureKind::Module,
            children
        }) => {
            html::Node::Fragment(map_children(children))
        },
        Ast::Enclosure(Enclosure {
            kind: EnclosureKind::SquareParen,
            children
        }) => {
            enclosure(
                "(",
                children,
                ")"
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
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn dev() {
    let source = include_str!("../../source.txt");
    let result = crate::frontend::parser::run_parser(source);
    let result = passes(result);
    let result = result
        .into_iter()
        .map(to_html)
        .collect::<Vec<_>>();
    for node in result {
        println!("{:#?}", node);
    }
}


