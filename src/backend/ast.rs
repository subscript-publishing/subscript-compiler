use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{VecDeque, LinkedList};

///////////////////////////////////////////////////////////////////////////////
// AST VEC
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct ChildList(pub Vec<Ast>);

impl ChildList {
    pub fn transform<F: Fn(Ast) -> Ast>(self, f: Rc<F>) -> Self {
        let children = self.0
            .into_iter()
            .map(|x| f(x))
            .collect();
        ChildList(children)
    }
}


///////////////////////////////////////////////////////////////////////////////
// AST BRANCH
///////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone)]
pub struct Block {
    pub name: Option<String>,
    pub parameters: Option<String>,
    pub children: ChildList,
    pub rewrite_rules: Vec<(Ast, Ast)>,
}

impl Block {
    pub fn is_anonymous_block(&self) -> bool {
        self.name == None
    }
    pub fn apply_rewrite_rules() {

    }
}


///////////////////////////////////////////////////////////////////////////////
// AST LEAF
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Content {
    pub string: String,
}

impl Content {
    pub fn new(value: &str) -> Self {
        Content{
            string: value.to_owned(),
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// GENERAL AST
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum Ast {
    /// Function call
    Block(Block),
    /// Other tokens
    Token(String),
    /// General text content
    Content(Content),
}

impl Ast {
    /// Bottom up transformation.
    pub fn transform<F: Fn(Ast) -> Ast>(self, f: Rc<F>) -> Self {
        match self {
            Ast::Block(node) => {
                let children = node.children.transform(f.clone());
                let rewrite_rules = node.rewrite_rules
                    .into_iter()
                    .map(|(x, y)| -> (Ast, Ast) {
                        (f(x), f(y))
                    })
                    .collect();
                let node = Block {
                    name: node.name,
                    parameters: node.parameters,
                    children,
                    rewrite_rules,
                };
                f(Ast::Block(node))
            }
            node @ Ast::Content(_) => {
                f(node)
            }
            node @ Ast::Token(_) => {
                f(node)
            }
        }
    }
    pub fn is_anonymous_block(&self) -> bool {
        self.unpack_block()
            .map(|block| {
                block.name == None
            })
            .unwrap_or(false)
    }
    pub fn is_name(&self, name: &str) -> bool {
        self.unpack_block()
            .map(|x| {
                if let Some(x) = x.name.as_ref() {
                    if let Some(x) = x.strip_prefix("\\") {
                        x == name
                    } else {
                        x == name
                    }
                } else {
                    false
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

    pub fn unpack_block(&self) -> Option<&Block> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None
        }
    }
    pub fn unpack_block_mut(&mut self) -> Option<&mut Block> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None
        }
    }
    pub fn unpack_token(&self) -> Option<&str> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None
        }
    }
    pub fn unpack_content(&self) -> Option<&Content> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None
        }
    }

    pub fn into_block(self) -> Option<Block> {
        match self {
            Ast::Block(x) => Some(x),
            _ => None
        }
    }
    pub fn into_token(self) -> Option<String> {
        match self {
            Ast::Token(x) => Some(x),
            _ => None
        }
    }
    pub fn into_content(self) -> Option<Content> {
        match self {
            Ast::Content(x) => Some(x),
            _ => None
        }
    }
}
