pub fn get_next_newline_indent(source: &str) -> Option<usize> {
    let mut iter = source.lines();
    let _ = iter.next();
    iter.filter(|source| !source.trim().is_empty())
        .find_map(|source| {
            let ix = source.find(|c: char| !c.is_whitespace())?;
            Some(ix)
        })
}

pub fn get_next_newline_indent_with_line(source: &str) -> Option<(usize, &str)> {
    let mut iter = source.lines();
    let _ = iter.next();
    iter.filter(|source| !source.trim().is_empty())
        .find_map(|source| {
            let ix = source.find(|c: char| !c.is_whitespace())?;
            Some((ix, source))
        })
}

pub fn get_indent_level(source: &str) -> Option<usize> {
    let mut counter = 0;
    for ch in source.chars() {
        if !ch.is_whitespace() {
            return Some(counter);
        }
        counter = counter + 1;
    }
    None
}
