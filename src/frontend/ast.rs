//! Frontend AST data types & related.
use std::rc::Rc;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use crate::backend;
use crate::backend::data::*;

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

#[derive(Debug, Clone)]
pub struct Highlight {
    range: CharRange,
    kind: Scope,
}

#[derive(Debug, Clone)]
pub enum Scope {
    Ident,
    Block,
}

