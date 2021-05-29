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
                        enclosure("{", children, "}")
                    }
                    EnclosureKind::Parens => {
                        enclosure("(", children, ")")
                    }
                    EnclosureKind::SquareParen => {
                        enclosure("[", children, "]")
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
            tag.insert_parameter("note");
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
        .map(crate::frontend::math::latex_pass)
        .map(node_to_html)
        .collect::<Vec<_>>();
    result
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn dev() {
    let source = include_str!("../../source.txt");
    // let result = to_html_pipeline(source);
    let result = crate::frontend::parser::run_parser(source);
    let result = passes(result);
    for node in result {
        println!("{:#?}", node);
    }
}


