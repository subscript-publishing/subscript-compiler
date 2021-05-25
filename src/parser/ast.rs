use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{VecDeque, LinkedList};

#[derive(Debug, Clone)]
pub struct Block<'a> {
    pub name: Option<&'a str>,
    pub parameters: Option<&'a str>,
    pub children: Vec<Ast<'a>>,
    pub rewrite_rules: Vec<(Ast<'a>, Ast<'a>)>,
}

#[derive(Debug, Clone)]
pub struct Content<'a> {
    pub string: &'a str,
}

#[derive(Debug, Clone)]
pub enum Ast<'a> {
    /// Function call
    Block(Block<'a>),
    /// Other tokens
    Token(&'a str),
    /// General text content
    Content(Content<'a>),
}

impl<'a> Ast<'a> {
    // pub fn match_block(
    //     &self,
    //     name: &str,
    // ) {
        
    // }

    pub fn is_name(&self, name: &str) -> bool {
        self.unpack_block()
            .and_then(|x| x.name)
            .map(|x| {
                println!("x: {}", x);
                if let Some(x) = x.strip_prefix("\\") {
                    x == name
                } else {
                    x == name
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
    pub fn is_token(&self) -> bool {
        match self {
            Ast::Token(_) => true,
            _ => false,
        }
    }
    pub fn is_content(&self) -> bool {
        match self {
            Ast::Content(_) => true,
            _ => false,
        }
    }

    pub fn unpack_block<'b>(&'b self) -> Option<&'b Block<'a>> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None
        }
    }
    pub fn unpack_block_mut<'b>(&'b mut self) -> Option<&'b mut Block<'a>> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None
        }
    }
    pub fn unpack_token(&self) -> Option<&'a str> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None
        }
    }
    pub fn unpack_content<'b>(&'b self) -> Option<&'b Content<'a>> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None
        }
    }

    pub fn into_block<'b>(self) -> Option<Block<'a>> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None
        }
    }
    pub fn into_token(self) -> Option<&'a str> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None
        }
    }
    pub fn into_content<'b>(self) -> Option<Content<'a>> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
pub enum Math {

}