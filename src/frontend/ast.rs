use std::rc::Rc;
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use std::vec;
use crate::backend;
use crate::backend::data::*;


#[derive(Debug, Clone)]
pub enum Node<'a> {
    Ident(Ann<Atom<'a>>),
    Enclosure(Ann<Enclosure<'a, Node<'a>>>),
    String(Ann<Atom<'a>>),
}

