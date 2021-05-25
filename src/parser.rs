use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{VecDeque, LinkedList};
use crate::parser_utils;

pub mod ast;
use ast::{Ast, Block, Content};

///////////////////////////////////////////////////////////////////////////////
// PARSE NODE
///////////////////////////////////////////////////////////////////////////////


///////////////////////////////////////////////////////////////////////////////
// PARSE NODE
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum Node<'a> {
    /// Function call
    Call {
        start: usize,
        end: usize,
        call: CurlyBraceMode<'a>,
    },
    /// Other tokens
    Token(Token<'a>),
    /// General text content
    Text{
        start: usize,
        end: usize,
        string: &'a str,
    },
}

impl<'a> Node<'a> {
    // pub fn get_name(&self) -> Option<&'a str> {
    //     match self {
    //         Node::Call{start, end, call} => {
    //             match call.header.clone() {
    //                 Some(header) => Some(header.name),
    //                 None => None
    //             }
    //         }
    //         _ => None
    //     }
    // }
    // pub fn is_call(&self) -> Option<CurlyBraceMode<'a>> {
    //     match self {
    //         Node::Call{start, end, call} => {
    //             Some(call.clone())
    //         }
    //         _ => None
    //     }
    // }
}


///////////////////////////////////////////////////////////////////////////////
// BASIC PARSER TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Token<'a>(pub &'a str);

#[derive(Debug, Clone)]
pub struct NameMode<'a> {
    pub start_index: usize,
    pub arguments: Option<&'a str>
}

#[derive(Debug, Clone)]
pub struct HeaderMode<'a> {
    pub name_start_index: usize,
    pub name: &'a str,
    pub arguments: Option<&'a str>
}

#[derive(Debug, Clone)]
pub struct CurlyBraceMode<'a> {
    pub header: Option<HeaderMode<'a>>,
    pub start_index: usize,
    pub children: LinkedList<Node<'a>>,
}

#[derive(Debug, Clone)]
pub struct SquareBracketMode<'a> {
    pub name_start_index: usize,
    pub name: &'a str,
    pub start_index: usize,
}

#[derive(Debug, Clone)]
pub struct TextMode {
    pub start_index: usize,
}

///////////////////////////////////////////////////////////////////////////////
// STACK
///////////////////////////////////////////////////////////////////////////////

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
    fn filter(&mut self, f: impl Fn(&State<'a>) -> bool) -> VecDeque<State> {
        let mut indexes = VecDeque::<usize>::new();
        for ix in 0..self.0.len() {
            if f(self.0.get(ix).unwrap()) {
                indexes.push_front(ix);
            }
        }
        let mut entries = VecDeque::<State>::new();
        for ix in indexes {
            let state = self.0.remove(ix).unwrap();
            assert!(f(&state));
            entries.push_back(state);
        }
        return entries
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


///////////////////////////////////////////////////////////////////////////////
// PARSER STATE MACHINE
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
enum Cmd {
    SkipTo(usize),
    NoOp,
}

#[derive(Debug, Clone)]
pub enum Error {
    Syntax,
}

mod helpers {
    use super::*;

    pub fn start_text_mode<'a, 'b>(
        source: &'a str,
        mut ix: usize,
        stack: &'b mut Stack<'a>,
    ) -> Result<(), ()> {
        if (&source[ix..]).trim().is_empty() {
            return Ok(())
        }
        if let Some(text_node) = stack.pop_text_mode() {
            stack.push(State::TextMode(text_node));
        } else {
            stack.push(State::TextMode(TextMode {
                start_index: ix,
            }));
        }
        Ok(())
    }

    pub fn add_to_parent<'a, 'b>(
        node: Node<'a>,
        stack: &'b mut Stack<'a>,
        top_level: &'b mut LinkedList<Node<'a>>,
    ) {
        if let Some(mut parent) = stack.pop_curly_brace_mode() {
            parent.children.push_back(node);
            stack.push(State::CurlyBraceMode(parent));
        } else {
            top_level.push_back(node);
        }
    }
    // Call this before init a new `NameNode` or text node..
    pub fn cleanup_stack_errors<'a, 'b>(
        source: &'a str,
        ix: usize,
        ch: char,
        stack: &'b mut Stack<'a>,
        top_level: &'b mut LinkedList<Node<'a>>,
    ) {
        assert!(stack.pop_square_bracket_mode().is_none());
        // Unclosed square parens node.
        if let Some(header_mode) = stack.pop_header_mode() {
            if ch != '\\' {return;}
            let start = header_mode.name_start_index;
            let node = CurlyBraceMode{
                header: Some(header_mode),
                start_index: ix,
                children: Default::default(),
            };
            let node = Node::Call {
                start,
                end: ix,
                call: node,
            };
            add_to_parent(node, stack, top_level);
        }
        // Unclosed name node.
        if let Some(name_node) = stack.pop_name_mode() {
            if ch != '\\' {return;}
            let name = &source[name_node.start_index..ix];
            let node = Node::Call {
                start: name_node.start_index,
                end: ix,
                call: CurlyBraceMode {
                    header: Some(HeaderMode {
                        name_start_index: ix,
                        name,
                        arguments: None,
                    }),
                    start_index: name_node.start_index,
                    children: Default::default(),
                }
            };
            add_to_parent(node, stack, top_level);
        }
    }
    pub fn end_text_mode<'a, 'b>(
        source: &'a str,
        ix: usize,
        stack: &'b mut Stack<'a>,
        top_level: &'b mut LinkedList<Node<'a>>,
    ) {
        if let Some(text_mode) = stack.pop_text_mode() {
            let string = &source[text_mode.start_index..ix];
            if string.is_empty() {return}
            let node = Node::Text{
                start: text_mode.start_index,
                end: ix,
                string,
            };
            if let Some(mut parent) = stack.pop_curly_brace_mode() {
                parent.children.push_back(node);
                stack.push(State::CurlyBraceMode(parent));
            } else {
                top_level.push_back(node);
            }
        }
        for txt in stack.filter(|x| x.is_text_mode()) {
            let txt = txt.unwrap_text_mode().unwrap();
            println!("TXT: {:?}", txt);
        }
    }
    pub fn end_name_mode_if_present<'a, 'b>(
        source: &'a str,
        ix: usize,
        stack: &'b mut Stack<'a>,
        top_level: &'b mut LinkedList<Node<'a>>,
    ) {
        if let Some(NameMode{start_index, arguments})  = stack.pop_name_mode() {
            let name = &source[start_index..ix];
            let call = CurlyBraceMode {
                header: Some(HeaderMode {
                    name_start_index: start_index,
                    name,
                    arguments,
                }),
                start_index: start_index,
                children: Default::default(),
            };
            let node = Node::Call {
                start: start_index,
                end: ix,
                call,
            };
            helpers::add_to_parent(node, stack, top_level);
        }
    }
    pub fn is_in_name_mode<'a, 'b>(stack: &'b mut Stack<'a>) -> bool {
        if let Some(name_mode)  = stack.pop_name_mode() {
            stack.push(State::NameMode(name_mode));
            return true
        } else {
            false
        }
    }
}

fn tick<'a, 'b>(
    source: &'a str,
    ix: usize,
    ch: char,
    stack: &'b mut Stack<'a>,
    top_level: &'b mut LinkedList<Node<'a>>,
    line: &mut usize,
    column: &mut usize,
) -> Result<Cmd, Error> {
    *column = *column + 1;
    let next_char = source.get(ix + 1..=ix+1);
    match ch {
        '_' if !helpers::is_in_name_mode(stack) => {
            helpers::end_text_mode(source, ix, stack, top_level);
            let node = Node::Token(Token(
                &source[ix..=ix]
            ));
            helpers::add_to_parent(node, stack, top_level);
            helpers::start_text_mode(source, ix + 1, stack);
        }
        '^' => {
            helpers::end_name_mode_if_present(source, ix, stack, top_level);
            helpers::end_text_mode(source, ix, stack, top_level);
            let node = Node::Token(Token(
                &source[ix..=ix]
            ));
            helpers::add_to_parent(node, stack, top_level);
            helpers::start_text_mode(source, ix + 1, stack);
        }
        '=' if next_char == Some(">") => {
            helpers::end_name_mode_if_present(source, ix, stack, top_level);
            helpers::end_text_mode(source, ix, stack, top_level);
            let node = Node::Token(Token(
                &source[ix..=ix + 1]
            ));
            helpers::add_to_parent(node, stack, top_level);
            helpers::start_text_mode(source, ix + 2, stack);
        }
        ' ' if helpers::is_in_name_mode(stack) => {
            helpers::end_name_mode_if_present(source, ix, stack, top_level);
            helpers::start_text_mode(source, ix, stack);
        }
        '\\' if next_char != Some("\\") => {
            helpers::end_name_mode_if_present(source, ix, stack, top_level);
            helpers::end_text_mode(source, ix, stack, top_level);
            // CHECKS
            helpers::cleanup_stack_errors(
                source,
                ix,
                ch,
                stack,
                top_level,
            );
            if source.len() >= ix + 1 {
                let rest_of_line = (&source[ix..]);
                let mut name_pos = None;
                for (pos,ch) in rest_of_line.chars().enumerate() {
                    let end_of_name = {
                        ch.is_whitespace() ||
                        ch == '{' ||
                        ch == '}' ||
                        ch == '[' ||
                        ch == ']' ||
                        (ch == '\\' && pos > 0)
                    };
                    if end_of_name {
                        name_pos = Some(pos);
                        break;
                    }
                }
                let name_pos = name_pos.unwrap();
                let name = &rest_of_line[0..name_pos];
                assert!(!name.trim().is_empty());
                let rest_of_line = (&source[ix + name_pos..]).lines().next().unwrap();
                let empty_line = rest_of_line
                    .replace("[", "")
                    .replace("]", "")
                    .trim()
                    .is_empty();
                let end_of_line = ix + name_pos;
                if empty_line {
                    let node = Node::Call {
                        start: ix,
                        end: end_of_line,
                        call: CurlyBraceMode {
                            header: Some(HeaderMode {
                                name_start_index: ix,
                                name,
                                arguments: None,
                            }),
                            start_index: ix,
                            children: Default::default(),
                        }
                    };
                    helpers::add_to_parent(node, stack, top_level);
                    // helpers::start_text_mode(source, end_of_line + 1, stack);
                    return Ok(Cmd::SkipTo(end_of_line));
                }
            }
            // DONE
            stack.push(State::NameMode(NameMode{
                start_index: ix,
                arguments: None,
            }));
        }
        '[' => {
            helpers::end_text_mode(source, ix, stack, top_level);
            let NameMode{start_index, arguments} = stack.pop_name_mode().unwrap();
            let name = &source[start_index..ix];
            stack.push(State::SquareBracketMode(SquareBracketMode{
                name_start_index: start_index,
                name,
                start_index: ix,
            }));
        }
        ']' => {
            helpers::end_text_mode(source, ix, stack, top_level);
            let SquareBracketMode{name_start_index, name, start_index} = stack
                .pop_square_bracket_mode()
                .unwrap();
            let arguments = &source[start_index..=ix];
            stack.push(State::HeaderMode(HeaderMode{
                name_start_index,
                name,
                arguments: Some(arguments),
            }));
        }
        '{' => {
            helpers::end_text_mode(source, ix, stack, top_level);
            assert!(stack.pop_square_bracket_mode().is_none());
            if let Some(header) = stack.pop_header_mode() {
                stack.push(State::CurlyBraceMode(CurlyBraceMode{
                    header: Some(header),
                    start_index: ix,
                    children: Default::default(),
                }));
            } else if let Some(NameMode{start_index, arguments}) = stack.pop_name_mode() {
                let name = &source[start_index..ix];
                stack.push(State::CurlyBraceMode(CurlyBraceMode{
                    header: Some(HeaderMode {
                        name_start_index: start_index,
                        name,
                        arguments: None
                    }),
                    start_index: ix,
                    children: Default::default(),
                }));
            } else {
                // ANONYMOUS CASE
                stack.push(State::CurlyBraceMode(CurlyBraceMode{
                    header: None,
                    start_index: ix,
                    children: Default::default(),
                }));
            }
            helpers::start_text_mode(source, ix + 1, stack);
        }
        '}' => {
            helpers::end_text_mode(source, ix, stack, top_level);
            let node = stack
                .pop_curly_brace_mode()
                .ok_or(Error::Syntax)?;
            // CASE - DEFAULT
            let node = Node::Call {
                start: node.header.as_ref().map(|x| x.name_start_index).unwrap_or(ix),
                end: ix,
                call: node,
            };
            helpers::add_to_parent(node, stack, top_level);
            helpers::start_text_mode(source, ix + 1, stack);
        }
        '\n' => {
            helpers::end_text_mode(source, ix, stack, top_level);
            *column = 0;
            *line = (*line) + 1;
            helpers::cleanup_stack_errors(
                source,
                ix,
                ch,
                stack,
                top_level,
            );
            if helpers::start_text_mode(source, ix + 1, stack).is_err() {
                helpers::end_text_mode(source, ix, stack, top_level);
            }
        }
        ch => {},
    }
    Ok(Cmd::NoOp)
}


///////////////////////////////////////////////////////////////////////////////
// RUN PARSER
///////////////////////////////////////////////////////////////////////////////

fn run_internal_parser<'a>(source: &'a str) -> LinkedList<Node<'a>> {
    let mut stack = Stack::default();
    let mut top_level: LinkedList<Node> = Default::default();
    let mut skip_to: Option<usize> = None;
    let mut line: usize = 0;
    let mut column: usize = 0;
    helpers::start_text_mode(source, 0, &mut stack);
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
            &mut line,
            &mut column,
        );
        match res {
            Err(Error::Syntax) => {
                helpers::start_text_mode(source, ix, &mut stack);
            }
            Ok(Cmd::SkipTo(start_from)) => {
                assert!(start_from > ix);
                skip_to = Some(start_from);
            }
            Ok(Cmd::NoOp) => ()
        }
    }
    top_level
}

fn parse_rewrite_rules<'a>(node: ast::Ast<'a>) -> Option<Vec<(ast::Ast<'a>, ast::Ast<'a>)>> {
    assert!(node.is_name("!where"));
    let mut res = Vec::new();
    let block: ast::Block = node.into_block()?;
    for child in block.children {
        println!("rewrite_rule {:?}", child);
    }
    Some(res)
}

fn normalize_children<'a>(xs: Vec<Ast<'a>>) -> Vec<Ast<'a>> {
    let mut children: LinkedList<ast::Ast> = LinkedList::default();
    for child in xs {
        if child.is_name("!where") {
            println!("YES");
            if let Some(mut last_node) = children.pop_back() {
                if let Some(mut last_node) = last_node.unpack_block_mut() {
                    if let Some(rewrite_rules) = parse_rewrite_rules(child) {
                        last_node.rewrite_rules.extend(rewrite_rules);
                    }
                }
                children.push_back(last_node);
                continue;
            }
        }
        children.push_back(child);
    }
    let children = children
        .into_iter()
        .collect::<Vec<_>>();
    children
}

fn transform<'a>(node: Node<'a>) -> ast::Ast<'a> {
    match node {
        Node::Call{start, end, mut call} => {
            let children = call.children
                .into_iter()
                .map(|child| transform(child))
                .collect();
            let children = normalize_children(children);
            let rewrite_rules = Vec::new();
            match call.header {
                Some(HeaderMode { name_start_index, name, arguments }) => {
                    ast::Ast::Block(ast::Block {
                        name: Some(name),
                        parameters: arguments,
                        children,
                        rewrite_rules,
                    })
                }
                None => {
                    ast::Ast::Block(ast::Block {
                        name: None,
                        parameters: None,
                        children,
                        rewrite_rules,
                    })
                }
            }
        }
        Node::Token(token) => {
            ast::Ast::Token(token.0)
        }
        Node::Text{start, end, string} => {
            ast::Ast::Content(ast::Content {
                string,
            })
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn run() {
    let source = include_str!("../source.txt");
    let result = run_internal_parser(source);
    let result = result
        .into_iter()
        .map(|x| transform(x))
        .collect::<Vec<_>>();
    let result = normalize_children(result);
    for node in result {
        // println!("{:#?}", node);
    }
}