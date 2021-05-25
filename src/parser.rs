use std::ops::Deref;
use std::borrow::Cow;
use either::Either;


///////////////////////////////////////////////////////////////////////////////
// PARSER
///////////////////////////////////////////////////////////////////////////////


fn get_next_newline_indent(source: &str) -> Option<usize> {
    let mut iter = source.lines();
    let _ = iter.next();
    iter.filter(|source| !source.trim().is_empty())
        .find_map(|source| {
            println!("> {}", source);
            let ix = source.find(|c: char| !c.is_whitespace())?;
            Some(ix)
        })
}

fn get_indent_level(source: &str) -> Option<usize> {
    let mut counter = 0;
    for ch in source.chars() {
        if ch != ' ' {
            return Some(counter);
        }
        counter = counter + 1;
    }
    None
}

#[derive(Debug, Clone)]
pub struct Position {
    /// The global offset of this node.
    offset: usize,
    line: usize,
    column: usize,
    parent_column: usize,
}

#[derive(Debug, Clone)]
pub struct Text<'a> {
    // offset: usize,
    position: Position,
    // block_level: usize,
    string: &'a str,
}

impl<'a> Text<'a> {
    pub fn offset(&self) -> usize {
        self.position.offset
    }
    pub fn parent_column(&self) -> usize {
        self.position.parent_column
    }
    pub fn current_column(&self) -> usize {
        self.position.column
    }
}

#[derive(Debug, Clone)]
pub struct Ann<T> {
    global_start: usize,
    global_end: usize,
    data: T
}

#[derive(Debug, Clone)]
pub struct Output<'a, T> {
    current: T,
    forward: Text<'a>,
}

macro_rules! consume_chars_until {
    ($out_ty:ty, $text:expr, {
        valid: [$($valid_pat:pat),* $(,)*],
        invalid: [$($invalid_pat:pat => $error_value:expr),* $(,)*],
    }) => {{
        let mut char_counter: usize = 0;
        let finalize = |char_counter| {
            let current = Text {
                position: Position {
                    offset: $text.position.offset,
                    line: $text.position.line,
                    column: $text.position.column,
                    parent_column: $text.position.parent_column,
                },
                string: &$text.string[0..char_counter],
            };
            let forward = Text {
                position: Position {
                    offset: $text.position.offset + char_counter,
                    line: $text.position.line,
                    column: $text.position.column + char_counter,
                    parent_column: $text.position.parent_column,
                },
                string: &$text.string[char_counter..]
            };
            Output{
                current,
                forward,
            }
        };
        let mut result: Option<$out_ty> = None;
        for ch in $text.string.chars() {
            char_counter = char_counter + 1;
            match ch {
                $(
                    $invalid_pat => {
                        if result.is_none() {
                            result = Some(Err($error_value))
                        }
                    },
                )*
                $(
                    $valid_pat => {
                        if result.is_none() {
                            result = Some(Ok(finalize(char_counter - 1)))
                        }
                    },
                )*
                _ => ()
            }
        }
        match result {
            Some(x) => x,
            _ => {
                unimplemented!("at end of file?")
            }
        }
    }};
    ($out_ty:ty, $text:expr, [$($left:pat),* $(,)*]) => {{
        let mut char_counter: usize = 0;
        let finalize = |char_counter| {
            let current = Text {
                position: Position {
                    offset: $text.position.offset,
                    line: $text.position.line,
                    column: $text.position.column,
                    parent_column: $text.position.parent_column,
                },
                string: &$text.string[0..char_counter],
            };
            let forward = Text {
                position: Position {
                    offset: $text.position.offset + char_counter,
                    line: $text.position.line,
                    column: $text.position.column + char_counter,
                    parent_column: $text.position.parent_column,
                },
                string: &$text.string[char_counter..]
            };
            Output{current, forward}
        };
        let mut result: Option<$out_ty> = None;
        for ch in $text.string.chars() {
            char_counter = char_counter + 1;
            match ch {
                $(
                    $left => {
                        if result.is_none() {
                            result = Some(Ok(finalize(char_counter - 1)));
                        }
                    },
                )*
                _ => ()
            }
        }
        match result {
            Some(x) => x,
            _ => {
                return Err(())
            }
        }
    }};
}

pub mod text {
    use super::*;

    pub fn character<'a>(text: Text<'a>, ch: char) -> Result<Output<'a, ()>, ()> {
        match text.string.chars().next() {
            Some(x) if x == ch => {
                Ok(Output {
                    current: (),
                    forward: Text {
                        position: Position {
                            offset: text.position.offset + 1,
                            line: text.position.line,
                            column: text.position.column + 1,
                            parent_column: text.position.parent_column,
                        },
                        string: &text.string[1..]
                    },
                })
            }
            _ => {
                Err(())
            }
        }
    }
    pub fn tag<'a>(text: Text<'a>, tag: &'a str) -> Result<Output<'a, ()>, ()> {
        if text.string.starts_with(tag) {
            let current = ();
            let forward = Text{
                // offset: text.offset + tag.len(),
                // block_level: text.block_level,
                position: Position {
                    offset: text.position.offset + tag.len(),
                    line: text.position.line,
                    column: text.position.column + tag.len(),
                    parent_column: text.position.parent_column,
                },
                string: &text.string[tag.len()..]
            };
            Ok(Output{
                current,
                forward
            })
        } else {
            Err(())
        }
    }
    pub fn forward<'a>(text: Text<'a>) -> Result<Output<'a, Text<'a>>, ()> {
        let result = consume_chars_until!(
            Result<Output<'a, Text<'a>>, ()>,
            text,
            ['\\', '\n']
        );
        result
    }
    pub fn identifier<'a>(text: Text<'a>) -> Result<Output<'a, Text<'a>>, ()> {
        consume_chars_until!(Result<Output<'a, Text<'a>>, ()>, text, [' ', '[', '{', '\n'])
    }
    pub fn parens<'a>(text: Text<'a>) -> Result<Output<'a, Text<'a>>, ()> {
        if let Ok(Output{current: _, forward: value}) = character(text, '[') {
            consume_chars_until!(Result<Output<'a, Text<'a>>, ()>, value, {
                valid: [
                    ']'
                ],
                invalid: [
                    '\n' => ()
                ],
            })
        } else {
            Err(())
        }
    }

    pub fn optional<'a, U>(
        text: Text<'a>,
        value: Result<Output<'a, U>, ()>,
    ) -> Output<'a, Option<U>> {
        match value {
            Ok(output) => {
                let current = Some(output.current);
                let forward = output.forward;
                Output {current, forward}
            }
            Err(_) => {
                Output {current: None, forward: text}
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
// AST
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct FunCall<'a> {
    name: Ann<&'a str>,
    body: Option<Ann<Vec<Ast<'a>>>>,
}

#[derive(Debug)]
pub enum Ast<'a> {
    Text(Ann<&'a str>),
    Call(Ann<FunCall<'a>>),
}


///////////////////////////////////////////////////////////////////////////////
// SUB-PARSERS
///////////////////////////////////////////////////////////////////////////////



fn parse_function_call<'a>(text: Text<'a>) -> Result<Output<'a, FunCall<'a>>, ()> {
    let starting_offset = text.offset();
    let parent_column_level = text.parent_column();
    let current_column_level = text.current_column();
    let next_newline_indent_level = get_next_newline_indent(&text.string)
        .unwrap_or_else(|| {
            parent_column_level
        });
    // CHECKS
    let mut is_block_node = {
        next_newline_indent_level > parent_column_level &&
        current_column_level == parent_column_level
    };
    let is_inline_node = {
        current_column_level > parent_column_level
    };
    if !is_inline_node && !is_block_node {
        is_block_node = true
    }
    assert!((!is_inline_node && !is_block_node) == false);
    // GO!
    if let Ok(output) = text::character(text, '\\') {
        if let Ok(Output{current: fun_name, forward}) = text::identifier(output.forward) {
            let global_start = fun_name.position.offset;
            let fun_name = Ann {
                global_start: fun_name.position.offset + 1,
                global_end: fun_name.position.offset + fun_name.string.len(),
                data: fun_name.string,
            };
            let mut is_brace_node = false;
            let mut is_line_node = false;
            let chars_until_newline: usize;
            {
                let mut char_counter = 0;
                let mut ignore_brace = false;
                for ch in forward.string.chars() {
                    match ch {
                        '{' => {
                            if !ignore_brace {
                                is_brace_node = true;
                            }
                        }
                        '\\' => {
                            ignore_brace = true;
                        }
                        '\n' => {break}
                        _ => ()
                    }
                    char_counter = char_counter + 1;
                }
                if char_counter > 0 {
                    is_line_node = true;
                }
                chars_until_newline = char_counter;
            };
            if is_brace_node {
                // TREAT AS INLINE NODE WITH BODY
                unimplemented!("DONT HIT THIS YET");
                let result = consume_chars_until!(
                    Result<Output<'a, Text<'a>>, ()>,
                    output.forward.clone(),
                    {
                        valid: ['}'],
                        invalid: [
                            '\n' => ()
                        ],
                    }
                )?;
                let body = parse_to_ast(result.current).unwrap();
                let current = FunCall {
                    name: fun_name,
                    body: Some(Ann {
                        global_start: unimplemented!(),
                        global_end: unimplemented!(),
                        data: body.current
                    }),
                };
                return Ok(Output {
                    current,
                    forward,
                });
            }
            else if is_line_node {
                let rest_of_line: &str = &forward.string[0..chars_until_newline];
                let line_text = Text {
                    position: Position {
                        offset: forward.position.offset,
                        line: forward.position.line,
                        column: forward.position.column,
                        parent_column: forward.position.parent_column,
                    },
                    string: rest_of_line,
                };
                let line_ast = parse_to_ast(line_text).unwrap();
                if !line_ast.forward.string.is_empty() {
                    println!("!line_ast.forward.string.is_empty(): ");
                    println!("{:?}", line_ast.forward.string);
                    println!("PARSED");
                    println!("{:?}", line_ast);
                }
                assert!(line_ast.forward.string.is_empty());
                let fun_call = FunCall {
                    name: fun_name,
                    // arguments: args,
                    body: Some(Ann {
                        global_start: 0,
                        global_end: 0,
                        data: line_ast.current,
                    })
                };
                // REST OF THE LINE
                let following_content = Text {
                    position: Position {
                        offset: forward.position.offset + chars_until_newline,
                        line: forward.position.line + 1,
                        column: 0,
                        parent_column: parent_column_level,
                    },
                    string: &forward.string[chars_until_newline..]
                };
                // DONE
                return Ok(Output {
                    current: fun_call,
                    forward: following_content
                });
            } else if is_inline_node {
                // INLINE NODE WITH NO BODY
                unimplemented!("DONT HIT THIS YET (NO INLINE NODE)");
                assert!(!is_block_node);
                let ending_offset = fun_name.global_end;
                let current = FunCall {
                    name: fun_name,
                    body: None,
                };
                return Ok(Output {
                    current,
                    forward,
                });
            } else if is_block_node {
                // BLOCK NODE WITH POSSIBLE INDENTED BODY
                let mut char_counter: usize = 0;
                let mut line_counter: usize = 0;
                // FIND LENGTH OF BLOCK NODE
                for line in forward.string.lines() {
                    // SKIP EMPTY LINES
                    if line.trim().is_empty() {
                        char_counter = char_counter + line.len();
                        line_counter = line_counter + 1;
                        // ADD ONE TO COUNTER FOR NEWLINE CHAR:
                        char_counter = char_counter + 1;
                        continue;
                    }
                    // CHECK VALID BLOCK
                    let valid_line_lvl = get_indent_level(line)
                        .map(|line_lvl| {line_lvl > parent_column_level})
                        .unwrap();
                    if valid_line_lvl {
                        char_counter = char_counter + line.len();
                        line_counter = line_counter + 1;
                        // ADD ONE TO COUNTER FOR NEWLINE CHAR:
                        char_counter = char_counter + 1;
                    } else {
                        break;
                    }
                }
                // NEW STRINGS BASED ON TOP LENGTH
                let block_string = &forward.string[0..char_counter];
                let rest_string = &forward.string[char_counter ..];
                // NEW TEXT OBJECTS
                let block_text = Text {
                    // offset: forward.offset,
                    // block_level: parent_column_level + next_newline_indent_level,
                    position: Position {
                        offset: forward.offset(),
                        line: forward.position.line,
                        column: 0,
                        parent_column: parent_column_level + next_newline_indent_level,
                    },
                    string: block_string,
                };
                let rest_text = Text {
                    // offset: forward.offset + char_counter,
                    // block_level: parent_column_level,
                    position: Position {
                        offset: forward.offset() + char_counter,
                        line: forward.position.line + line_counter,
                        column: 0,
                        parent_column: parent_column_level,
                    },
                    string: rest_string,
                };
                // PARSE BODY
                let block_ast = parse_to_ast(block_text).unwrap();
                assert!(block_ast.forward.string.is_empty());
                // FUNCTION CALL
                let fun_call = FunCall {
                    name: fun_name,
                    // arguments: args,
                    body: Some(Ann {
                        global_start: 0,
                        global_end: 0,
                        data: block_ast.current,
                    })
                };
                // REST OF THE UNPARSED/UNCONSUMED AST
                return Ok(Output {
                    current: fun_call,
                    forward: rest_text
                });
            } else {
                // WHAT TO DO?
                unimplemented!()
            }
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

fn parse_arbitrary_content<'a>(text: Text<'a>) -> Result<Output<'a, Ast<'a>>, Either<(), ()>> {
    let starting_offset = text.offset();
    let Output{current: text, forward} = text::forward(text)
        .map_err(|_| Either::Left(()))?;
    if text.string.is_empty() {
        return Err(Either::Right(()));
    }
    Ok(Output {
        current: Ast::Text(Ann {
            global_start: starting_offset,
            global_end: forward.offset(),
            data: text.string,
        }),
        forward
    })
}

///////////////////////////////////////////////////////////////////////////////
// ROOT PARSER
///////////////////////////////////////////////////////////////////////////////

fn parse_to_ast<'a>(text: Text<'a>) -> Result<Output<'a, Vec<Ast<'a>>>, ()> {
    let root_starting_pos = text.position.clone();
    let mut root_ending_pos = text.position.clone();
    let mut current_text_is_empty = false;
    let mut current_text: Text<'a> = text;
    let mut ast_nodes: Vec<Ast<'a>> = Vec::new();
    loop {
        let new_starting_pos = current_text.position.clone();
        // if current_text.string.is_empty() {
        //     println!("| EMPTY: {:?}", current_text);
        //     break
        // }
        if let Ok(output) = text::character(current_text.clone(), '\n') {
            let Output{current: ast, forward} = output;
            println!("| NEWLINE: {:?}", current_text);
            // assert!(current_text.string.trim().is_empty());
            let new_column = get_indent_level(forward.string)
                .unwrap_or_else(|| {
                    0
                });
            let new_position = Position {
                offset: forward.position.offset + 1,
                line: forward.position.line + 1,
                column: new_column,
                parent_column: forward.position.parent_column,
            };
            root_ending_pos = new_position;
            current_text = forward;
            ast_nodes.push(Ast::Text(
                Ann {
                    global_start: new_starting_pos.offset,
                    global_end: new_starting_pos.offset + 1,
                    data: "\n",
                }
            ));
            // if current_text.string.is_empty() {
            //     break
            // }
        } else {
            let result = parse_arbitrary_content(current_text.clone());
            println!("| result: {:?}", result);
            println!("| result: {:?}", current_text);

            if let Ok(output) = result {
                let Output{current: ast, forward} = output;
                root_ending_pos = forward.position.clone();
                current_text = forward;
                ast_nodes.push(ast);
            } else if let Ok(output) = parse_function_call(current_text.clone()) {
                println!("| FUN CALL: {:?}", current_text);
                let Output{current: ast, forward} = output;
                root_ending_pos = forward.position.clone();
                current_text = forward;
                let ast = Ast::Call(Ann {
                    global_start: new_starting_pos.offset,
                    global_end: root_ending_pos.offset,
                    data: ast
                });
                ast_nodes.push(ast);
                if current_text.string.is_empty() {
                    break;
                }
            } else if let Err(Either::Right(())) = result {
                current_text_is_empty = true;
                // return Ok(Output{
                //     current: ast_nodes,
                //     forward: Text {
                //         position: current_text.position.clone(),
                //         string: ""
                //     },
                // });
                break;
            } else {
                break
            }
        }
    }
    Ok(Output{
        current: ast_nodes,
        forward: {
            if current_text_is_empty {
                Text {
                    position: current_text.position.clone(),
                    string: ""
                }
            } else {
                current_text
            }
        },
    })
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub fn run_parser<'a>(source: &'a str) -> Result<Vec<Ast<'a>>, ()> {
    let text = Text{
        position: Position {
            offset: 0,
            line: 0,
            column: 0,
            parent_column: 0,
        },
        string: include_str!("../source.txt"),
    };
    let result = parse_to_ast(text).unwrap();
    if !result.forward.string.is_empty() {
        println!("UNCONSUMED:");
        println!("{}", result.forward.string);
    }
    assert!(result.forward.string.is_empty());
    Ok(result.current)
}

pub fn run() {
    let source = include_str!("../source.txt");
    let result = run_parser(source);
    println!("result: {:#?}", result);
}

