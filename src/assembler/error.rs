use std::fmt;

/// A one-based line and byte column in the UTF-8 input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    InvalidOperand,
    UnsupportedInstruction,
    UnknownDirective,
    DuplicateSymbol,
    UndefinedSymbol,
    OutOfRange,
    Misaligned,
    Layout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub location: SourceLocation,
    pub kind: ErrorKind,
    pub message: String,
}

impl SourceLocation {
    pub(super) fn error(self, kind: ErrorKind, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            location: self,
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.location.line, self.location.column, self.message
        )
    }
}

impl std::error::Error for Diagnostic {}
