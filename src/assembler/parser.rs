use super::lexer::{Token, lex};
use super::{Diagnostic, ErrorKind, SourceLocation};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Operand<'a> {
    Atom(&'a str),
    Memory(Option<&'a str>, &'a str),
    Bytes(Vec<u8>),
}

pub(super) struct Statement<'a> {
    pub location: SourceLocation,
    pub name: String,
    pub operands: Vec<(SourceLocation, Operand<'a>)>,
}

pub(super) struct Line<'a> {
    pub labels: Vec<(SourceLocation, &'a str)>,
    pub statement: Option<Statement<'a>>,
}

pub(super) fn symbol(value: &str) -> bool {
    let mut chars = value.bytes();
    chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || matches!(ch, b'_' | b'.'))
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'_' | b'.' | b'$'))
}

pub(super) fn parse(source: &str) -> Result<Vec<Line<'_>>, Diagnostic> {
    source
        .strip_prefix('\u{feff}')
        .unwrap_or(source)
        .lines()
        .enumerate()
        .map(|(index, line)| parse_line(line, index + 1))
        .collect()
}

fn parse_line(line: &str, number: usize) -> Result<Line<'_>, Diagnostic> {
    let tokens = lex(line, number)?;
    let mut remaining = tokens.as_slice();
    let mut labels = Vec::new();
    while let [(location, Token::Atom(name)), (_, Token::Colon), rest @ ..] = remaining {
        if !symbol(name) {
            return Err(location.error(ErrorKind::Syntax, "invalid label name"));
        }
        labels.push((*location, *name));
        remaining = rest;
    }
    let statement = match remaining {
        [] => None,
        [(location, Token::Atom(name)), rest @ ..] => {
            let mut operands = Vec::new();
            if !rest.is_empty() {
                for group in rest.split(|(_, token)| *token == Token::Comma) {
                    let location = group.first().map_or(*location, |(location, _)| *location);
                    let operand = match group {
                        [(_, Token::Atom(value))] => Operand::Atom(value),
                        [(_, Token::Bytes(bytes))] => Operand::Bytes(bytes.clone()),
                        [(_, Token::Open), (_, Token::Atom(base)), (_, Token::Close)] => {
                            Operand::Memory(None, base)
                        }
                        [
                            (_, Token::Atom(offset)),
                            (_, Token::Open),
                            (_, Token::Atom(base)),
                            (_, Token::Close),
                        ] => Operand::Memory(Some(offset), base),
                        _ => {
                            return Err(location
                                .error(ErrorKind::Syntax, "expected a comma-separated operand"));
                        }
                    };
                    operands.push((location, operand));
                }
            }
            Some(Statement {
                location: *location,
                name: name.to_ascii_lowercase(),
                operands,
            })
        }
        [(location, _), ..] => {
            return Err(location.error(ErrorKind::Syntax, "expected an instruction or directive"));
        }
    };
    Ok(Line { labels, statement })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_comments_memory_and_utf8() {
        let lines =
            parse("  loop: again: LW $t0, -4($sp) # comment\n.asciiz \"é#;\\n\"\n").unwrap();
        let line = &lines[0];
        assert_eq!(line.labels[0].1, "loop");
        assert_eq!(line.labels[1].1, "again");
        let instruction = line.statement.as_ref().unwrap();
        assert_eq!(instruction.name, "lw");
        assert_eq!(
            instruction.operands[1].1,
            Operand::Memory(Some("-4"), "$sp")
        );
        assert_eq!(
            lines[1].statement.as_ref().unwrap().operands[0].1,
            Operand::Bytes("é#;\n".as_bytes().to_vec())
        );
    }

    #[test]
    fn invalid_operand_has_line_and_column() {
        let error = parse("# comment\n  lw $t0, 4($sp").err().unwrap();
        assert_eq!(error.kind, ErrorKind::Syntax);
        assert_eq!(
            error.location,
            SourceLocation {
                line: 2,
                column: 11
            }
        );
    }

    #[test]
    fn rejects_empty_operands_invalid_labels_and_unterminated_strings() {
        for line in ["add $1,, $2", "add $1,", "2bad: nop", ".ascii \"unclosed"] {
            assert!(parse(line).is_err(), "{line}");
        }
    }
}
