use crate::frontend::Span;
use ariadne::{Color, ColorGenerator, Fmt, Label, Report, ReportKind, Source};

#[derive(Debug)]
pub enum ErrorKind {
    Lexical,
    Syntax,
    Semantic
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub file: String,
    pub source: String,
    pub span: Span,
    pub message: String,
    pub note: Option<String>
}

impl Error {
    pub fn new(
        kind: ErrorKind,
        file: &str,
        source: &str,
        span: Span,
        message: &str,
        note: Option<&str>
    ) -> Self {
        Self {
            kind,
            file: file.to_string(),
            source: source.to_string(),
            span,
            message: message.to_string(),
            note: note.map(|s| s.to_string())
        }
    }

    pub fn display(&self) {
        let mut colors = ColorGenerator::new();
        let label = colors.next();
        let note = Color::Fixed(81);
        let code = match self.kind {
            ErrorKind::Lexical => 1,
            ErrorKind::Syntax => 2,
            ErrorKind::Semantic => 3
        };
        let mut report = Report::build(ReportKind::Error, (self.file.as_str(), self.span.start..self.span.end))
            .with_code(code)
            .with_message(self.message.clone())
            .with_label(
                Label::new((self.file.as_str(), self.span.start..self.span.end))
                    .with_message(self.message.clone().fg(label))
                    .with_color(label)
            );
        if let Some(n) = &self.note {
            report = report.with_note(n.clone().fg(note));
        }
        let _ = report
            .finish()
            .eprint((self.file.as_str(), Source::from(self.source.clone())));
    }
}
