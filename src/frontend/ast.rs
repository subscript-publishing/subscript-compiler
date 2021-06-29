//! Frontend AST data types & related.
use std::cell::RefCell;
use std::rc::Rc;
use std::borrow::{Borrow, Cow};
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use serde::{Serialize, Deserialize};
use crate::frontend::data::*;

///////////////////////////////////////////////////////////////////////////////
// INDEXING DATA TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize)]
pub struct CharIndex {
    pub byte_index: usize,
    pub char_index: usize,
}

impl CharIndex {
    pub fn zero() -> Self {
        CharIndex{
            byte_index: 0,
            char_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize)]
pub struct CharRange {
    pub start: CharIndex,
    pub end: CharIndex,
}

impl CharRange {
    pub fn join(start: Option<CharIndex>, end: Option<CharIndex>) -> Option<Self> {
        if let Some(start) = start {
            if let Some(end) = end {
                return Some(CharRange{start, end})
            }
        }
        None
    }
    pub fn new(start: CharIndex, end: CharIndex) -> Self {
        CharRange{start, end}
    }
    pub fn byte_index_range<'a>(&self, source: &'a str) -> Option<(usize, usize)> {
        fn find_utf8_end(s: &str, i: usize) -> Option<usize> {
            s.char_indices().nth(i).map(|(_, x)| x.len_utf8())
        }
        let start_byte = self.start.byte_index;
        let end_byte = self.end.byte_index;
        let real_end_byte = source
            .get(start_byte..=end_byte)
            .map(|_| end_byte)
            .or_else(|| {
                let corrected_end = find_utf8_end(source, end_byte)?;
                source
                    .get(start_byte..=corrected_end)
                    .map(|_| corrected_end)
            });
        real_end_byte.map(|l| (start_byte, l))
    }
    pub fn substrng<'a>(&self, source: &'a str) -> Option<&'a str> {
        if let Some((start, end)) = self.byte_index_range(source) {
            let sub_str = source.get(start..end).unwrap();
            Some(sub_str)
        } else {
            None
        }
    }
    pub fn into_annotated_tree<T>(self, data: T) -> Ann<T> {
        Ann {
            range: Some(self),
            data,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ann<T> {
    range: Option<CharRange>,
    pub data: T,
}

impl<T> Ann<T> {
    pub fn unannotated(data: T) -> Self {
        let range = None;
        Ann {range, data}
    }
    pub fn new(range: CharRange, data: T) -> Self {
        Ann {range: Some(range), data}
    }
    pub fn join(range: Option<CharRange>, data: T) -> Self {
        Ann {range, data}
    }
    pub fn range(&self) -> Option<CharRange> {
        self.range
    }
    pub fn start(&self) -> Option<CharIndex> {
        if let Some(range) = self.range {
            return Some(range.start)
        }
        None
    }
    pub fn end(&self) -> Option<CharIndex> {
        if let Some(range) = self.range {
            return Some(range.end)
        }
        None
    }
    pub fn map<U>(self, f: impl Fn(T) -> U) -> Ann<U> {
        Ann {
            range: self.range,
            data: f(self.data),
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// FRONTEND
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Tag<'a> {
    pub name: Ann<Atom<'a>>,
    pub parameters: Option<Vec<Node<'a>>>,
    /// Each child node generally should be an `Enclosure` (with the `CurlyBrace` kind).
    /// Until perhaps the codegen.
    pub children: Vec<Node<'a>>,
    pub rewrite_rules: Vec<RewriteRule<Node<'a>>>,
}

impl<'a> Tag<'a> {
    /// Some tag with no parameters and just children.
    pub fn new(name: Ann<&'a str>, children: Vec<Node<'a>>) -> Self {
        Tag{
            name: Ann {
                range: name.range,
                data: Cow::Borrowed(name.data)
            },
            parameters: None,
            children,
            rewrite_rules: Vec::new(),
        }
    }
    pub fn has_name(&self, name: &str) -> bool {
        return self.name() == name
    }
    pub fn insert_parameter(&mut self, value: Ann<&str>) {
        let mut args = self.parameters.clone().unwrap_or(Vec::new());
        args.push(Node::String(Ann::join(
            value.range,
            Cow::Owned(value.data.to_owned()),
        )));
        self.parameters = Some(args);
    }
    pub fn insert_unannotated_parameter(&mut self, value: &str) {
        let mut args = self.parameters.clone().unwrap_or(Vec::new());
        args.push(Node::String(Ann::unannotated(
            Cow::Owned(value.to_owned())
        )));
        self.parameters = Some(args);
    }
    // /// Short for `Tag::insert_unannotated_parameter`
    // pub fn insert_attr(&mut self, value: &str) {
    //     self.insert_unannotated_parameter(value)
    // }
    pub fn name(&self) -> &str {
        &self.name.data
    }
    pub fn to_string(&self) -> String {
        Node::Tag(self.clone()).to_string()
    }
    pub fn is_heading_node(&self) -> bool {
        HEADING_TAG_NAMES.contains(self.name())
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeEnvironment<'a> {
    pub parents: Vec<Atom<'a>>,
}

impl<'a> NodeEnvironment<'a> {
    pub fn push_parent(&mut self, name: Atom<'a>) {
        self.parents.push(name)
    }
    pub fn is_math_env(&self) -> bool {
        self.parents
            .iter()
            .any(|x| {
                let option1 = x == INLINE_MATH_TAG;
                let option2 = BLOCK_MATH_TAGS.iter().any(|y| {
                    x == y
                });
                option1 || option2
            })
    }
    pub fn is_default_env(&self) -> bool {
        !self.is_math_env()
    }
}


///////////////////////////////////////////////////////////////////////////////
// AST
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum Node<'a> {
    /// The parser doesn’t emit AST `Tag` nodes. This is done in a later
    /// processing phase.
    Tag(Tag<'a>),
    /// Some identifier that may or may not be followed by square parentheses
    /// and/or a curly brace enclosure. E.g. `\name`.
    Ident(Ann<Atom<'a>>),
    /// An enclosure can be a multitude of things:
    /// * Some syntactic enclosure: 
    ///     * Curly braces
    ///     * Parentheses
    ///     * Square parentheses
    /// * Some error with it’s invalid start & end token (i.e. a opening `[` and closing `}`)
    /// * Additionally, after parsing, an enclosure can also be a fragment (i.e. a list of AST nodes)
    Enclosure(Ann<Enclosure<'a, Node<'a>>>),
    /// Some string of arbitrary characters or a single special token.
    String(Ann<Atom<'a>>),
    /// Some unbalanced token that isn’t associated with an enclosure. 
    /// In Subscript, enclosure symbols must be balanced. If the author
    /// must use such in their publications, then use the tag version. 
    InvalidToken(Ann<Atom<'a>>),
}


impl<'a> Node<'a> {
    /// Some tag with no parameters and just children.
    pub fn new_tag(name: Ann<&'a str>, children: Vec<Node<'a>>) -> Self {
        Node::Tag(Tag::new(name, children))
    }
    pub fn new_ident(str: Ann<&'a str>) -> Self {
        Node::Ident(str.map(|x| Cow::Borrowed(x)))
    }
    pub fn new_enclosure(
        range: CharRange,
        kind: EnclosureKind<'a>,
        children: Vec<Node<'a>>,
    ) -> Self {
        Node::Enclosure(Ann::new(range, Enclosure {kind, children}))
    }
    pub fn new_string(str: Ann<&'a str>) -> Self {
        Node::Ident(str.map(|x| Cow::Borrowed(x)))
    }
    pub fn unannotated_tag(name: &'a str, children: Vec<Node<'a>>) -> Self {
        Node::Tag(Tag::new(
            Ann::unannotated(name),
            children
        ))
    }
    pub fn unannotated_tag_(name: &'a str, child: Node<'a>) -> Self {
        Node::Tag(Tag::new(
            Ann::unannotated(name),
            vec![child]
        ))
    }
    pub fn unannotated_ident(str: &'a str) -> Self {
        Node::Ident(Ann::unannotated(Cow::Borrowed(str)))
    }
    pub fn unannotated_enclosure(
        kind: EnclosureKind<'a>,
        children: Vec<Node<'a>>,
    ) -> Self {
        Node::Enclosure(Ann::unannotated(Enclosure {kind, children}))
    }
    pub fn unannotated_str(str: &'a str) -> Self {
        Node::String(Ann::unannotated(Cow::Borrowed(str)))
    }
    pub fn unannotated_string(str: String) -> Self {
        Node::String(Ann::unannotated(Cow::Owned(str)))
    }

    pub fn new_fragment(nodes: Vec<Self>) -> Self {
        let data = Enclosure {
            kind: EnclosureKind::Fragment,
            children: nodes,
        };
        Node::Enclosure(Ann::unannotated(data))
    }
    pub fn is_whitespace(&self) -> bool {
        match self {
            Node::String(txt) => {
                let x: &str = &txt.data;
                x.trim().is_empty()
            },
            _ => false
        }
    }
    pub fn is_tag(&self) -> bool {
        match self {
            Node::Tag(_) => true,
            _ => false,
        }
    }
    pub fn is_ident(&self) -> bool {
        match self {
            Node::Ident(_) => true,
            _ => false,
        }
    }
    pub fn is_enclosure(&self) -> bool {
        match self {
            Node::Enclosure(_) => true,
            _ => false,
        }
    }
    pub fn is_string(&self) -> bool {
        match self {
            Node::String(_) => true,
            _ => false,
        }
    }
    pub fn is_any_enclosure(&self) -> bool {
        match self {
            Node::Enclosure(_) => true,
            _ => false,
        }
    }
    pub fn is_enclosure_of_kind(&self, k: EnclosureKind) -> bool {
        match self {
            Node::Enclosure(Ann{data: Enclosure{kind, ..}, ..}) => {
                kind == &k
            },
            _ => false,
        }
    }
    pub fn is_named_block(&self, name: &str) -> bool {
        self.unwrap_tag()
            .map(|x| *x.name.data == *name)
            .unwrap_or(false)
    }
    pub fn get_string(&'a self) -> Option<Ann<Atom<'a>>> {
        match self {
            Node::String(cow) => Some(cow.clone()),
            _ => None,
        }
    }
    pub fn get_enclosure_children(&self, kind: EnclosureKind) -> Option<&Vec<Node<'a>>> {
        match self {
            Node::Enclosure(Ann{
                data: x,
                ..
            }) if x.kind == kind => {
                Some(x.children.as_ref())
            }
            _ => None,
        }
    }
    pub fn unblock(self) -> Vec<Self> {
        match self {
            Node::Enclosure(
                Ann{data: block, ..}
            ) if block.kind == EnclosureKind::CurlyBrace => {
                block.children
            }
            x => vec![x]
        }
    }
    /// Unpacks an `Node::Enclosure` with the `Fragment` kind or
    /// returns a singleton vec.
    pub fn into_fragment(self) -> Vec<Self> {
        match self {
            Node::Enclosure(Ann{
                data: Enclosure{
                    kind: EnclosureKind::Fragment,
                    children
                },
                ..
            }) => children,
            x => vec![x]
        }
    }
    pub fn unwrap_tag(&self) -> Option<&Tag<'a>> {
        match self {
            Node::Tag(x) => Some(x),
            _ => None,
        }
    }
    pub fn unwrap_tag_mut(&mut self) -> Option<&mut Tag<'a>> {
        match self {
            Node::Tag(x) => Some(x),
            _ => None,
        }
    }
    pub fn unwrap_ident<'b>(&'b self) -> Option<&'b Ann<Atom<'a>>> {
        match self {
            Node::Ident(x) => Some(x),
            _ => None,
        }
    }
    pub fn unwrap_enclosure<'b>(&'b self) -> Option<&'b Ann<Enclosure<'a, Node<'a>>>> {
        match self {
            Node::Enclosure(x) => Some(x),
            _ => None,
        }
    }
    pub fn unwrap_curly_brace<'b>(&'b self) -> Option<&'b Vec<Node<'a>>> {
        match self {
            Node::Enclosure(
                Ann{data, ..}
            ) if data.kind == EnclosureKind::CurlyBrace => Some(&data.children),
            _ => None,
        }
    }
    pub fn unwrap_string<'b>(&'b self) -> Option<&'b Ann<Atom<'a>>> {
        match self {
            Node::String(x) => Some(x),
            _ => None,
        }
    }
    pub fn unwrap_string_mut<'b>(&'b mut self) -> Option<&'b mut Ann<Atom<'a>>> {
        match self {
            Node::String(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_tag(self) -> Option<Tag<'a>> {
        match self {
            Node::Tag(x) => Some(x),
            _ => None,
        }
    }

    /// Bottom up 'node to ndoe' transformation.
    pub fn transform<F: Fn(NodeEnvironment<'a>, Node<'a>) -> Node<'a>>(
        self,
        mut env: NodeEnvironment<'a>, f: Rc<F>
    ) -> Self {
        match self {
            Node::Tag(node) => {
                env.push_parent(node.name.data.clone());
                let children = node.children
                    .into_iter()
                    .map(|x| x.transform(env.clone(), f.clone()))
                    .collect();
                let rewrite_rules = node.rewrite_rules
                    .into_iter()
                    .map(|rule| -> RewriteRule<Node<'a>> {
                        RewriteRule {
                            from: rule.from.transform(env.clone(), f.clone()),
                            to: rule.to.transform(env.clone(), f.clone()),
                        }
                    })
                    .collect();
                let node = Tag {
                    name: node.name,
                    parameters: node.parameters,
                    children,
                    rewrite_rules,
                };
                f(env.clone(), Node::Tag(node))
            }
            Node::Enclosure(node) => {
                let kind = node.data.kind;
                let range = node.range;
                let children = node.data.children
                    .into_iter()
                    .map(|x| x.transform(env.clone(), f.clone()))
                    .collect();
                let data = Enclosure{
                    kind,
                    children,
                };
                let node = Node::Enclosure(Ann::join(range, data));
                f(env.clone(), node)
            }
            node @ Node::Ident(_) => {
                f(env.clone(), node)
            }
            node @ Node::String(_) => {
                f(env.clone(), node)
            }
            node @ Node::InvalidToken(_) => {
                f(env.clone(), node)
            }
        }
    }
    // pub fn transform_unit<U: Clone, F: Fn(NodeEnvironment<'a>, &Node<'a>) -> U>(
    //     &self,
    //     mut env: NodeEnvironment<'a>, f: Rc<F>
    // ) -> Vec<U> {
    //     match self {
    //         Node::Tag(node) => {
    //             env.push_parent(node.name.data.clone());
    //             let children = node.children
    //                 .into_iter()
    //                 .flat_map(|x| x.transform_unit(env.clone(), f.clone()))
    //                 .collect::<Vec<U>>();
    //             let rewrite_rules = node.rewrite_rules
    //                 .into_iter()
    //                 .flat_map(|rule| {
    //                     let from = rule.from.transform_unit(env.clone(), f.clone());
    //                     let to = rule.to.transform_unit(env.clone(), f.clone());
    //                     vec![from, to].concat()
    //                 })
    //                 .collect::<Vec<U>>();
    //             let node = f(env.clone(), &Node::Tag(node.clone()));
    //             vec![children, rewrite_rules, node].concat()
    //         }
    //         Node::Enclosure(node) => {
    //             let kind = node.data.kind;
    //             let range = node.range;
    //             let children = node.data.children
    //                 .into_iter()
    //                 .map(|x| x.transform_unit(env.clone(), f.clone()))
    //                 .collect();
    //             let data = Enclosure{
    //                 kind,
    //                 children,
    //             };
    //             let node = Node::Enclosure(Ann::join(range, data));
    //             f(env.clone(), node)
    //         }
    //         node @ Node::Ident(_) => {
    //             f(env.clone(), node)
    //         }
    //         node @ Node::String(_) => {
    //             f(env.clone(), node)
    //         }
    //         node @ Node::InvalidToken(_) => {
    //             f(env.clone(), node)
    //         }
    //     }
    // }
    pub fn transform_mut<F: FnMut(NodeEnvironment<'a>, Node<'a>) -> Node<'a>>(
        self,
        mut env: NodeEnvironment<'a>, f: Rc<RefCell<F>>
    ) -> Self {
        match self {
            Node::Tag(node) => {
                env.push_parent(node.name.data.clone());
                let children = node.children
                    .into_iter()
                    .map(|x| x.transform_mut(env.clone(), f.clone()))
                    .collect();
                let rewrite_rules = node.rewrite_rules
                    .into_iter()
                    .map(|rule| -> RewriteRule<Node<'a>> {
                        RewriteRule {
                            from: rule.from.transform_mut(env.clone(), f.clone()),
                            to: rule.to.transform_mut(env.clone(), f.clone()),
                        }
                    })
                    .collect();
                let node = Tag {
                    name: node.name,
                    parameters: node.parameters,
                    children,
                    rewrite_rules,
                };
                (f.borrow_mut())(env.clone(), Node::Tag(node))
            }
            Node::Enclosure(node) => {
                let kind = node.data.kind;
                let range = node.range;
                let children = node.data.children
                    .into_iter()
                    .map(|x| x.transform_mut(env.clone(), f.clone()))
                    .collect();
                let data = Enclosure{
                    kind,
                    children,
                };
                let node = Node::Enclosure(Ann::join(range, data));
                (f.borrow_mut())(env.clone(), node)
            }
            node @ Node::Ident(_) => {
                (f.borrow_mut())(env.clone(), node)
            }
            node @ Node::String(_) => {
                (f.borrow_mut())(env.clone(), node)
            }
            node @ Node::InvalidToken(_) => {
                (f.borrow_mut())(env.clone(), node)
            }
        }
    }
    /// Bottom up transformation of AST child nodes within the same enclosure.
    pub fn transform_children<F>(
        self,
        f: Rc<F>
    ) -> Self where F: Fn(Vec<Node<'a>>) -> Vec<Node<'a>> {
        match self {
            Node::Tag(node) => {
                let children = node.children
                    .into_iter()
                    .map(|x| -> Node {
                        x.transform_children(f.clone())
                    })
                    .collect::<Vec<_>>();
                let children = f(children);
                let rewrite_rules = node.rewrite_rules
                    .into_iter()
                    .map(|rule| -> RewriteRule<Node<'a>> {
                        RewriteRule {
                            from: rule.from.transform_children(f.clone()),
                            to: rule.to.transform_children(f.clone()),
                        }
                    })
                    .collect();
                let parameters = node.parameters;
                let node = Tag {
                    name: node.name,
                    parameters,
                    children,
                    rewrite_rules,
                };
                Node::Tag(node)
            }
            Node::Enclosure(node) => {
                let range = node.range();
                let children = node.data.children
                    .into_iter()
                    .map(|x| x.transform_children(f.clone()))
                    .collect();
                let children = (f)(children);
                let data = Enclosure{
                    kind: node.data.kind,
                    children,
                };
                Node::Enclosure(Ann::join(range, data))
            }
            node @ Node::Ident(_) => node,
            node @ Node::String(_) => node,
            node @ Node::InvalidToken(_) => node,
        }
    }
    /// For syntax highlighting VIA the compiler frontend.
    pub fn into_highlight_ranges(
        self,
        nesting: Vec<Atom<'a>>,
        binder: Option<Atom<'a>>,
    ) -> Vec<Highlight<'a>> {
        match self {
            Node::Tag(..) => {
                unimplemented!()
            }
            Node::Enclosure(node) => {
                let is_fragment = node.data.kind == EnclosureKind::Fragment;
                let range = node.range;
                let kind = match node.data.kind {
                    EnclosureKind::CurlyBrace => HighlightKind::CurlyBrace,
                    EnclosureKind::SquareParen => HighlightKind::SquareParen,
                    EnclosureKind::Parens => HighlightKind::Parens,
                    EnclosureKind::Fragment => HighlightKind::Fragment,
                    EnclosureKind::Error{open, close} => HighlightKind::Error{
                        open: open,
                        close: close,
                    },
                };
                let mut last_ident: Option<Atom> = None;
                let mut child_nesting = nesting.clone();
                if let Some(binder) = binder.clone() {
                    child_nesting.push(binder);
                }
                let children = node.data.children
                    .into_iter()
                    .flat_map(|x| {
                        if x.is_ident() {
                            let ident = x.unwrap_ident().unwrap().clone();
                            last_ident = Some(ident.data);
                        }
                        if x.is_string() && !x.is_whitespace() {
                            last_ident = None;
                        }
                        x.into_highlight_ranges(child_nesting.clone(), last_ident.clone())
                    })
                    .collect::<Vec<_>>();
                let highlight = Highlight {
                    kind,
                    range,
                    binder: binder.clone(),
                    nesting,
                };
                if is_fragment {
                    children
                } else {
                    let mut xs = vec![highlight];
                    xs.extend(children);
                    xs
                }
            }
            Node::Ident(value) => {
                let range = value.range;
                let highlight = Highlight {
                    kind: HighlightKind::Ident(value.data),
                    range,
                    binder: binder.clone(),
                    nesting,
                };
                vec![highlight]
            }
            Node::InvalidToken(value) => {
                let range = value.range;
                let highlight = Highlight {
                    kind: HighlightKind::InvalidToken(value.data),
                    range,
                    binder: binder.clone(),
                    nesting,
                };
                vec![highlight]
            }
            Node::String(value) => Vec::new(),
        }
    }

    pub fn to_string(&self) -> String {
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
            Node::Tag(tag) => {
                let name = pack(tag.name.data.clone());
                let children = tag.children
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("");
                format!("\\{}{}", name, children)
            }
            Node::Enclosure(Ann{data, ..}) => {
                let children = data.children
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("");
                match data.kind.clone() {
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
            Node::Ident(x) => ident(x.data.clone()),
            Node::String(x) => pack(x.data.clone()),
            Node::InvalidToken(x) => pack(x.data.clone()),
        }
    }
    pub fn syntactically_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Node::Tag(x1), Node::Tag(x2)) => {
                let check1 = x1.name() == x2.name();
                let check2 = x1.children.len() == x2.children.len();
                if check1 && check2 {
                    return x1.children
                        .iter()
                        .zip(x2.children.iter())
                        .all(|(x, y)| {
                            x.syntactically_equal(y)
                        })
                }
                false
            }
            (Node::Enclosure(x1), Node::Enclosure(x2)) => {
                let check1 = x1.data.kind == x2.data.kind;
                let check2 = x1.data.children.len() == x2.data.children.len();
                if check1 && check2 {
                    return x1.data.children
                        .iter()
                        .zip(x2.data.children.iter())
                        .all(|(x, y)| {
                            x.syntactically_equal(y)
                        })
                }
                false
            }
            (Node::Ident(x1), Node::Ident(x2)) => {
                &x1.data == &x2.data
            }
            (Node::String(x1), Node::String(x2)) => {
                &x1.data == &x2.data
            }
            (Node::InvalidToken(x1), Node::InvalidToken(x2)) => {
                &x1.data == &x2.data
            }
            (_, _) => unimplemented!()
        }
    }
    /// Push to a fragment or tag node.
    /// TODO: Should we also push to any `EnclosureKind`?
    pub fn push_child(self, child: Self) -> Self {
        match self {
            Node::Enclosure(mut node) if node.data.is_fragment() => {
                node.data.children.push(child);
                Node::Enclosure(node)
            }
            Node::Tag(mut tag) => {
                tag.children.push(child);
                Node::Tag(tag)
            }
            x => Node::new_fragment(vec![x, child])
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// HIGHLIGHTER RELATED DATA TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight<'a> {
    pub range: Option<CharRange>,
    pub kind: HighlightKind<'a>,
    pub binder: Option<Atom<'a>>,
    pub nesting: Vec<Atom<'a>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HighlightKind<'a> {
    CurlyBrace,
    SquareParen,
    Parens,
    Fragment,
    Error {
        open: Atom<'a>,
        close: Option<Atom<'a>>,
    },
    InvalidToken(Atom<'a>),
    Ident(Atom<'a>),
}

