use std::borrow::Cow;
use std::collections::HashMap;
use crate::frontend::data::{LayoutKind, Text};

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
                        let key: &str = &left.0;
                        let value: &str = &right.0;
                        let value = value.strip_prefix("\'").unwrap_or(value);
                        let value = value.strip_prefix("\"").unwrap_or(value);
                        let value = value.strip_suffix("\'").unwrap_or(value);
                        let value = value.strip_suffix("\"").unwrap_or(value);
                        result.push_str(key);
                        result.push_str("=");
                        result.push_str(&format!("{:?}", value));
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


/// Render the entire document.
#[derive(Debug, Clone)]
pub struct Document<'a> {
    pub toc: Node<'a>,
    pub body: Vec<Node<'a>>
}

impl<'a> Document<'a> {
    pub fn from_source(source: &'a str) -> Document<'a> {
        let body = crate::frontend::pass::pp_normalize::run_compiler_frontend(source);
        let body = crate::frontend::pass::html_normalize::html_canonicalization(body);
        let body = body
            .into_iter()
            .map(crate::frontend::pass::math::latex_pass)
            .collect::<Vec<_>>();
        let toc = crate::frontend
            ::pass
            ::html_normalize
            ::generate_table_of_contents_tree(
                &crate::frontend::ast::Node::new_fragment(body.clone())
            );
        let toc = crate::frontend::pass::to_html::node_to_html(
            crate::frontend::pass::math::latex_pass(toc)
        );
        let body = body
            .into_iter()
            .map(crate::frontend::pass::html_normalize::annotate_heading_nodes)
            .map(crate::frontend::pass::to_html::node_to_html)
            .collect::<Vec<_>>();
        Document{toc, body}
    }
    pub fn render_to_string(self) -> String {
        let toc = self.toc.to_html_str().to_string();
        let body = self.body
            .into_iter()
            .map(Node::to_html_str)
            .map(|x| x.0)
            .collect::<Vec<_>>()
            .join("\n");
        String::from(include_str!("../../assets/template.html"))
            .replace("<!--{{deps}}-->", include_str!("../../assets/deps.html"))
            .replace("/*{{css}}*/", include_str!("../../assets/styling.css"))
            .replace("<!--{{toc}}-->", &toc)
            .replace("<!--{{body}}-->", &body)
    }
}



