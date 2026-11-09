//! Turns a [`LayoutDocument`] into text.
//!
//! The printer is language independent. It:
//! 1. keeps the line breaks from the source (at most one blank line in a row),
//! 2. computes the indentation of every line,
//! 3. wraps lines that are longer than `column_width` at bracket groups,
//! 4. decides the spaces between tokens,
//! 5. aligns trailing comments.
//!
//! The output always uses `\n` line endings. The engine converts them afterwards.

use crate::layout::{LayoutDocument, LayoutToken, MultiLine, Spacing};

/// Printer settings, taken from the configuration.
#[derive(Clone, Debug)]
pub struct PrintOptions {
    pub indent_size: usize,
    pub use_tabs: bool,
    pub column_width: usize,
    pub align_comments: bool,
    /// Whether long lines may be wrapped.
    pub wrap_lines: bool,
}

/// The smallest gap between code and an aligned trailing comment.
const ALIGNED_COMMENT_GAP: usize = 2;

/// One output line: a run of tokens and its indentation level.
#[derive(Clone, Debug)]
struct Line {
    first: usize,
    last: usize,
    /// Indentation level of this line.
    indent: usize,
    /// Indentation level of the innermost open bracket group at the start of this line.
    base: usize,
    /// Empty lines to print in front of this line.
    blank_lines_before: usize,
}

/// Prints the document. Returns the text with `\n` line endings.
pub fn print(document: &LayoutDocument, options: &PrintOptions) -> String {
    if document.tokens.is_empty() {
        return String::new();
    }
    let mut breaks = initial_line_breaks(document);
    if options.wrap_lines {
        wrap_long_lines(document, &mut breaks, options);
    }
    let lines = build_lines(document, &breaks);
    render(document, &lines, options)
}

/// Line breaks before each token: the source line breaks, with at most one blank line in
/// a row, and no blank lines right after an opening bracket or before a closing one.
fn initial_line_breaks(document: &LayoutDocument) -> Vec<usize> {
    let tokens = &document.tokens;
    let mut breaks = Vec::with_capacity(tokens.len());
    for index in 0..tokens.len() {
        let mut count = tokens[index].line_breaks_before.min(2);
        if index == 0 {
            count = 0;
        } else if count > 1 && (tokens[index - 1].opens_group.is_some() || tokens[index].closes_group.is_some()) {
            count = 1;
        }
        breaks.push(count);
    }
    breaks
}

/// Splits the tokens into lines and computes the indentation of every line.
fn build_lines(document: &LayoutDocument, breaks: &[usize]) -> Vec<Line> {
    let tokens = &document.tokens;
    let mut lines: Vec<Line> = Vec::new();
    for index in 0..tokens.len() {
        if index == 0 || breaks[index] > 0 {
            let blank_lines_before = if index == 0 { 0 } else { breaks[index] - 1 };
            lines.push(Line {
                first: index,
                last: index,
                indent: 0,
                base: 0,
                blank_lines_before,
            });
        } else if let Some(line) = lines.last_mut() {
            line.last = index;
        }
    }

    let mut line_of_token = vec![0; tokens.len()];
    for (line_index, line) in lines.iter().enumerate() {
        for token in line.first..=line.last {
            line_of_token[token] = line_index;
        }
    }

    // Each open bracket group remembers the indentation level that was current where it
    // was opened. Its contents are indented one level deeper, and its closing bracket goes
    // back to that level. After a closing bracket, the rest of the line continues at the
    // level of that bracket's group, so in
    //     fn compute(a: i32,
    //         b: i32) -> i32 {
    // the `{` belongs to the `fn` level, not to the parameter level.
    let mut open_group_levels: Vec<usize> = Vec::new();
    for line_index in 0..lines.len() {
        let first = lines[line_index].first;
        let last = lines[line_index].last;
        let token = &tokens[first];

        let base = match open_group_levels.last() {
            Some(&level) => level + 1,
            None => 0,
        };
        let indent = if token.column_zero {
            0
        } else if token.closes_group.is_some() {
            open_group_levels.last().copied().unwrap_or(0)
        } else {
            base + continuation_levels(token, line_index, &lines, &line_of_token)
        };
        lines[line_index].base = base;
        lines[line_index].indent = indent;

        let mut current_level = indent;
        for index in first..=last {
            if tokens[index].closes_group.is_some() {
                if let Some(level) = open_group_levels.pop() {
                    current_level = level;
                }
            }
            if tokens[index].opens_group.is_some() {
                open_group_levels.push(current_level);
            }
        }
    }

    indent_comment_lines(document, &mut lines);
    lines
}

/// Counts the extra indentation for a token that starts a line in the middle of a
/// statement or expression: one level for every earlier line that begins with an
/// enclosing syntax node.
///
/// Only nodes that begin a line count. In `} else if x {` the inner `if` starts in the
/// middle of the line, so it does not push the following `else` further right.
fn continuation_levels(token: &LayoutToken, line_index: usize, lines: &[Line], line_of_token: &[usize]) -> usize {
    let mut earlier_lines: Vec<usize> = Vec::new();
    for &anchor in &token.anchors {
        let anchor_line = line_of_token[anchor];
        let starts_line = lines[anchor_line].first == anchor;
        if anchor_line < line_index && starts_line && !earlier_lines.contains(&anchor_line) {
            earlier_lines.push(anchor_line);
        }
    }
    let mut levels = earlier_lines.len();
    if token.outdent && levels > 0 {
        levels -= 1;
    }
    levels
}

/// A line that starts with a comment is indented like the next line of code, because a
/// comment usually describes what follows it. A comment right before a closing bracket
/// is indented like the contents of that bracket.
fn indent_comment_lines(document: &LayoutDocument, lines: &mut [Line]) {
    let tokens = &document.tokens;
    for line_index in 0..lines.len() {
        let first = &tokens[lines[line_index].first];
        if !first.is_comment || first.column_zero {
            continue;
        }
        let mut next_code_line = None;
        for candidate in line_index + 1..lines.len() {
            let candidate_first = &tokens[lines[candidate].first];
            if !candidate_first.is_comment && !candidate_first.column_zero {
                next_code_line = Some(candidate);
                break;
            }
        }
        lines[line_index].indent = match next_code_line {
            Some(next) => {
                if tokens[lines[next].first].closes_group.is_some() {
                    lines[next].base
                } else {
                    lines[next].indent
                }
            }
            None => lines[line_index].base,
        };
    }
}

/// Adds line breaks inside bracket groups until no line is longer than the column width,
/// or until nothing more can be wrapped.
fn wrap_long_lines(document: &LayoutDocument, breaks: &mut [usize], options: &PrintOptions) {
    let mut wrapped = vec![false; document.groups.len()];
    loop {
        let lines = build_lines(document, breaks);
        let mut changed = false;
        for line in &lines {
            let width = match code_width(document, line, options) {
                Some(width) => width,
                None => continue,
            };
            if width <= options.column_width {
                continue;
            }
            if let Some(group) = choose_group_to_wrap(document, line, &wrapped) {
                wrap_group(document, group, breaks);
                wrapped[group] = true;
                changed = true;
                break;
            }
        }
        if !changed {
            return;
        }
    }
}

/// Picks the outermost wrappable group that opens on this line.
fn choose_group_to_wrap(document: &LayoutDocument, line: &Line, wrapped: &[bool]) -> Option<usize> {
    let mut depth: usize = 0;
    let mut best: Option<(usize, usize)> = None; // (depth, group)
    for index in line.first..=line.last {
        let token = &document.tokens[index];
        if token.closes_group.is_some() {
            depth = depth.saturating_sub(1);
        }
        if let Some(group_index) = token.opens_group {
            let group = &document.groups[group_index];
            let has_contents = group.close > group.open + 1;
            if group.wrappable && has_contents && !wrapped[group_index] {
                let better = match best {
                    Some((best_depth, _)) => depth < best_depth,
                    None => true,
                };
                if better {
                    best = Some((depth, group_index));
                }
            }
            depth += 1;
        }
    }
    best.map(|(_, group)| group)
}

/// Puts every element of a group on its own line.
fn wrap_group(document: &LayoutDocument, group_index: usize, breaks: &mut [usize]) {
    let group = &document.groups[group_index];
    add_line_break(document, breaks, group.open + 1, group.close);
    for &separator in &group.separators {
        if separator + 1 < group.close {
            add_line_break(document, breaks, separator + 1, group.close);
        }
    }
    add_line_break(document, breaks, group.close, group.close);
}

/// Adds a line break before token `at`. A comment that trails on the same line stays
/// with the code before it, so the break moves past it.
fn add_line_break(document: &LayoutDocument, breaks: &mut [usize], at: usize, limit: usize) {
    let mut index = at;
    while index < limit && document.tokens[index].is_comment && breaks[index] == 0 {
        index += 1;
    }
    if breaks[index] == 0 {
        breaks[index] = 1;
    }
}

/// Whether a space is printed between two tokens on the same line.
fn needs_space(previous: &LayoutToken, token: &LayoutToken) -> bool {
    match token.spacing {
        Spacing::Space => true,
        Spacing::Keep => token.had_space_before,
        Spacing::NoSpace => token.had_space_before && would_merge(&previous.text, &token.text),
    }
}

/// True when printing two tokens without a space between them could make them read as
/// different tokens, for example `-` `-` becoming `--`, or `a` `b` becoming `ab`.
fn would_merge(left: &str, right: &str) -> bool {
    let last = match left.chars().last() {
        Some(character) => character,
        None => return false,
    };
    let first = match right.chars().next() {
        Some(character) => character,
        None => return false,
    };
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '$' || !c.is_ascii();
    let is_operator = |c: char| "+-*/%=<>!&|^~?:.@#".contains(c);
    let is_quote = |c: char| c == '"' || c == '\'' || c == '`';
    (is_word(last) && is_word(first))
        || (is_operator(last) && is_operator(first))
        || (last.is_ascii_digit() && first == '.')
        || (last == '.' && first.is_ascii_digit())
        || (is_word(last) && is_quote(first))
}

fn indent_width(level: usize, options: &PrintOptions) -> usize {
    level * options.indent_size
}

fn indent_text(level: usize, options: &PrintOptions) -> String {
    if options.use_tabs {
        "\t".repeat(level)
    } else {
        " ".repeat(level * options.indent_size)
    }
}

/// Whitespace for a width given in columns (used when shifting multi-line tokens).
fn whitespace_for_width(width: usize, options: &PrintOptions) -> String {
    if options.use_tabs {
        let tabs = width / options.indent_size;
        let spaces = width % options.indent_size;
        "\t".repeat(tabs) + &" ".repeat(spaces)
    } else {
        " ".repeat(width)
    }
}

fn text_width(text: &str) -> usize {
    text.chars().count()
}

/// Whether the last token of the line is a comment that trails code.
fn has_trailing_comment(document: &LayoutDocument, line: &Line) -> bool {
    let last = &document.tokens[line.last];
    line.last > line.first && last.is_comment && !last.is_multi_line()
}

/// The width of a line without its trailing comment. Returns `None` when the line
/// contains a multi-line token, because such lines are never wrapped or aligned.
fn code_width(document: &LayoutDocument, line: &Line, options: &PrintOptions) -> Option<usize> {
    let mut last = line.last;
    if has_trailing_comment(document, line) {
        last -= 1;
    }
    let mut width = indent_width(line.indent, options);
    for index in line.first..=last {
        let token = &document.tokens[index];
        if token.is_multi_line() {
            return None;
        }
        if index > line.first && needs_space(&document.tokens[index - 1], token) {
            width += 1;
        }
        width += text_width(&token.text);
    }
    Some(width)
}

/// For each line, the column where its trailing comment starts, if it is aligned.
///
/// Consecutive lines with trailing comments and the same indentation form a block. All
/// comments in a block start in the same column: two spaces after the longest line.
fn comment_columns(document: &LayoutDocument, lines: &[Line], options: &PrintOptions) -> Vec<Option<usize>> {
    let mut columns: Vec<Option<usize>> = vec![None; lines.len()];
    if !options.align_comments {
        return columns;
    }
    let mut block_start = 0;
    while block_start < lines.len() {
        let first_width = if has_trailing_comment(document, &lines[block_start]) {
            code_width(document, &lines[block_start], options)
        } else {
            None
        };
        let mut widest = match first_width {
            Some(width) => width,
            None => {
                block_start += 1;
                continue;
            }
        };
        let mut block_end = block_start + 1;
        while block_end < lines.len() {
            let line = &lines[block_end];
            let continues_block = line.blank_lines_before == 0
                && line.indent == lines[block_start].indent
                && has_trailing_comment(document, line);
            if !continues_block {
                break;
            }
            match code_width(document, line, options) {
                Some(width) => widest = widest.max(width),
                None => break,
            }
            block_end += 1;
        }
        for column in columns.iter_mut().take(block_end).skip(block_start) {
            *column = Some(widest + ALIGNED_COMMENT_GAP);
        }
        block_start = block_end;
    }
    columns
}

/// Builds the final text.
fn render(document: &LayoutDocument, lines: &[Line], options: &PrintOptions) -> String {
    let columns = comment_columns(document, lines, options);
    let mut output = String::new();
    for (line_index, line) in lines.iter().enumerate() {
        if line_index > 0 {
            output.push('\n');
            for _ in 0..line.blank_lines_before {
                output.push('\n');
            }
        }
        output.push_str(&indent_text(line.indent, options));
        let mut column = indent_width(line.indent, options);

        for index in line.first..=line.last {
            let token = &document.tokens[index];
            if index > line.first {
                let is_trailing_comment = index == line.last && has_trailing_comment(document, line);
                if is_trailing_comment {
                    let target = columns[line_index].unwrap_or(0);
                    let spaces = if target > column { target - column } else { 1 };
                    output.push_str(&" ".repeat(spaces));
                    column += spaces;
                } else if needs_space(&document.tokens[index - 1], token) {
                    output.push(' ');
                    column += 1;
                }
            }
            let first_on_line = index == line.first;
            column = write_token(&mut output, token, line, first_on_line, column, options);
        }
    }
    output.push('\n');
    output
}

/// Writes one token and returns the column after it.
fn write_token(
    output: &mut String,
    token: &LayoutToken,
    line: &Line,
    first_on_line: bool,
    column: usize,
    options: &PrintOptions,
) -> usize {
    if !token.is_multi_line() {
        output.push_str(&token.text);
        return column + text_width(&token.text);
    }

    let mut mode = token.multi_line;
    if mode == MultiLine::StarAligned && !first_on_line {
        mode = MultiLine::Shift;
    }
    let new_indent = indent_width(line.indent, options) as isize;
    let shift = new_indent - token.original_line_indent as isize;

    let mut last_line_width = 0;
    for (number, text_line) in token.text.split('\n').enumerate() {
        let printed = if number == 0 {
            text_line.to_string()
        } else {
            output.push('\n');
            match mode {
                MultiLine::Verbatim => text_line.to_string(),
                MultiLine::StarAligned => {
                    let content = text_line.trim();
                    if content.is_empty() {
                        String::new()
                    } else {
                        indent_text(line.indent, options) + " " + content
                    }
                }
                MultiLine::Shift => {
                    if token.protected_lines.contains(&number) {
                        text_line.to_string()
                    } else {
                        shift_line(text_line, shift, token.is_comment, options)
                    }
                }
            }
        };
        output.push_str(&printed);
        last_line_width = text_width(&printed);
        if number == 0 {
            last_line_width += column;
        }
    }
    last_line_width
}

/// Moves one line of a multi-line token left or right.
fn shift_line(text: &str, shift: isize, trim_end: bool, options: &PrintOptions) -> String {
    let content = text.trim_start();
    let content = if trim_end { content.trim_end() } else { content };
    if content.trim().is_empty() {
        return String::new();
    }
    let leading = &text[..text.len() - text.trim_start().len()];
    let mut width: isize = 0;
    for character in leading.chars() {
        if character == '\t' {
            width += options.indent_size as isize;
        } else {
            width += 1;
        }
    }
    let new_width = (width + shift).max(0) as usize;
    whitespace_for_width(new_width, options) + content
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Group, LayoutToken};

    fn options() -> PrintOptions {
        PrintOptions {
            indent_size: 4,
            use_tabs: false,
            column_width: 40,
            align_comments: true,
            wrap_lines: true,
        }
    }

    /// Builds `name(a, b, c)` style documents by hand.
    fn call(name: &str, arguments: &[&str]) -> LayoutDocument {
        let mut tokens = vec![LayoutToken::new(name), LayoutToken::new("(")];
        tokens[1].spacing = Spacing::NoSpace;
        let mut separators = Vec::new();
        for (position, argument) in arguments.iter().enumerate() {
            if position > 0 {
                let mut comma = LayoutToken::new(",");
                comma.spacing = Spacing::NoSpace;
                separators.push(tokens.len());
                tokens.push(comma);
            }
            let mut token = LayoutToken::new(argument);
            token.spacing = if position == 0 {
                Spacing::NoSpace
            } else {
                Spacing::Space
            };
            tokens.push(token);
        }
        let mut close = LayoutToken::new(")");
        close.spacing = Spacing::NoSpace;
        tokens.push(close);
        let close_index = tokens.len() - 1;
        tokens[1].opens_group = Some(0);
        tokens[close_index].closes_group = Some(0);
        LayoutDocument {
            tokens,
            groups: vec![Group {
                open: 1,
                close: close_index,
                separators,
                wrappable: true,
            }],
        }
    }

    #[test]
    fn short_call_stays_on_one_line() {
        let document = call("foo", &["a", "b"]);
        assert_eq!(print(&document, &options()), "foo(a, b)\n");
    }

    #[test]
    fn long_call_is_wrapped_one_argument_per_line() {
        let document = call("function_name", &["first_argument", "second_argument"]);
        assert_eq!(
            print(&document, &options()),
            "function_name(\n    first_argument,\n    second_argument\n)\n"
        );
    }

    #[test]
    fn wrapping_is_skipped_when_disabled() {
        let document = call("function_name", &["first_argument", "second_argument"]);
        let mut settings = options();
        settings.wrap_lines = false;
        assert_eq!(
            print(&document, &settings),
            "function_name(first_argument, second_argument)\n"
        );
    }

    #[test]
    fn tabs_are_used_for_indentation() {
        let document = call("function_name", &["first_argument", "second_argument"]);
        let mut settings = options();
        settings.use_tabs = true;
        assert_eq!(
            print(&document, &settings),
            "function_name(\n\tfirst_argument,\n\tsecond_argument\n)\n"
        );
    }

    #[test]
    fn tokens_that_would_merge_keep_their_space() {
        assert!(would_merge("-", "-"));
        assert!(would_merge("a", "b"));
        assert!(would_merge("1", ".x"));
        assert!(would_merge("/", "/"));
        assert!(!would_merge("a", "("));
        assert!(!would_merge(")", ";"));
    }

    #[test]
    fn trailing_comments_are_aligned() {
        let mut tokens = Vec::new();
        let lines = [
            ("var first = 10;", "// First value"),
            ("var second = 20;", "// Second value"),
        ];
        for (code, comment) in lines {
            let mut code_token = LayoutToken::new(code);
            code_token.line_breaks_before = 1;
            tokens.push(code_token);
            let mut comment_token = LayoutToken::new(comment);
            comment_token.is_comment = true;
            comment_token.had_space_before = true;
            tokens.push(comment_token);
        }
        let document = LayoutDocument {
            tokens,
            groups: Vec::new(),
        };
        assert_eq!(
            print(&document, &options()),
            "var first = 10;   // First value\nvar second = 20;  // Second value\n"
        );
        let mut settings = options();
        settings.align_comments = false;
        assert_eq!(
            print(&document, &settings),
            "var first = 10; // First value\nvar second = 20; // Second value\n"
        );
    }

    #[test]
    fn star_comments_are_realigned() {
        let mut token = LayoutToken::new("/**\n       * Text\n       */");
        token.is_comment = true;
        token.multi_line = MultiLine::StarAligned;
        let mut document = call("f", &["x"]);
        // Put the comment inside the group so it gets indented.
        token.line_breaks_before = 1;
        document.tokens.insert(2, token);
        document.tokens[3].line_breaks_before = 1;
        document.groups[0].close += 1;
        let last = document.tokens.len() - 1;
        document.tokens[last].line_breaks_before = 1;
        assert_eq!(
            print(&document, &options()),
            "f(\n    /**\n     * Text\n     */\n    x\n)\n"
        );
    }
}
