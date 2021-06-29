//! Common data types used throughout the compiler.
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque, LinkedList};
use std::iter::FromIterator;
use lazy_static::lazy_static;


pub static INLINE_MATH_TAG: &'static str = "[inline-math]";
pub static BLOCK_MATH_TAGS: &[&'static str] = &[
    "equation",
];


lazy_static! {
    pub static ref HEADING_TAG_NAMES: HashSet<&'static str> = HashSet::from_iter(vec![
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
    ]);
}

pub static ALL_SUBSCRIPT_TAGS: &[&'static str] = &[
    "note",
    "layout",
    "equation",
];

pub static ALLOWED_HTML_TAGS: &[&'static str] = &[
    "address",
    "article",
    "aside",
    "footer",
    "header",
    "h1",
    "section",
    "blockquote",
    "dd",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "hr",
    "li",
    "ol",
    "p",
    "pre",
    "ul",
    "a",
    "abbr",
    "b",
    "bdi",
    "bdo",
    "br",
    "cite",
    "code",
    "data",
    "dfn",
    "em",
    "i",
    "kbd",
    "mark",
    "q",
    "rb",
    "rp",
    "rt",
    "rtc",
    "ruby",
    "s",
    "samp",
    "small",
    "span",
    "strong",
    "sub",
    "sup",
    "time",
    "u",
    "var",
    "wbr",
    "area",
    "audio",
    "img",
    "map",
    "track",
    "video",
    "embed",
    "iframe",
    "object",
    "param",
    "picture",
    "portal",
    "source",
    "svg",
    "math",
    "canvas",
    "noscript",
    "script",
    "del",
    "ins",
    "caption",
    "col",
    "colgroup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "button",
    "datalist",
    "fieldset",
    "form",
    "input",
    "label",
    "legend",
    "meter",
    "optgroup",
    "option",
    "output",
    "progress",
    "select",
    "textarea",
    "details",
    "dialog",
    "menu",
    "summary",
    "slot",
    "template",
    "acronym",
    "applet",
    "basefont",
    "bgsound",
    "big",
    "blink",
    "center",
    "content",
    "dir",
    "font",
    "frame",
    "frameset",
    "hgroup",
    "image",
    "keygen",
    "marquee",
    "menuitem",
    "nobr",
    "noembed",
    "noframes",
    "plaintext",
    "rb",
    "rtc",
    "shadow",
    "spacer",
    "strike",
    "tt",
];


///////////////////////////////////////////////////////////////////////////////
// LAYOUT
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum LayoutKind {
    Block,
    Inline,
}


///////////////////////////////////////////////////////////////////////////////
// STRING DATA TYPES
///////////////////////////////////////////////////////////////////////////////

pub type Atom<'a> = Cow<'a, str>;

#[derive(Debug, Clone, Hash, Eq, Default)]
pub struct Text<'a>(pub Cow<'a, str>);

impl<'a> Text<'a> {
    pub fn new(value: &'a str) -> Self {
        Text(Cow::Borrowed(value))
    }
    pub fn from_string(value: String) -> Self {
        Text(Cow::Owned(value))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn append(self, other: Text<'a>) -> Self {
        Text(self.0 + other.0)
    }
}
impl<'a> std::fmt::Display for Text<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<'a> PartialEq for Text<'a> {
    fn eq(&self, other: &Self) -> bool {
        let left = &*self.0;
        let right = &*other.0;
        left == right
    }
}

impl<'a> PartialEq<str> for Text<'a> {
    fn eq(&self, other: &str) -> bool {
        let left = &*self.0;
        let right = other;
        left == right
    }
}



pub static TOKEN_SET: &'static [&'static str] = &["\\", "[", "]", "{", "}", "(", ")", "=>", "_", "^"];

fn get_end_kind_for(begin_kind: &str) -> &str {
    match begin_kind {
        "{" => "}",
        "[" => "]",
        "(" => ")",
        _ => unreachable!()
    }
}

fn get_begin_kind_for(end_kind: &str) -> &str {
    match end_kind {
        "}" => "{",
        "]" => "[",
        ")" => "(",
        _ => unreachable!()
    }
}

pub fn is_token<'a>(value: &'a str) -> bool {
    for tk in TOKEN_SET {
        if *tk == value {
            return true;
        }
    }
    false
}


///////////////////////////////////////////////////////////////////////////////
// COMMON AST RELATED DATA TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq)]
pub struct RewriteRule<T> {
    pub from: T,
    pub to: T,
}

#[derive(Debug, Clone)]
pub struct CurlyBrace<T>(pub Vec<T>);

#[derive(Debug, Clone)]
pub struct SquareParen<T>(pub Vec<T>);

#[derive(Debug, Clone, PartialEq)]
pub enum EnclosureKind<'a> {
    CurlyBrace,
    SquareParen,
    Parens,
    /// Intenral - akin to HTML fragment which is just a list of nodes.
    Fragment,
    Error {
        open: Atom<'a>,
        close: Option<Atom<'a>>,
    },
}

impl<'a> EnclosureKind<'a> {
    pub fn new(open: Atom<'a>, close: Atom<'a>) -> EnclosureKind<'a> {
        EnclosureKind::parse(open, Some(close))
    }
    pub fn parse(open: Atom<'a>, close: Option<Atom<'a>>) -> EnclosureKind<'a> {
        let open_str: &str = &open;
        match (open_str, close.as_ref()) {
            ("{", Some(x)) if x == "}" => EnclosureKind::CurlyBrace,
            ("[", Some(x)) if x == "]" => EnclosureKind::SquareParen,
            ("(", Some(x)) if x == ")" => EnclosureKind::Parens,
            (_, _) => EnclosureKind::Error {open, close},
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enclosure<'a, T> {
    pub kind: EnclosureKind<'a>,
    pub children: Vec<T>,
}

impl<'a, T> Enclosure<'a, T> {
    pub fn is_curly_brace(&self) -> bool {
        match self.kind {
            EnclosureKind::CurlyBrace => true,
            _ => false,
        }
    }
    pub fn is_square_parens(&self) -> bool {
        match self.kind {
            EnclosureKind::SquareParen => true,
            _ => false,
        }
    }
    pub fn is_parens(&self) -> bool {
        match self.kind {
            EnclosureKind::Parens => true,
            _ => false,
        }
    }
    pub fn is_error(&self) -> bool {
        match self.kind {
            EnclosureKind::Error{..} => true,
            _ => false,
        }
    }
    pub fn is_fragment(&self) -> bool {
        match self.kind {
            EnclosureKind::Fragment{..} => true,
            _ => false,
        }
    }
}

impl<'a, T> Enclosure<'a, T> {
    pub fn new_curly_brace(children: Vec<T>) -> Self {
        Enclosure {
            kind: EnclosureKind::CurlyBrace,
            children
        }
    }
    pub fn new_curly_brace_(child: T) -> Self {
        Enclosure {
            kind: EnclosureKind::CurlyBrace,
            children: vec![child]
        }
    }
}
