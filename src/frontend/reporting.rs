// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::source::Source;
use crate::frontend::token::Span;

const REPORT_CONFIG: ariadne::Config = ariadne::Config::new()
    .with_cross_gap(true)
    .with_index_type(ariadne::IndexType::Byte);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SyntaxErrorType {
    IllegalCharacter { span: Span },
    InvalidItem { span: Span, reason: &'static str },
    UnexpectedCharacter,
    UnterminatedItem,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SyntaxError {
    pub kind: SyntaxErrorType,
    pub item_name: &'static str,
    pub note: Option<&'static str>,
    pub token_span: Span,
}

pub fn print_syntax_error<S: Source>(err: &SyntaxError, source: &S) {
    use ariadne::{Color, Label, ReportKind};

    let sid = source.source_id();

    let mut report = ariadne::Report::build(ReportKind::Error, (sid, err.token_span.range()))
        .with_config(REPORT_CONFIG)
        .with_label(
            Label::new((sid, err.token_span.range())) //.with_message("in this token")
                //.with_color(Color::Black),
        );
    //.with_note(format!("in {}", err.item_name));

    if let Some(note) = err.note {
        report.set_note(note);
    }

    //warn!(target:"report", "token_span = {:?}", err.token_span.range());

    match err.kind {
        SyntaxErrorType::IllegalCharacter { span } => {
            report.set_message(format!("illegal character in {}", err.item_name));
            report.add_label(
                Label::new((sid, span.range()))
                
            );
            warn!(target:"report", "item_span = {:?}", span.range());
        }
        SyntaxErrorType::InvalidItem { span, reason } => {
            report.set_message(format!("invalid {}", err.item_name));
            report.add_label(
                Label::new((sid, span.range()))
                    .with_message(reason)
                    .with_color(Color::Red),
            );
            //warn!(target:"report", "item_span = {:?}", span.range());
        }
        SyntaxErrorType::UnterminatedItem => {
            report.set_message(format!("unterminated {}", err.item_name));
        }
        SyntaxErrorType::UnexpectedCharacter => {
            report.set_message("Unexpected character while scanning tokens");
        }
    }

    report
        .finish()
        .print((sid, ariadne::Source::from(source.text())))
        .unwrap();
}
