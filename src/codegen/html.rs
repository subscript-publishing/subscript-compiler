use std::borrow::Cow;
use std::collections::HashMap;
use crate::frontend::data::Text;

///////////////////////////////////////////////////////////////////////////////
// MATH-ML
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum MathMl {

}

///////////////////////////////////////////////////////////////////////////////
// HTML
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Element<'a> {
    name: Text<'a>,
    attributes: HashMap<Text<'a>, Text<'a>>,
    children: Vec<Node<'a>>,
}

#[derive(Debug, Clone)]
pub enum Node<'a> {
    Element(Element<'a>),
    Text(Text<'a>),
    Fragment(Vec<Node<'a>>),
}

impl<'a> Node<'a> {
    pub fn to_html_str(self) -> Text<'a> {
        let left = Text(Cow::Borrowed("left"));
        let right = Text(Cow::Borrowed("right"));
        let res = left.append(right);
        res
    }
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn dev() {
    let left = Text(Cow::Borrowed("left"));
    let right = Text(Cow::Borrowed("right"));
    let res = left.append(right);
    println!("res: {}", res);
}



