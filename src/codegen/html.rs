use std::borrow::Cow;
use std::collections::HashMap;
use crate::frontend::data::{LayoutKind, Text};

///////////////////////////////////////////////////////////////////////////////
// BASICS
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum Image {
    Svg {
        kind: LayoutKind,
        payload: String,
    },
}

#[derive(Debug, Clone)]
pub enum ImageType {
    Svg,
}

impl Image {
    pub fn layout(&self) -> LayoutKind {
        match self {
            Self::Svg { kind, payload } => kind.clone(),
        }
    }
    pub fn image_type(&self) -> ImageType {
        match self {
            Self::Svg {..} => ImageType::Svg,
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// HTML TREE
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Element<'a> {
    pub name: Text<'a>,
    pub attributes: HashMap<Text<'a>, Text<'a>>,
    pub children: Vec<Node<'a>>,
}


#[derive(Debug, Clone)]
pub enum Node<'a> {
    Element(Element<'a>),
    Text(Text<'a>),
    Image(Image),
    Fragment(Vec<Node<'a>>),
}

impl<'a> Node<'a> {
    pub fn new_text(val: &'a str) -> Self {
        Node::Text(Text::new(val))
    }
    pub fn to_html_str(self) -> Text<'a> {
        unimplemented!()
    }
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn dev() {
    
}



