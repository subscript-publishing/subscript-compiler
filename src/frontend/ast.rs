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
    pub fn substrng<'a>(&self, source: &'a str) -> Option<&'a str> {
        fn find_utf8_end(s: &str, i: usize) -> Option<usize> {
            s.char_indices().nth(i).map(|(_, x)| x.len_utf8())
        }
        let start_byte = self.start.byte_index;
        let end_byte = self.end.byte_index;
        source
            .get(start_byte..=end_byte)
            .or_else(|| {
                let corrected_end = find_utf8_end(source, end_byte)?;
                source.get(start_byte..=end_byte)
            })
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
    // pub fn highlight_ranges(self) -> Vec<>
}


///////////////////////////////////////////////////////////////////////////////
// HIGHLIGHTER RELATED DATA TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    range: CharRange,
    scope: Scope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Scope {
    CurlyBrace,
    SquareParen,
    Parens,
    Error {
        open: String,
        close: String,
    },
}

