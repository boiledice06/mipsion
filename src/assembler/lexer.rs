use super::{Diagnostic, ErrorKind, SourceLocation};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Token<'a> {
    Atom(&'a str),
    Bytes(Vec<u8>),
    Colon,
    Comma,
    Open,
    Close,
}

pub(super) fn lex(
    line: &str,
    line_number: usize,
) -> Result<Vec<(SourceLocation, Token<'_>)>, Diagnostic> {
    let mut input = line.char_indices().peekable();
    let mut tokens = Vec::new();
    while let Some((start, ch)) = input.next() {
        let location = SourceLocation {
            line: line_number,
            column: start + 1,
        };
        let token = match ch {
            ch if ch.is_whitespace() => continue,
            '#' | ';' => break,
            ':' => Token::Colon,
            ',' => Token::Comma,
            '(' => Token::Open,
            ')' => Token::Close,
            '"' => {
                let mut bytes = Vec::new();
                loop {
                    match input.next().map(|(_, ch)| ch) {
                        Some('"') => break,
                        Some('\\') => bytes.push(match input.next().map(|(_, ch)| ch) {
                            Some('n') => b'\n',
                            Some('r') => b'\r',
                            Some('t') => b'\t',
                            Some('0') => 0,
                            Some('"') => b'"',
                            Some('\\') => b'\\',
                            _ => {
                                return Err(
                                    location.error(ErrorKind::Syntax, "invalid string escape")
                                );
                            }
                        }),
                        Some(ch) => bytes.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes()),
                        None => {
                            return Err(location.error(ErrorKind::Syntax, "unterminated string"));
                        }
                    }
                }
                Token::Bytes(bytes)
            }
            _ => {
                while input.peek().is_some_and(|&(_, ch)| {
                    !ch.is_whitespace() && !matches!(ch, '#' | ';' | ':' | ',' | '(' | ')' | '"')
                }) {
                    input.next();
                }
                let end = input.peek().map_or(line.len(), |&(offset, _)| offset);
                Token::Atom(&line[start..end])
            }
        };
        tokens.push((location, token));
    }
    Ok(tokens)
}
