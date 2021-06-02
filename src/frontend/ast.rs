//! Frontend AST data types & related.
use std::rc::Rc;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use serde::{Serialize, Deserialize};
use crate::backend;
use crate::backend::data::*;

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
        Ann::from_range(self, data)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ann<T> {
    pub start: CharIndex,
    pub end: CharIndex,
    pub data: T,
}

impl<T> Ann<T> {
    pub fn from_range(range: CharRange, data: T) -> Self {
        Ann {
            start: range.start,
            end: range.end,
            data,
        }
    }
    pub fn into_char_range(&self) -> CharRange {
        let start = self.start;
        let end = self.end;
        CharRange{start, end}
    }
}

///////////////////////////////////////////////////////////////////////////////
// AST
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum Node<'a> {
    Ident(Ann<Atom<'a>>),
    Enclosure(Ann<Enclosure<'a, Node<'a>>>),
    String(Ann<Atom<'a>>),
}


impl<'a> Node<'a> {
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
    pub fn unwrap_string<'b>(&'b self) -> Option<&'b Ann<Atom<'a>>> {
        match self {
            Node::String(x) => Some(x),
            _ => None,
        }
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
    pub fn into_highlight_ranges(
        self,
        nesting: usize,
        binder: Option<Atom<'a>>,
    ) -> Vec<Highlight<'a>> {
        match self {
            Node::Enclosure(node) => {
                let is_fragment = node.data.kind == EnclosureKind::Fragment;
                let range = node.into_char_range();
                let kind = match node.data.kind {
                    EnclosureKind::CurlyBrace => HighlightKind::CurlyBrace,
                    EnclosureKind::SquareParen => HighlightKind::SquareParen,
                    EnclosureKind::Parens => HighlightKind::Parens,
                    EnclosureKind::Fragment => HighlightKind::Fragment,
                    EnclosureKind::Error{open, close} => HighlightKind::Error{
                        open: Cow::Borrowed(open),
                        close: Cow::Borrowed(close),
                    },
                };
                let mut last_ident: Option<Atom> = None;
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
                        x.into_highlight_ranges(nesting + 1, last_ident.clone())
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
                let range = value.into_char_range();
                let highlight = Highlight {
                    kind: HighlightKind::Ident(value.data),
                    range,
                    binder: binder.clone(),
                    nesting,
                };
                vec![highlight]
            }
            Node::String(value) => Vec::new(),
        }
    }
    pub fn new_fragment(nodes: Vec<Self>) -> Self {
        Node::Enclosure(Ann{
            start: CharIndex::zero(),
            end: CharIndex::zero(),
            data: Enclosure {
                kind: EnclosureKind::Fragment,
                children: nodes,
            }
        })
    }
    /// Unpacks an `Node::Enclosure` with the `Fragment` kind or
    /// returns a singleton vec.
    pub fn into_fragment(self) -> Vec<Self> {
        match self {
            Node::Enclosure(Ann{
                start,
                end,
                data: Enclosure{
                    kind: EnclosureKind::Fragment,
                    children
                }
            }) => children,
            x => vec![x]
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// HIGHLIGHTER RELATED DATA TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight<'a> {
    pub range: CharRange,
    pub kind: HighlightKind<'a>,
    pub binder: Option<Atom<'a>>,
    pub nesting: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HighlightKind<'a> {
    CurlyBrace,
    SquareParen,
    Parens,
    Fragment,
    Error {
        open: Atom<'a>,
        close: Atom<'a>,
    },
    Ident(Atom<'a>),
}

