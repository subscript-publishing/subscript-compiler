use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{VecDeque, LinkedList};

///////////////////////////////////////////////////////////////////////////////
// AST VEC
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct ChildList<'a>(pub Vec<Ast<'a>>);

impl<'a> ChildList<'a> {
    // pub fn into_zipper(&self, f: impl Fn(
    //     Ast<'a>,
    //     Ast<'a>,
    //     Ast<'a>
    // )) {

    // }
}



///////////////////////////////////////////////////////////////////////////////
// GENERAL AST
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Block<'a> {
    pub name: Option<&'a str>,
    pub parameters: Option<&'a str>,
    pub children: ChildList<'a>,
    pub rewrite_rules: Vec<(Ast<'a>, Ast<'a>)>,
}

impl<'a> Block<'a> {
    pub fn is_anonymous_block(&self) -> bool {
        self.name == None
    }
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
    pub fn is_anonymous_block(&self) -> bool {
        self.unpack_block()
            .map(|block| {
                block.name == None
            })
            .unwrap_or(false)
    }
    pub fn is_name(&self, name: &str) -> bool {
        self.unpack_block()
            .and_then(|x| x.name)
            .map(|x| {
                if let Some(x) = x.strip_prefix("\\") {
                    x == name
                } else {
                    x == name
                }
            })
            .unwrap_or(false)
    }
    pub fn token_matches(&self, value: &str) -> bool {
        self.unpack_token()
            .map(|x| x == value)
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
    pub fn to_backend(self) -> crate::backend::ast::Ast {
        use crate::backend::ast;
        use std::borrow::ToOwned;
        match self {
            Ast::Block(node) => {
                let children = node.children.0
                    .into_iter()
                    .map(Self::to_backend)
                    .collect::<Vec<_>>();
                let children = ast::ChildList(children);
                let rewrite_rules = node.rewrite_rules
                    .into_iter()
                    .map(|(x, y)| (x.to_backend(), y.to_backend()))
                    .collect();
                ast::Ast::Block(ast::Block{
                    name: node.name.map(ToOwned::to_owned),
                    parameters: node.parameters.map(ToOwned::to_owned),
                    children,
                    rewrite_rules,
                })
            }
            Ast::Content(node) => {
                ast::Ast::Content(ast::Content::new(node.string))
            }
            Ast::Token(node) => {
                ast::Ast::Token(node.to_owned())
            }
        }
    }
}


#[derive(Debug, Clone)]
pub enum Math {

}