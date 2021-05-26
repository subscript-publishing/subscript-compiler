use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use crate::frontend::data::{Symbol};

///////////////////////////////////////////////////////////////////////////////
// AST - BLOCK
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Block<'a> {
    pub name: Option<Cow<'a, str>>,
    pub parameters: Option<Cow<'a, str>>,
    pub children: Vec<Ast<'a>>,
    pub rewrite_rules: Vec<(Ast<'a>, Ast<'a>)>,
}

///////////////////////////////////////////////////////////////////////////////
// ROOT AST
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum Ast<'a> {
    Block(Block<'a>),
    Parens(Vec<Ast<'a>>),
    Content(Cow<'a, str>),
    Token(Cow<'a, str>),
}

impl<'a> Ast<'a> {
    pub fn is_anonymous_block(&self, name: &str) -> bool {
        self.unpack_block()
            .map(|x| {
                x.name.is_none()
            })
            .unwrap_or(false)
    }
    pub fn is_named_block(&self, name: &str) -> bool {
        self.unpack_block()
            .map(|x| {
                if let Some(x) = x.name.as_ref() {
                    return *x == name
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }
    pub fn is_block(&self) -> bool {
        match self {
            Ast::Block(_) => true,
            _ => false,
        }
    }
    pub fn is_parens(&self) -> bool {
        match self {
            Ast::Parens(_) => true,
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
    pub fn unpack_block(&self) -> Option<&Block<'a>> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_parens(&self) -> Option<&Vec<Ast<'a>>> {
        match self {
            Ast::Parens(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_content(&self) -> Option<&Cow<'a, str>> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None,
        }
    }
    pub fn unpack_token(&self) -> Option<&Cow<'a, str>> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_block(self) -> Option<Block<'a>> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_parens(self) -> Option<Vec<Ast<'a>>> {
        match self {
            Ast::Parens(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_content(self) -> Option<Cow<'a, str>> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None,
        }
    }
    pub fn into_token(self) -> Option<Cow<'a, str>> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None,
        }
    }
}



///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn run() {
    let source = include_str!("../../source.txt");
    let result = crate::frontend::parser::run_parser_internal_ast(source);
    
}


