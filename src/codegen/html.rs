use std::borrow::Cow;
use std::collections::HashMap;
use crate::compiler::data::{LayoutKind, Text};

///////////////////////////////////////////////////////////////////////////////
// BASICS
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub enum Image {
    Svg {
        kind: LayoutKind,
        payload: String,
    },
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Image::Svg{kind, payload} => {
                f.debug_struct("Svg")
                    .field("kind", &kind)
                    .field("payload", &String::from("DataNotShown"))
                    .finish()
            }
        }
    }
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
        match self {
            Node::Text(node) => node,
            Node::Element(node) => {
                let attributes = node.attributes
                    .into_iter()
                    .map(|(left, right)| -> String {
                        let mut result = String::new();
                        result.push_str(&format!("{}", left.0));
                        result.push_str("=");
                        result.push_str(&format!("{:?}", right.0));
                        result
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let attributes = {
                    if attributes.is_empty() {
                        Text::default()
                    } else {
                        let mut attrs = attributes;
                        attrs.insert(0, ' ');
                        Text::from_string(attrs)
                    }
                };
                let children = node.children
                    .into_iter()
                    .map(Node::to_html_str)
                    .map(|x| x.0)
                    .collect::<Vec<_>>()
                    .join("");
                let children = Text::from_string(children);
                Text::from_string(format!(
                    "<{name}{attrs}>{children}</{name}>",
                    name=node.name,
                    attrs=attributes,
                    children=children,
                ))
            }
            Node::Fragment(nodes) => {
                let children = nodes
                    .into_iter()
                    .map(Node::to_html_str)
                    .map(|x| x.0)
                    .collect::<Vec<_>>()
                    .join("");
                Text::from_string(children)
            }
            Node::Image(image) => {
                unimplemented!()
            }
        }
    }
}

pub fn render_document<'a>(html: Vec<Node<'a>>) -> String {
    let body = html
        .into_iter()
        .map(Node::to_html_str)
        .map(|x| x.0)
        .collect::<Vec<_>>()
        .join("\n");
    let html = String::from(include_str!("../../assets/template.html.txt"));
    let html = html.replace("{{deps}}", include_str!("../../assets/deps.html"));
    let html = html.replace("{{css}}", include_str!("../../assets/styling.css"));
    html.replace("{{body}}", &body)
}

///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub(crate) fn dev() {
    // let source = include_str!("../../source.txt");
    // let parsed = crate::backend::parser::run_parser(source);
    // let result = crate::backend::passes::to_html_pipeline(parsed);
    // // let result = crate::frontend::parser::run_parser(source);
    // // let result = passes(result);
    // // let result = result
    // //     .into_iter()
    // //     .map(to_html)
    // //     .collect::<Vec<_>>();
    // for node in result.clone() {
    //     println!("{:#?}", node.to_html_str());
    // }
    // // println!("------------------------------------------------------------");
    // let output = render_document(result);
    // // println!("{}", result);
    // std::fs::write("output.html", output);
}



