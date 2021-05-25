use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{VecDeque, LinkedList};
use crate::parser_utils;

#[derive(Debug, Clone)]
pub enum Node<'a> {
    Call(CurlyBraceMode<'a>),
    Text{string: &'a str},
}

#[derive(Debug, Clone)]
pub struct Parser {
    column: usize,
}

#[derive(Debug, Clone)]
pub struct NameMode<'a> {
    start_index: usize,
    arguments: Option<&'a str>
}

#[derive(Debug, Clone)]
pub struct HeaderMode<'a> {
    name: &'a str,
    arguments: Option<&'a str>
}

#[derive(Debug, Clone)]
pub struct CurlyBraceMode<'a> {
    header: HeaderMode<'a>,
    start_index: usize,
    children: LinkedList<Node<'a>>,
}

#[derive(Debug, Clone)]
pub struct SquareBracketMode<'a> {
    name: &'a str,
    start_index: usize,
}

#[derive(Debug, Clone)]
pub struct TextMode {
    start_index: usize,
}

#[derive(Debug, Clone)]
enum State<'a> {
    NameMode(NameMode<'a>),
    SquareBracketMode(SquareBracketMode<'a>),
    HeaderMode(HeaderMode<'a>),
    CurlyBraceMode(CurlyBraceMode<'a>),
    TextMode(TextMode),
}

impl<'a> State<'a> {
    fn is_name_mode(&self) -> bool {
        self.unwrap_name_mode().is_some()
    }
    fn is_curly_brace_mode(&self) -> bool {
        self.unwrap_curly_brace_mode().is_some()
    }
    fn is_header_mode(&self) -> bool {
        self.unwrap_header_mode().is_some()
    }
    fn is_square_bracket_mode(&self) -> bool {
        self.unwrap_square_bracket_mode().is_some()
    }
    fn is_text_mode(&self) -> bool {
        self.unwrap_text_mode().is_some()
    }

    fn unwrap_name_mode(&self) -> Option<NameMode> {
        match self {
            State::NameMode(x) => Some(x.clone()),
            _ => None
        }
    }
    fn unwrap_header_mode(&self) -> Option<HeaderMode> {
        match self {
            State::HeaderMode(x) => Some(x.clone()),
            _ => None
        }
    }
    fn unwrap_curly_brace_mode(&self) -> Option<CurlyBraceMode> {
        match self {
            State::CurlyBraceMode(x) => Some(x.clone()),
            _ => None
        }
    }
    fn unwrap_square_bracket_mode(&self) -> Option<SquareBracketMode> {
        match self {
            State::SquareBracketMode(x) => Some(x.clone()),
            _ => None
        }
    }
    fn unwrap_text_mode(&self) -> Option<TextMode> {
        match self {
            State::TextMode(x) => Some(x.clone()),
            _ => None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Stack<'a>(VecDeque<State<'a>>);

impl<'a> Stack<'a> {
    fn push(&mut self, entry: State<'a>) {
        self.0.push_front(entry);
    }
    fn pop_name_mode(&mut self) -> Option<NameMode<'a>> {
        let mut index: Option<usize> = None;
        for (ix,entry) in self.0.iter().enumerate() {
            if entry.is_name_mode() {
                match self.0.remove(ix).unwrap() {
                    State::NameMode(x) => return Some(x),
                    _ => panic!()
                }
            }
        }
        None
    }
    fn pop_curly_brace_mode(&mut self) -> Option<CurlyBraceMode<'a>> {
        for (ix,entry) in self.0.iter().enumerate() {
            if entry.is_curly_brace_mode() {
                match self.0.remove(ix).unwrap() {
                    State::CurlyBraceMode(x) => return Some(x),
                    _ => panic!()
                }
            }
        }
        None
    }
    fn pop_square_bracket_mode(&mut self) -> Option<SquareBracketMode<'a>> {
        for (ix,entry) in self.0.iter().enumerate() {
            if entry.is_square_bracket_mode() {
                match self.0.remove(ix).unwrap() {
                    State::SquareBracketMode(x) => return Some(x),
                    _ => panic!()
                }
            }
        }
        None
    }
    fn pop_header_mode(&mut self) -> Option<HeaderMode<'a>> {
        for (ix,entry) in self.0.iter().enumerate() {
            if entry.is_header_mode() {
                match self.0.remove(ix).unwrap() {
                    State::HeaderMode(x) => return Some(x),
                    _ => panic!()
                }
            }
        }
        None
    }
    fn pop_text_mode(&mut self) -> Option<TextMode> {
        let mut index: Option<usize> = None;
        for (ix,entry) in self.0.iter().enumerate() {
            if entry.is_text_mode() && index.is_none() {
                index = Some(ix);
            }
            break
        }
        match index {
            Some(index) => {
                Some(self.0.remove(index).unwrap().unwrap_text_mode().unwrap())
            }
            None => None
        }
    }
}

fn tick<'a, 'b>(
    source: &'a str,
    ix: usize,
    ch: char,
    stack: &'b mut Stack<'a>,
    top_level: &'b mut LinkedList<Node<'a>>,
) -> Option<usize> {
    let mut end_text_mode = || {
        if let Some(text_mode) = stack.pop_text_mode() {
            let string = &source[text_mode.start_index..ix];
            let node = Node::Text{string};
            if let Some(mut parent) = stack.pop_curly_brace_mode() {
                parent.children.push_back(node);
                stack.push(State::CurlyBraceMode(parent));
            } else {
                top_level.push_back(node);
            }
        }
    };
    fn start_text_mode<'a, 'b>(
        source: &'a str,
        ix: usize,
        stack: &'b mut Stack<'a>,
    ) {
        if stack.pop_text_mode().is_none() {
            stack.push(State::TextMode(TextMode {
                start_index: ix,
            }));
        }
    }
    // let mut is_token = None;
    match ch {
        '\\' => {
            // println!("name: {:?}", name);
            end_text_mode();
            assert!(stack.pop_square_bracket_mode().is_none());
            stack.push(State::NameMode(NameMode{
                start_index: ix,
                arguments: None,
            }));
        }
        '[' => {
            end_text_mode();
            let NameMode{start_index, arguments} = stack.pop_name_mode().unwrap();
            let name = &source[start_index..ix];
            stack.push(State::SquareBracketMode(SquareBracketMode{
                name,
                start_index: ix,
            }));
        }
        ']' => {
            end_text_mode();
            let SquareBracketMode{name, start_index} = stack.pop_square_bracket_mode().unwrap();
            let arguments = &source[start_index..=ix];
            stack.push(State::HeaderMode(HeaderMode{
                name,
                arguments: Some(arguments),
            }));
        }
        '{' => {
            end_text_mode();
            assert!(stack.pop_square_bracket_mode().is_none());
            if let Some(header) = stack.pop_header_mode() {
                stack.push(State::CurlyBraceMode(CurlyBraceMode{
                    header,
                    start_index: ix,
                    children: Default::default(),
                }));
            } else {
                let NameMode{start_index, arguments} = stack.pop_name_mode().unwrap();
                let name = &source[start_index..ix];
                stack.push(State::CurlyBraceMode(CurlyBraceMode{
                    header: HeaderMode {
                        name,
                        arguments: None
                    },
                    start_index: ix,
                    children: Default::default(),
                }));
            }
            start_text_mode(source, ix + 1, stack);
        }
        '}' => {
            end_text_mode();
            let node = stack.pop_curly_brace_mode().unwrap();
            let node = Node::Call(node);
            if let Some(mut parent) = stack.pop_curly_brace_mode() {
                parent.children.push_back(node);
                stack.push(State::CurlyBraceMode(parent));
            } else {
                top_level.push_back(node);
            }
            start_text_mode(source, ix + 1, stack);
        }
        ch => {},
    }
    None
}

fn dev() {
    let source = include_str!("../source.txt");
    let mut stack = Stack::default();
    let mut top_level: LinkedList<Node> = Default::default();
    let mut skip_to: Option<usize> = None;
    for (ix, ch) in source.chars().enumerate() {
        if let Some(start_from) = skip_to {
            if ix < start_from {
                continue
            } else {
                skip_to = None
            }
        }
        let res = tick(
            source,
            ix,
            ch,
            &mut stack,
            &mut top_level,
        );
        if let Some(start_from) = res {
            assert!(start_from > ix);
            skip_to = Some(start_from);
        }
    }
    println!("top_level: {:#?}", top_level);
}


pub fn run() {
    dev();
}