use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
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
// PASSES
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
        }
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
                    // let from = from
                    //     .into
                    // children = match_and_apply_rewrite_rule(
                    //     from,
                    //     to,
                    //     children,
                    // );
                }
                unimplemented!()
            }
            Ast::Enclosure(block) => {
                unimplemented!()
            }
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
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn dev() {
    let source = include_str!("../../source.txt");
    let result = crate::frontend::parser::run_parser(source);
    // let result = passes(result);
    for node in result {
        println!("{:#?}", node);
    }
}


