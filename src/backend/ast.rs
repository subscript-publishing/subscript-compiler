//! Backend AST data types & related.
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use crate::compiler::data::{
    Text,
    Enclosure,
    Atom,
    CurlyBrace,
    SquareParen,
    RewriteRule,
    EnclosureKind,
    INLINE_MATH_TAG,
};

///////////////////////////////////////////////////////////////////////////////
// AST - BLOCK
///////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone, PartialEq)]
pub struct Tag<'a> {
    pub name: Atom<'a>,
    pub parameters: Option<Vec<Ast<'a>>>,
    /// Each child node generally should be an `Enclosure` (with the `CurlyBrace` kind).
    /// Until perhaps the codegen.
    pub children: Vec<Ast<'a>>,
    pub rewrite_rules: Vec<RewriteRule<Ast<'a>>>,
}

impl<'a> Tag<'a> {
    /// Some tag with no parameters and just children.
    pub fn new(name: &'a str, children: Vec<Ast<'a>>) -> Self {
        Tag{
            name: Cow::Borrowed(name),
            parameters: None,
            children,
            rewrite_rules: Vec::new(),
        }
    }
    pub fn has_name(&self, name: &str) -> bool {
        return self.name == name 
    }
    pub fn insert_parameter(&mut self, value: &str) {
        let mut args = self.parameters.clone().unwrap_or(Vec::new());
        args.push(Ast::Content(Cow::Owned(
            value.to_owned()
        )));
        self.parameters = Some(args);
    }
    pub fn name(&self) -> &str {
        &self.name
    }
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
    pub fn new_fragment(nodes: Vec<Self>) -> Self {
        Ast::Enclosure(Enclosure {
            kind: EnclosureKind::Fragment,
            children: nodes,
        })
    }
    /// Unpacks an `Ast::Enclosure` with the `Fragment` kind or
    /// returns a singleton vec.
    pub fn into_fragment(self) -> Vec<Self> {
        match self {
            Ast::Enclosure(Enclosure{
                kind: EnclosureKind::Fragment,
                children
            }) => children,
            x => vec![x]
        }
    }
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
    pub fn get_enclosure_children(&self, kind: EnclosureKind) -> Option<&Vec<Ast<'a>>> {
        match self {
            Ast::Enclosure(x) if x.kind == kind => Some(x.children.as_ref()),
            _ => None,
        }
    }
    pub fn get_string(&'a self) -> Option<Cow<'a, str>> {
        match self {
            Ast::Content(cow) => Some(cow.clone()),
            _ => None,
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
    pub fn is_any_enclosure(&self) -> bool {
        match self {
            Ast::Enclosure(_) => true,
            _ => false,
        }
    }
    pub fn is_enclosure_of_kind(&self, kind: EnclosureKind) -> bool {
        match self {
            Ast::Enclosure(Enclosure { kind, .. }) => &kind == &kind,
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
    pub fn unpack_content_mut(&mut self) -> Option<&mut Atom<'a>> {
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
    pub fn is_whitespace(&self) -> bool {
        match self {
            Ast::Content(node) => node == &" ",
            Ast::Token(node) => node == &" ",
            _ => false
        }
    }
    pub fn to_string(self) -> String {
        fn pack<'a>(x: Cow<'a, str>) -> String {
            match x {
                Cow::Borrowed(x) => String::from(x),
                Cow::Owned(x) => x,
            }
        }
        fn ident<'a>(x: Cow<'a, str>) -> String {
            let mut txt = pack(x);
            txt.insert(0, '\\');
            txt
        }
        fn enclosure<'a>(
            start: Atom<'a>,
            content: String,
            end: Option<Atom<'a>>,
        ) -> String {
            let end = end
                .map(|x| x.to_string())
                .unwrap_or(String::new());
            format!("{}{}{}", start, content, end)
        }
        fn enclosure_str<'a>(
            start: &str,
            content: String,
            end: &str,
        ) -> String {
            format!("{}{}{}", start, content, end)
        }
        match self {
            Ast::Tag(tag) => {
                let name = pack(tag.name);
                let children = tag.children
                    .into_iter()
                    .map(Ast::to_string)
                    .collect::<Vec<_>>()
                    .join("");
                format!("\\{}{}", name, children)
            }
            Ast::Enclosure(block) => {
                let children = block.children
                    .into_iter()
                    .map(Ast::to_string)
                    .collect::<Vec<_>>()
                    .join("");
                match block.kind {
                    EnclosureKind::Fragment => {
                        children
                    }
                    EnclosureKind::CurlyBrace => {
                        enclosure_str("{", children, "}")
                    }
                    EnclosureKind::Parens => {
                        enclosure_str("(", children, ")")
                    }
                    EnclosureKind::SquareParen => {
                        enclosure_str("[", children, "]")
                    }
                    EnclosureKind::Error{open, close} => {
                        enclosure(open, children, close)
                    }
                }
            }
            Ast::Ident(x) => ident(x),
            Ast::Content(x) => pack(x),
            Ast::Token(x) => pack(x),
        }
    }
    pub fn unblock(self) -> Vec<Self> {
        match self {
            Ast::Enclosure(block) if block.kind == EnclosureKind::CurlyBrace => {
                block.children
            }
            x => vec![x]
        }
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
                let parameters = node.parameters.map(|param| {
                    let param = param
                        .into_iter()
                        .map(|x| {
                            x.child_list_transformer(f.clone())
                        })
                        .collect();
                    (f.parameters)(param)
                });
                let node = Tag {
                    name: node.name,
                    parameters,
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
// DEV
///////////////////////////////////////////////////////////////////////////////

pub(crate) fn dev() {
    let source = include_str!("../../source.txt");
    // let result = to_html_pipeline(source);
    // let result = crate::backend::parser::run_parser(source);
    // let result = crate::frontend::passes(result);
    // for node in result {
    //     println!("{:#?}", node);
    // }
}


