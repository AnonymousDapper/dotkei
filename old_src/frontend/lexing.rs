// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

// TODO: read the fucking rustc source

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::reporting::{SyntaxError, SyntaxErrorType};
use crate::frontend::source::Source;

use crate::frontend::text::{
    ascii_to_hex, decode_utf8_char, get_check_for_based_int, is_ident_continue_ascii,
    is_ident_continue_unicode, is_ident_start_unicode, is_numeric_with_sep_ascii,
    is_operator_ascii, is_operator_unicode, is_whitespace_no_nl, test_basic_escape,
};

use crate::frontend::token::{Index, Span, Token, TokenKind, match_keyword_slice};

use crate::storage::{LiteralStorage, LiteralType};

#[derive(PartialEq)]
enum State {
    Expression { is_inside_fmt: bool },
    Text,
}

#[derive(Debug, PartialEq)]
pub enum LexerStatus {
    Ok(Token),
    Recovered,
    Abort,
}

enum RecoveryPolicy {
    Dont,
    ToNewline,
    ToDoubleQuote,
    ToBacktick,
    MaybeToChar(u8),
}

pub struct Lexer<'src, S> {
    source: &'src S,
    current: Index,
    start: Index,
    trivia_start: Index,
    state: State,
    fmt_string_start: Option<Index>,
    errors: Vec<SyntaxError>,
    literals: LiteralStorage,
}

impl<'src, S> Lexer<'src, S>
where
    S: Source,
{
    pub fn new(source: &'src S) -> Lexer<'src, S> {
        Lexer {
            source,
            current: 0,
            start: 0,
            trivia_start: 0,
            state: State::Expression {
                is_inside_fmt: false,
            },
            fmt_string_start: None,
            errors: Vec::new(),
            literals: LiteralStorage::new(),
        }
    }

    // Lexer internals
    // ----------------------------------------------------

    fn peek(&self) -> u8 {
        self.source.read_at(self.current).unwrap_or(b'\0')
    }

    fn peek_next(&self) -> u8 {
        self.source.read_at(self.current + 1).unwrap_or(b'\0')
    }

    fn advance(&mut self) -> u8 {
        let c = self.peek();
        self.current += 1;

        c
    }

    fn advance_by(&mut self, amnt: Index) {
        self.current += amnt;
    }

    fn start_token(&mut self) {
        self.start = self.current;
    }

    fn start_trivia(&mut self) {
        self.trivia_start = self.current;
    }

    fn span(&self) -> Span {
        Span::new(self.start, self.current)
    }

    fn format_string_span(&self) -> Span {
        Span::new(self.fmt_string_start.unwrap_or(self.start), self.current)
    }

    fn offset_unicode_from(&mut self, index: Index) -> Span {
        while self.peek() >= 0x80 {
            self.advance();
        }

        Span::new(index, self.current)
    }

    fn trivia_index(&self) -> Index {
        self.trivia_start
    }

    fn single_span_at(&self, offset: Index) -> Span {
        Span::new(self.current - offset, (self.current - offset) + 1)
    }

    fn double_span_at(&self, offset: Index) -> Span {
        Span::new((self.current - offset) - 2, self.current - offset)
    }

    fn lex_trivia(&mut self) {
        self.start_trivia();

        while is_whitespace_no_nl(self.peek()) {
            self.advance();
        }

        //debug!(target:"trivia", "Processed {} bytes of trivia", self.current - self.trivia_start);
    }

    fn single(&mut self, kind: TokenKind) -> LexerStatus {
        self.start_token();
        self.advance();

        LexerStatus::Ok(Token {
            kind,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn double(&mut self, kind: TokenKind) -> LexerStatus {
        self.start_token();
        self.advance();
        self.advance();

        LexerStatus::Ok(Token {
            kind,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn recover_to_newline(&mut self) {
        while !matches!(self.peek(), b'\n' | b'\0') {
            self.advance();
        }
        self.advance();
    }

    fn recover_to(&mut self, byte: u8) {
        while !matches!(self.peek(), b @ b'\n' | b @ b'\0' | b if b == byte) {
            self.advance();
        }
        self.advance();
    }

    // TODO: figure out recovery reporting for keeping track of lexer state in format strings
    fn emit_error(&mut self, err: SyntaxError, recovery: RecoveryPolicy) -> LexerStatus {
        self.errors.push(err);

        match recovery {
            RecoveryPolicy::ToNewline => {
                if let State::Expression {
                    is_inside_fmt: true,
                } = self.state
                {
                    self.recover_to(b'}');
                } else {
                    self.recover_to_newline();
                }
            }
            RecoveryPolicy::MaybeToChar(b) => {
                self.recover_to(b);
            }
            RecoveryPolicy::ToDoubleQuote => {
                self.recover_to(b'"');
            }
            RecoveryPolicy::ToBacktick => {
                self.recover_to(b'`');
            }
            RecoveryPolicy::Dont => {}
        }

        LexerStatus::Recovered
    }

    // Public status methods
    // ----------------------------------------------------

    pub fn get_errors(&self) -> &Vec<SyntaxError> {
        &self.errors
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    // Internal token producing methods
    // ----------------------------------------------------

    fn comment(&mut self) -> LexerStatus {
        self.start_token();
        //trace!(target:"comment", "starting comment parse, current = {:#?}", self.peek() as char);

        self.advance_by(2); // consume -/

        let kind = match self.peek() {
            b'/' => TokenKind::OuterDoc,
            b'!' => TokenKind::InnerDoc,
            _ => TokenKind::Comment,
        };

        //trace!(target:"comment", "doc: {}, raw: {:#?}", doc_comment, self.peek() as char);

        while self.peek() != b'\n' {
            self.advance();
        }

        LexerStatus::Ok(Token {
            kind,

            trivia_start: self.trivia_index(),
            span: self.span(),
        })
    }

    // TODO: add exponent handling
    fn number(&mut self) -> LexerStatus {
        self.start_token();

        while is_numeric_with_sep_ascii(self.peek()) {
            self.advance();
        }

        if self.peek() == b'.' && self.peek_next().is_ascii_digit() {
            self.advance();

            while is_numeric_with_sep_ascii(self.peek()) {
                self.advance();
            }

            return LexerStatus::Ok(Token {
                kind: TokenKind::Float,
                span: self.span(),
                trivia_start: self.trivia_index(),
            });
        }

        LexerStatus::Ok(Token {
            kind: TokenKind::Int,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn based_int(&mut self) -> LexerStatus {
        self.start_token();
        self.advance();

        let valid_char_check = get_check_for_based_int(self.peek());
        self.advance();

        if let Some(check) = valid_char_check {
            while check(self.peek()) {
                self.advance();
            }
            LexerStatus::Ok(Token {
                kind: TokenKind::Int,
                span: self.span(),
                trivia_start: self.trivia_index(),
            })
        } else {
            let err = SyntaxError {
                kind: SyntaxErrorType::InvalidItem {
                    span: self.single_span_at(1),
                    reason: "invalid radix",
                },
                item_name: "integer literal",
                note: Some("radixes are hex: 0x, octal: 0o, and binary: 0b"),
                token_span: self.span(),
            };
            self.emit_error(err, RecoveryPolicy::MaybeToChar(b' '))
        }
    }

    fn operator_ascii(&mut self) -> LexerStatus {
        self.start_token();

        while is_operator_ascii(self.peek()) {
            self.advance();
        }

        let kind = match self.source.read_span(self.span()) {
            // handle Plus, Pipe, Dot, Equals
            // Arrow, BigArrow, StepDot, StepDotEq
            b"*" => TokenKind::Star,
            b"+" => TokenKind::Plus,
            b"|" => TokenKind::Pipe,
            b"." => TokenKind::Dot,
            b"=" => TokenKind::Equals,
            b"&" => TokenKind::Amp,

            b"->" => TokenKind::Arrow,
            b"=>" => TokenKind::BigArrow,
            b"<-" => TokenKind::BackArrow,
            b".." => TokenKind::StepDot,
            b"..=" => TokenKind::StepDotEq,

            _ => TokenKind::Operator,
        };

        LexerStatus::Ok(Token {
            kind,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn ident_ascii(&mut self) -> LexerStatus {
        self.start_token();
        self.advance();

        while is_ident_continue_ascii(self.peek()) {
            self.advance();
        }

        let kind = match_keyword_slice(self.source.read_span(self.span()));

        LexerStatus::Ok(Token {
            kind,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn unicode(&mut self) -> LexerStatus {
        self.start_token();

        let (c, len) =
            decode_utf8_char(self.source.read_to_end(self.current)).expect("invalid UTF-8");
        self.advance_by(len);

        debug!(target:"unicode", "L: {}, c: {}", len, c);
        //debug!(target: "unicode", "Start: {}, Current: {}", self.start, self.current);

        if is_operator_unicode(c) {
            // let kind = if c == '∀' {
            //     TokenKind::Keyword(KeywordType::ForAll)
            // } else {
            //     TokenKind::Operator
            // };
            let kind = TokenKind::Operator;

            return LexerStatus::Ok(Token {
                kind,
                span: self.span(),
                trivia_start: self.trivia_index(),
            });
        }

        if is_ident_start_unicode(c) {
            return self.ident_unicode();
        }

        let err = SyntaxError {
            kind: SyntaxErrorType::UnexpectedCharacter,
            item_name: "unicode character",
            note: None,
            token_span: self.span(),
        };

        self.emit_error(err, RecoveryPolicy::ToNewline)
    }

    fn ident_unicode(&mut self) -> LexerStatus {
        while self.current < self.source.length() {
            let (c, len) = match decode_utf8_char(self.source.read_to_end(self.current)) {
                Some(x) => x,
                None => break,
            };

            trace!(target:"ident unicode", "L: {}, c: {}", len, c);

            if is_ident_continue_unicode(c) {
                self.advance_by(len);
            } else {
                break;
            }
        }

        LexerStatus::Ok(Token {
            kind: TokenKind::Ident,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn string(&mut self) -> LexerStatus {
        // string = " (
        //              ![" \ \r]
        //              | quote_escape
        //              | ascii_escape
        //              | unicode_escape
        //              | newline_continue
        //          )* "

        // quote_escape = \` | \"

        // ascii_escape = \x HEX HEX <- max x7F
        //              | \a
        //              | \n
        //              | \r
        //              | \t
        //              | \\
        //              | \0

        // unicode_escape = \u { (HEX _*){1,6} }

        // newline_continue = \ NL

        self.start_token();
        trace!(target:"string", "current: {:#?}", self.peek() as char);
        self.advance(); // consume initial delimiter

        // 1. assume no-copy lexing
        // 2. escape sequences -> copy/clone value

        let mut copy_val = Vec::new();

        loop {
            let c = self.advance();

            match c {
                b'\\' => {
                    trace!(target:"string", "got escape, next: {:#?}", self.peek() as char);
                    // if we have an escape character, the source will
                    // not match the intended value so we have to allocate and parse
                    let esc_char = self.advance();

                    if let Some(new) = test_basic_escape(esc_char) {
                        copy_val.push(new);
                    } else {
                        // still need to handle \x and \u
                        match esc_char {
                            b'x' => {
                                let hex_start = self.current;
                                // ASCII hex sequence; exactly 2
                                let oct = self.advance();
                                let hex = self.advance();

                                // syntactical analysis here assures hex char is 0x7f or lower
                                if matches!(oct, b'0'..=b'7') && hex.is_ascii_hexdigit() {
                                    if let Some(digit) = ascii_to_hex(&[oct, hex])
                                        && char::from_u32(digit).is_some()
                                    {
                                        // truncating cast ok due to previous syntax checks
                                        copy_val.push(digit as u8);
                                    } else {
                                        let err = SyntaxError {
                                            kind: SyntaxErrorType::InvalidItem {
                                                span: Span::new(hex_start, self.current),
                                                reason: "invalid hex value",
                                            },
                                            item_name: "ASCII escape",
                                            note: Some(
                                                "ASCII escape sequences alllow hex values up to 0x7F",
                                            ),
                                            token_span: self.span(),
                                        };
                                        return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                    }
                                } else {
                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::IllegalCharacter {
                                            span: Span::new(hex_start, self.current),
                                        },
                                        item_name: "ASCII escape",
                                        note: Some(
                                            "ASCII escape sequences require exactly two hex digits",
                                        ),
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                }
                            }
                            b'u' => {
                                // Unicode hex sequence; 1-6
                                if self.advance() != b'{' {
                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::InvalidItem {
                                            span: self.single_span_at(1),
                                            reason: "missing opening brace",
                                        },
                                        item_name: "Unicode escape",
                                        note: Some(
                                            "Unicode escape sequences must be wrapped with curly braces",
                                        ),
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                }

                                let mut bytes = Vec::new();
                                let uc_start = self.current;

                                loop {
                                    let hex_char = self.peek();

                                    if hex_char == b'}' {
                                        break;
                                    }

                                    if hex_char.is_ascii_hexdigit() {
                                        if bytes.len() >= 6 {
                                            self.advance();
                                            let err = SyntaxError {
                                                kind: SyntaxErrorType::InvalidItem {
                                                    span: Span::new(uc_start, self.current),
                                                    reason: "too many digits",
                                                },
                                                item_name: "Unicode escape",
                                                note: Some(
                                                    "Unicode escape sequences contain at most six hex digits",
                                                ),
                                                token_span: self.span(),
                                            };
                                            return self
                                                .emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                        }
                                        bytes.push(hex_char);
                                        self.advance();
                                    } else if hex_char == b'_' {
                                        self.advance();
                                        continue;
                                    } else {
                                        let err = SyntaxError {
                                            kind: SyntaxErrorType::IllegalCharacter {
                                                span: self.single_span_at(1),
                                            },
                                            item_name: "Unicode escape",
                                            note: None,
                                            token_span: self.span(),
                                        };
                                        return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                    }
                                }

                                if self.peek() != b'}' {
                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::UnterminatedItem,
                                        item_name: "Unicode escape",
                                        note: Some("Missing closing brace"),
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                }

                                self.advance();
                                trace!(target:"string: unicode escape", "exiting, current is {:#?}", self.peek() as char);

                                if let Some(digit) = ascii_to_hex(&bytes)
                                    && let Some(unicode_char) = char::from_u32(digit)
                                {
                                    let mut slice = [0u8; 4];
                                    let encoded = unicode_char.encode_utf8(&mut slice);

                                    copy_val.extend_from_slice(encoded.as_bytes());
                                } else {
                                    let char_vec =
                                        bytes.iter().map(|b| *b as char).collect::<Vec<char>>();

                                    debug!(target:"string: unicode escape", "not a valid unicode sequence: {:#?}", char_vec);

                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::InvalidItem {
                                            span: Span::new(uc_start, self.current),
                                            reason: "Escape sequence bytes are not a valid UTF-8 codepoint",
                                        },
                                        item_name: "Unicode escape",
                                        note: None,
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                }
                            }
                            b'\n' => {
                                // line continuation
                                //self.advance();
                            }
                            _ => {
                                let err = SyntaxError {
                                    kind: SyntaxErrorType::IllegalCharacter {
                                        span: self.single_span_at(1),
                                    },
                                    item_name: "escape sequence",
                                    note: None,
                                    token_span: self.span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                            }
                        }
                    }
                }
                b'\n' => {
                    let err = SyntaxError {
                        kind: SyntaxErrorType::UnterminatedItem,
                        item_name: "string literal",
                        note: None,
                        token_span: self.span(),
                    };
                    return self.emit_error(err, RecoveryPolicy::ToNewline);
                }
                b'\"' => {
                    break;
                }
                _ => {
                    copy_val.push(c);
                }
            };
        }

        let source = match String::from_utf8(copy_val) {
            Ok(src) => LiteralType::String(src),
            Err(_) => {
                let err = SyntaxError {
                    kind: SyntaxErrorType::InvalidItem {
                        span: self.span(),
                        reason: "invalid UTF-8 sequence",
                    },
                    item_name: "string literal",
                    note: None,
                    token_span: self.span(),
                };
                return self.emit_error(err, RecoveryPolicy::ToNewline);
            }
        };

        self.literals.store_literal(source, self.start);

        LexerStatus::Ok(Token {
            kind: TokenKind::String,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn char(&mut self) -> LexerStatus {
        // char = ` (
        //          ![` \ \n \r \t]
        //          | quote_escape
        //          | ascii_escape
        //          | unicode_escape
        //      ) `

        // quote_escape = \` | \"

        // ascii_escape = \x HEX HEX <- max x7F
        //              | \a
        //              | \n
        //              | \r
        //              | \t
        //              | \\
        //              | \0

        // unicode_escape = \u { (HEX _*){1,6} }

        self.start_token();
        trace!(target:"char", "current: {:#?}", self.peek() as char);
        self.advance();

        let copy_val: char;

        let c = self.advance();

        match c {
            b'\\' => {
                trace!(target:"char", "got escape, next: {:#?}", self.peek() as char);

                let esc_char = self.advance();

                if let Some(new) = test_basic_escape(esc_char) {
                    copy_val = new as char;
                } else {
                    copy_val = match esc_char {
                        b'x' => {
                            let hex_start = self.current;
                            // ASCII hex sequence; exactly 2
                            let oct = self.advance();
                            let hex = self.advance();

                            let complete = self.peek() == b'`';

                            if matches!(oct, b'0'..=b'7') && hex.is_ascii_hexdigit() && complete {
                                if let Some(digit) = ascii_to_hex(&[oct, hex])
                                    && let Some(new_char) = char::from_u32(digit)
                                {
                                    new_char
                                } else {
                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::InvalidItem {
                                            span: Span::new(hex_start, self.current),
                                            reason: "invalid hex value",
                                        },
                                        item_name: "ASCII escape",
                                        note: Some("ASCII escapes allow hex values up to 0x7F"),
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                }
                            } else {
                                if !complete {
                                    self.advance();
                                }

                                let err = SyntaxError {
                                    kind: SyntaxErrorType::IllegalCharacter {
                                        span: Span::new(hex_start, self.current),
                                    },
                                    item_name: "ASCII escape",
                                    note: Some("ASCII escapes require exactly two hex digits"),
                                    token_span: self.span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                            }
                        }
                        b'u' => {
                            // Unicode hex sequence; 1-6
                            if self.advance() != b'{' {
                                let err = SyntaxError {
                                    kind: SyntaxErrorType::InvalidItem {
                                        span: self.single_span_at(1),
                                        reason: "missing opening brace",
                                    },
                                    item_name: "Unicode escape",
                                    note: Some(
                                        "Unicode escape sequences must be wrapped with curly braces",
                                    ),
                                    token_span: self.span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::ToBacktick);
                            }

                            let mut bytes = Vec::new();
                            let uc_start = self.current;

                            loop {
                                let hex_char = self.peek();

                                if hex_char == b'}' {
                                    break;
                                }

                                if hex_char.is_ascii_hexdigit() {
                                    if bytes.len() >= 6 {
                                        self.advance();
                                        let err = SyntaxError {
                                            kind: SyntaxErrorType::InvalidItem {
                                                span: Span::new(uc_start, self.current),
                                                reason: "too many digits",
                                            },
                                            item_name: "Unicode escape",
                                            note: Some(
                                                "Unicode escapes have at most six hex digits",
                                            ),
                                            token_span: self.span(),
                                        };
                                        return self.emit_error(err, RecoveryPolicy::ToBacktick);
                                    }
                                    bytes.push(hex_char);
                                    self.advance();
                                } else if hex_char == b'_' {
                                    self.advance();
                                    continue;
                                } else {
                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::IllegalCharacter {
                                            span: self.single_span_at(1),
                                        },
                                        item_name: "Unicode escape",
                                        note: None,
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToBacktick);
                                }
                            }

                            if self.peek() != b'}' {
                                let err = SyntaxError {
                                    kind: SyntaxErrorType::UnterminatedItem,
                                    item_name: "Unicode escape",
                                    note: Some("missing closing brace"),
                                    token_span: self.span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::ToBacktick);
                            };

                            self.advance();
                            trace!(target:"char: unicode escape", "exiting, current is {:#?}", self.peek() as char);

                            if let Some(digit) = ascii_to_hex(&bytes)
                                && let Some(unicode_char) = char::from_u32(digit)
                            {
                                unicode_char
                            } else {
                                let char_vec =
                                    bytes.iter().map(|b| *b as char).collect::<Vec<char>>();

                                debug!(target:"char: unicode escape", "not a valid unicode sequence: {:#?}", char_vec);

                                let err = SyntaxError {
                                    kind: SyntaxErrorType::InvalidItem {
                                        span: Span::new(uc_start, self.current),
                                        reason: "Escape sequence bytes are not a valid UTF-8 codepoint",
                                    },
                                    item_name: "Unicode escape",
                                    note: None,
                                    token_span: self.span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::ToBacktick);
                            }
                        }
                        _ => {
                            let err = SyntaxError {
                                kind: SyntaxErrorType::IllegalCharacter {
                                    span: self.single_span_at(1),
                                },
                                item_name: "escape sequence",
                                note: None,
                                token_span: self.span(),
                            };
                            return self.emit_error(err, RecoveryPolicy::ToBacktick);
                        }
                    };
                }
            }

            b'\n' | b'\t' | b'\r' => {
                let err = SyntaxError {
                    kind: SyntaxErrorType::IllegalCharacter {
                        span: self.single_span_at(1),
                    },
                    item_name: "character literal",
                    note: Some(
                        "Un-escaped linefeeds, tabs, and carriage returns are not permitted in character literals",
                    ),
                    token_span: self.span(),
                };
                return self.emit_error(err, RecoveryPolicy::ToBacktick);
            }

            _ => {
                copy_val = c as char;
            }
        };

        let tb = self.peek();
        if tb == b'`' {
            self.advance();
        } else {
            warn!(target:"char", "unexpected byte in character literal: {:#?}", tb as char);
            let err = SyntaxError {
                kind: SyntaxErrorType::UnterminatedItem,
                item_name: "character literal",
                note: None,
                token_span: self.span(),
            };
            return self.emit_error(err, RecoveryPolicy::ToNewline);
        }

        self.literals
            .store_literal(LiteralType::Char(copy_val), self.start);

        LexerStatus::Ok(Token {
            kind: TokenKind::Char,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn byte_string(&mut self) -> LexerStatus {
        // byte_string = " (
        //                  [^" \ \r]
        //                  | quote_escape
        //                  | byte_escape
        //                  | newline_continue
        //              )* "

        // quote_escape = \` | \"

        // byte_escape = \x HEX HEX <- max xFF
        //              | \a
        //              | \n
        //              | \r
        //              | \t
        //              | \\
        //              | \0

        // newline_continue = \ NL

        self.start_token();
        trace!(target:"byte_string", "current: {:#?}", self.peek() as char);

        // consume double delimiter pair
        self.advance();
        self.advance();

        // 1. assume no-copy lexing
        // 2. escape sequences -> copy/clone value

        let mut copy_val = Vec::new();

        loop {
            let c = self.advance();

            match c {
                b'\\' => {
                    trace!(target:"byte_string", "got escape, next: {:#?}", self.peek() as char);

                    let esc_char = self.advance();

                    if let Some(new) = test_basic_escape(esc_char) {
                        copy_val.push(new);
                    } else {
                        // still need to handle \x and \u
                        match esc_char {
                            b'x' => {
                                let hex_start = self.current;
                                // ASCII hex sequence; exactly 2
                                let hex_1 = self.advance();
                                let hex_2 = self.advance();

                                if hex_1.is_ascii_hexdigit() && hex_2.is_ascii_hexdigit() {
                                    if let Some(digit) = ascii_to_hex(&[hex_1, hex_2]) {
                                        // truncating cast ok since we're only pulling 2 hex chars
                                        copy_val.push(digit as u8);
                                    } else {
                                        let err = SyntaxError {
                                            kind: SyntaxErrorType::InvalidItem {
                                                span: Span::new(hex_start, self.current),
                                                reason: "invalid hex value",
                                            },
                                            item_name: "Byte escape",
                                            note: None,
                                            token_span: self.span(),
                                        };
                                        return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                    }
                                } else {
                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::IllegalCharacter {
                                            span: Span::new(hex_start, self.current),
                                        },
                                        item_name: "Byte escape",
                                        note: Some(
                                            "Byte escape sequences require exactly two hex digits",
                                        ),
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                                }
                            }
                            b'\n' => {
                                // line continuation
                                //self.advance();
                            }
                            _ => {
                                let err = SyntaxError {
                                    kind: SyntaxErrorType::IllegalCharacter {
                                        span: self.single_span_at(1),
                                    },
                                    item_name: "escape sequence",
                                    note: None,
                                    token_span: self.span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                            }
                        }
                    }
                }
                b'\n' => {
                    let err = SyntaxError {
                        kind: SyntaxErrorType::UnterminatedItem,
                        item_name: "byte string literal",
                        note: None,
                        token_span: self.span(),
                    };
                    return self.emit_error(err, RecoveryPolicy::ToNewline);
                }
                b'\"' => {
                    break;
                }
                _ => {
                    copy_val.push(c);
                }
            };
        }

        self.literals
            .store_literal(LiteralType::Bytes(copy_val), self.start);

        LexerStatus::Ok(Token {
            kind: TokenKind::ByteString,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    fn byte(&mut self) -> LexerStatus {
        // char = ` (
        //          ![` \ \n \r \t]
        //          | quote_escape
        //          | ascii_escape
        //          | unicode_escape
        //      ) `

        // quote_escape = \` | \"

        // ascii_escape = \x HEX HEX <- max x7F
        //              | \a
        //              | \n
        //              | \r
        //              | \t
        //              | \\
        //              | \0

        // unicode_escape = \u { (HEX _*){1,6} }

        self.start_token();
        trace!(target:"byte", "current: {:#?}", self.peek() as char);

        // consume double delimiter pair
        self.advance();
        self.advance();

        let copy_val: u8;

        let c = self.advance();

        match c {
            b'\\' => {
                trace!(target:"byte", "got escape, next: {:#?}", self.peek() as char);

                let esc_char = self.advance();

                if let Some(new) = test_basic_escape(esc_char) {
                    copy_val = new;
                } else {
                    copy_val = match esc_char {
                        b'x' => {
                            let hex_start = self.current;
                            // ASCII hex sequence; exactly 2
                            let hex_1 = self.advance();
                            let hex_2 = self.advance();

                            let complete = self.peek() == b'`';

                            if hex_1.is_ascii_hexdigit() && hex_2.is_ascii_hexdigit() && complete {
                                if let Some(digit) = ascii_to_hex(&[hex_1, hex_2]) {
                                    digit as u8
                                } else {
                                    let err = SyntaxError {
                                        kind: SyntaxErrorType::InvalidItem {
                                            span: Span::new(hex_start, self.current),
                                            reason: "Byte escape sequence is not a valid hexadecimal number",
                                        },
                                        item_name: "Byte escape",
                                        note: None,
                                        token_span: self.span(),
                                    };
                                    return self.emit_error(err, RecoveryPolicy::ToBacktick);
                                }
                            } else {
                                if !complete {
                                    self.advance();
                                }

                                let err = SyntaxError {
                                    kind: SyntaxErrorType::IllegalCharacter {
                                        span: Span::new(hex_start, self.current),
                                    },
                                    item_name: "Byte escape",
                                    note: Some(
                                        "Byte escape sequences require exactly two hex digits",
                                    ),
                                    token_span: self.span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::ToBacktick);
                            }
                        }
                        _ => {
                            let err = SyntaxError {
                                kind: SyntaxErrorType::IllegalCharacter {
                                    span: self.single_span_at(1),
                                },
                                item_name: "escape sequence",
                                note: None,
                                token_span: self.span(),
                            };
                            return self.emit_error(err, RecoveryPolicy::ToBacktick);
                        }
                    };
                }
            }

            b'\n' | b'\t' | b'\r' => {
                let err = SyntaxError {
                    kind: SyntaxErrorType::IllegalCharacter {
                        span: self.single_span_at(1),
                    },
                    item_name: "byte literal",
                    note: Some(
                        "Un-escaped linefeeds, tabs, and carriage returns are not permitted in byte literals",
                    ),
                    token_span: self.span(),
                };
                return self.emit_error(err, RecoveryPolicy::ToBacktick);
            }

            _ => {
                copy_val = c;
            }
        };

        let tb = self.peek();
        if tb == b'`' {
            self.advance();
        } else {
            warn!(target:"byte", "unexpected byte in byte literal: {:#?}", tb as char);
            let err = SyntaxError {
                kind: SyntaxErrorType::UnterminatedItem,
                item_name: "byte literal",
                note: None,
                token_span: self.span(),
            };
            return self.emit_error(err, RecoveryPolicy::ToBacktick);
        }

        self.literals
            .store_literal(LiteralType::Byte(copy_val), self.start);

        LexerStatus::Ok(Token {
            kind: TokenKind::Byte,
            span: self.span(),
            trivia_start: self.trivia_index(),
        })
    }

    // Public API
    // ----------------------------------------------------

    pub fn take_literal_storage(self) -> LiteralStorage {
        self.literals
    }

    // All sub-methods start on the byte before their corresponding
    // token value
    pub fn next_token(&mut self) -> LexerStatus {
        if self.current < self.source.length() {
            if let State::Expression { is_inside_fmt } = self.state {
                self.lex_trivia();
                let c = self.peek();

                //trace!(target:"next", "Processed trivia, now: {:#?}", c as char);

                let tok = match c {
                    b'\n' => self.single(TokenKind::Newline),
                    b'-' if self.peek_next() == b'/' => self.comment(),

                    // integer literals with specified base: 0<alphabetic><literal> (case-insensitive)
                    // validation checks are performed in the sublexer
                    b'0' if self.peek_next().is_ascii_alphabetic() => self.based_int(),

                    b'0'..=b'9' => self.number(),

                    // Punctuation not legal in operators
                    b',' => self.single(TokenKind::Comma),
                    b';' => self.single(TokenKind::Semi),
                    b'\'' => self.single(TokenKind::Intrinsic),
                    b')' => self.single(TokenKind::RParen),
                    b'[' => self.single(TokenKind::LSquare),
                    b']' => self.single(TokenKind::RSquare),
                    b'{' => self.single(TokenKind::LBrace),

                    // inside fmt handling
                    b'}' => {
                        if self.peek_next() == b'}' {
                            self.start_token();
                            self.advance_by(2);
                            let err = SyntaxError {
                                kind: SyntaxErrorType::IllegalCharacter {
                                    span: self.double_span_at(0),
                                },
                                item_name: "format string",
                                note: Some(
                                    "double curly braces not permitted in format string expression context. Exit expression context with a single brace",
                                ),
                                token_span: self.format_string_span(),
                            };
                            // attempt recovery to end of string for now
                            return self.emit_error(err, RecoveryPolicy::ToDoubleQuote);
                        }
                        if is_inside_fmt {
                            self.state = State::Text;
                            self.single(TokenKind::FStrExprEnd)
                        } else {
                            self.single(TokenKind::RBrace)
                        }
                    }

                    b'(' if self.peek_next() == b')' => self.double(TokenKind::Unit),
                    b':' if self.peek_next() == b':' => self.double(TokenKind::Fence),

                    b':' => self.single(TokenKind::Colon),
                    b'(' => self.single(TokenKind::LParen),

                    b'"' => self.string(),
                    b'`' => self.char(),

                    b'b' if self.peek_next() == b'"' => self.byte_string(),
                    b'b' if self.peek_next() == b'`' => self.byte(),

                    b'f' if self.peek_next() == b'"' => {
                        self.state = State::Text;
                        self.fmt_string_start = Some(self.start);

                        return self.double(TokenKind::FStrStart);
                    }

                    // User-defined operators, including pre-defined operators
                    // of the same valid characters
                    c if is_operator_ascii(c) => self.operator_ascii(),

                    c if c.is_ascii_alphabetic() || c == b'_' => self.ident_ascii(),

                    c if c >= 0x80 => self.unicode(),

                    _ => {
                        self.start_token();
                        self.advance();
                        let err = SyntaxError {
                            kind: SyntaxErrorType::UnexpectedCharacter,
                            item_name: "token",
                            note: None,
                            token_span: self.span(),
                        };
                        return self.emit_error(err, RecoveryPolicy::ToNewline);
                    }
                };
                return tok;
            } else if let State::Text = self.state {
                self.start_token();
                self.start_trivia();
                trace!(target:"fmt_string", "starting text parse");

                let mut copy_val = Vec::new();

                loop {
                    let c = self.peek();

                    match c {
                        b'\\' => {
                            self.advance();
                            trace!(target:"fmt_string", "got escape, next: {:#?}", self.peek() as char);

                            let esc_char = self.advance();

                            if let Some(new) = test_basic_escape(esc_char) {
                                copy_val.push(new);
                            } else {
                                match esc_char {
                                    b'x' => {
                                        let hex_start = self.current;
                                        let oct = self.advance();
                                        let hex = self.advance();

                                        if matches!(oct, b'0'..=b'7') && hex.is_ascii_hexdigit() {
                                            if let Some(digit) = ascii_to_hex(&[oct, hex])
                                                && char::from_u32(digit).is_some()
                                            {
                                                copy_val.push(digit as u8);
                                            } else {
                                                let err = SyntaxError {
                                                    kind: SyntaxErrorType::InvalidItem {
                                                        span: Span::new(hex_start, self.current),
                                                        reason: "invalid hex value",
                                                    },
                                                    item_name: "ASCII escape",
                                                    note: Some(
                                                        "ASCII escape sequences allow hex values up to 0x7F",
                                                    ),
                                                    token_span: self.format_string_span(),
                                                };
                                                return self.emit_error(err, RecoveryPolicy::Dont);
                                            }
                                        } else {
                                            let err = SyntaxError {
                                                kind: SyntaxErrorType::IllegalCharacter {
                                                    span: Span::new(hex_start, self.current),
                                                },
                                                item_name: "ASCII escape",
                                                note: Some(
                                                    "ASCII escape sequences require exactly two hex digits",
                                                ),
                                                token_span: self.format_string_span(),
                                            };
                                            return self.emit_error(err, RecoveryPolicy::Dont);
                                        }
                                    }
                                    b'u' => {
                                        if self.advance() != b'{' {
                                            let err = SyntaxError {
                                                kind: SyntaxErrorType::InvalidItem {
                                                    span: self.single_span_at(1),
                                                    reason: "missing opening brace",
                                                },
                                                item_name: "Unicode escape",
                                                note: Some(
                                                    "Unicode escape sequences must be wrapped with curly braces",
                                                ),
                                                token_span: self.format_string_span(),
                                            };
                                            return self.emit_error(err, RecoveryPolicy::Dont);
                                        }

                                        let mut bytes = Vec::new();
                                        let uc_start = self.current;

                                        loop {
                                            let hex_char = self.peek();

                                            if hex_char == b'}' {
                                                break;
                                            }

                                            if hex_char.is_ascii_hexdigit() {
                                                if bytes.len() >= 6 {
                                                    self.advance();
                                                    let err = SyntaxError {
                                                        kind: SyntaxErrorType::InvalidItem {
                                                            span: Span::new(uc_start, self.current),
                                                            reason: "too many digits",
                                                        },
                                                        item_name: "Unicode escape",
                                                        note: Some(
                                                            "Unicode escape sequences contain at most six hex digits",
                                                        ),
                                                        token_span: self.format_string_span(),
                                                    };
                                                    return self
                                                        .emit_error(err, RecoveryPolicy::Dont);
                                                }

                                                bytes.push(hex_char);
                                                self.advance();
                                            } else if hex_char == b'_' {
                                                self.advance();
                                                continue;
                                            } else {
                                                let err = SyntaxError {
                                                    kind: SyntaxErrorType::IllegalCharacter {
                                                        span: self
                                                            .offset_unicode_from(self.current),
                                                    },
                                                    item_name: "Unicode escape",
                                                    note: None,
                                                    token_span: self.format_string_span(),
                                                };
                                                return self.emit_error(err, RecoveryPolicy::Dont);
                                            }
                                        }

                                        if self.peek() != b'}' {
                                            let err = SyntaxError {
                                                kind: SyntaxErrorType::UnterminatedItem,
                                                item_name: "Unicode escape",
                                                note: Some("Missing closing brace"),
                                                token_span: self.format_string_span(),
                                            };
                                            return self.emit_error(err, RecoveryPolicy::Dont);
                                        }

                                        self.advance();
                                        trace!(target:"fmt_string: unicode escape", "exiting, current is {:#?}", self.peek() as char);

                                        if let Some(digit) = ascii_to_hex(&bytes)
                                            && let Some(unicode_char) = char::from_u32(digit)
                                        {
                                            let mut slice = [0u8; 4];
                                            let encoded = unicode_char.encode_utf8(&mut slice);

                                            copy_val.extend_from_slice(encoded.as_bytes());
                                        } else {
                                            let char_vec = bytes
                                                .iter()
                                                .map(|b| *b as char)
                                                .collect::<Vec<char>>();

                                            debug!(target:"fmt_string: unicode escape", "not a valid unicode sequence: {:#?}", char_vec);

                                            let err = SyntaxError {
                                                kind: SyntaxErrorType::InvalidItem {
                                                    span: Span::new(uc_start, self.current),
                                                    reason: "Escape sequence bytes are not a valid UTF-8 codepoint",
                                                },
                                                item_name: "Unicode escape",
                                                note: None,
                                                token_span: self.format_string_span(),
                                            };
                                            return self.emit_error(err, RecoveryPolicy::Dont);
                                        }
                                    }
                                    b'\n' => {}
                                    _ => {
                                        let err = SyntaxError {
                                            kind: SyntaxErrorType::IllegalCharacter {
                                                span: self.single_span_at(1),
                                            },
                                            item_name: "escape sequence",
                                            note: None,
                                            token_span: self.format_string_span(),
                                        };
                                        return self.emit_error(err, RecoveryPolicy::Dont);
                                    }
                                }
                            }
                        }
                        b'\n' => {
                            self.advance();
                            self.state = State::Expression {
                                is_inside_fmt: false,
                            };
                            let err = SyntaxError {
                                kind: SyntaxErrorType::UnterminatedItem,
                                item_name: "string literal",
                                note: None,
                                token_span: self.format_string_span(),
                            };
                            self.fmt_string_start = None;
                            return self.emit_error(err, RecoveryPolicy::ToNewline);
                        }
                        b'\"' => {
                            if self.current == self.start {
                                self.advance();
                                self.state = State::Expression {
                                    is_inside_fmt: false,
                                };
                                self.fmt_string_start = None;

                                return LexerStatus::Ok(Token {
                                    kind: TokenKind::FStrEnd,
                                    span: self.span(),
                                    trivia_start: self.trivia_index(),
                                });
                            } else {
                                break;
                            }
                        }
                        b'{' if self.peek_next() == b'{' => {
                            self.advance();
                            copy_val.push(b'{');

                            self.advance(); // consume second RBrace
                        }
                        b'}' => {
                            if self.peek_next() == b'}' {
                                warn!(target:"fmt", "caught double right brace");

                                self.advance_by(2);
                                copy_val.push(b'}');
                            } else {
                                self.advance();
                                let err = SyntaxError {
                                    kind: SyntaxErrorType::IllegalCharacter {
                                        span: self.single_span_at(1),
                                    },
                                    item_name: "format string",
                                    note: Some(
                                        "Unmatched closing brace. To insert curly braces in a format string, escape it with another of the same type",
                                    ),
                                    token_span: self.format_string_span(),
                                };
                                return self.emit_error(err, RecoveryPolicy::Dont);
                            }
                        }
                        b'{' => {
                            if self.current == self.start {
                                self.advance();
                                self.state = State::Expression {
                                    is_inside_fmt: true,
                                };
                                return LexerStatus::Ok(Token {
                                    kind: TokenKind::FStrExprStart,
                                    span: self.span(),
                                    trivia_start: self.trivia_index(),
                                });
                            } else {
                                break;
                            }
                        }
                        _ => {
                            copy_val.push(c);
                            self.advance();
                        }
                    }
                }

                let source = match String::from_utf8(copy_val) {
                    Ok(src) => LiteralType::String(src),
                    Err(_) => {
                        let err = SyntaxError {
                            kind: SyntaxErrorType::InvalidItem {
                                span: self.span(),
                                reason: "invalid UTF-8 sequence",
                            },
                            item_name: "format string",
                            note: None,
                            token_span: self.format_string_span(),
                        };
                        return self.emit_error(err, RecoveryPolicy::Dont);
                    }
                };

                self.literals.store_literal(source, self.start);

                return LexerStatus::Ok(Token {
                    kind: TokenKind::FStrContent,
                    span: self.span(),
                    trivia_start: self.trivia_index(),
                });
            }
        }
        // we're at the end of our input source
        LexerStatus::Abort
    } // close fn

    #[cfg(test)]
    pub fn next_as_string(&mut self) -> Option<(Token, &str, String)> {
        use TokenKind::*;

        let mut flow_val = self.next_token();

        while let LexerStatus::Recovered = flow_val {
            flow_val = self.next_token();
        }

        let tok = match flow_val {
            LexerStatus::Ok(v) => v,
            _ => return None,
        };

        let src = self.source.read_text_span(tok.span).to_owned();

        let (ty, val) = match &tok.kind {
            Ident => ("ident", src),
            Int => ("int", src),
            Float => ("float", src),
            Keyword(kwt) => ("kw", format!("{:?}", kwt)),
            Comma => ("comma", src),
            Semi => ("semi", src),
            Plus => ("plus", src),
            Star => ("star", src),
            Intrinsic => ("intrinsic", src),
            Pipe => ("pipe", src),
            LParen => ("lparen", src),
            RParen => ("rparen", src),
            LSquare => ("lsquare", src),
            RSquare => ("rsquare", src),
            LBrace => ("lbrace", src),
            RBrace => ("rbrace", src),
            Dot => ("dot", src),
            Colon => ("colon", src),
            Equals => ("equals", src),
            Amp => ("amp", src),
            Unit => ("unit", src),
            Arrow => ("arrow", src),
            BackArrow => ("backarrow", src),
            BigArrow => ("bigarrow", src),
            Fence => ("fence", src),
            StepDot => ("stepdot", src),
            StepDotEq => ("stepdoteq", src),
            Operator => ("operator", src),
            TokenKind::String => (
                "string",
                self.literals
                    .get_literal(tok.span.start)
                    .unwrap()
                    .format_value(),
            ),
            ByteString => (
                "bytestr",
                self.literals
                    .get_literal(tok.span.start)
                    .unwrap()
                    .format_value(),
            ),
            Char => (
                "char",
                self.literals
                    .get_literal(tok.span.start)
                    .unwrap()
                    .format_value(),
            ),
            Byte => (
                "byte",
                self.literals
                    .get_literal(tok.span.start)
                    .unwrap()
                    .format_value(),
            ),
            FStrStart => ("fstart", src),
            FStrContent => (
                "fcontent",
                self.literals
                    .get_literal(tok.span.start)
                    .unwrap()
                    .format_value(),
            ),
            FStrExprStart => ("fexprstart", src),
            FStrExprEnd => ("fexprend", src),
            FStrEnd => ("fend", src),
            Comment => ("comment", src),
            OuterDoc | InnerDoc => ("doc", src),
            Newline => ("nl", src),
            Eof => ("eof", std::string::String::new()),
        };

        Some((tok, ty, val))
    }

    pub fn collect_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let flow = self.next_token();
            match flow {
                LexerStatus::Ok(tok) => tokens.push(tok),
                LexerStatus::Recovered => {
                    continue;
                }
                LexerStatus::Abort => {
                    break;
                }
            }
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::source::ConstantSource;
    use crate::frontend::token::*;

    macro_rules! string_compare {
        ($source:literal => $( $ty:ident $val:literal );* ;) => {
            let source = crate::frontend::source::ConstantSource::new(String::from($source));

            let expected: Vec<(&str, String)> = vec![
                $(( stringify!($ty), String::from($val))),*
            ];

            let mut lexer = Lexer::new(&source);

            let format_line = |token: Token| {
                let tspan = token.span;

                let (sline, scol) = source.line_column(tspan.start);
                let (_, ecol) = source.line_column(tspan.end);

                let line_src = source.read_text_span(source.line_span(sline)).trim_end().to_owned();

                format!("{0}\n{1:>start$}{1:~>len$}", line_src, '^', start=(scol+1) as usize, len=(ecol-scol) as usize)

            };

            for (expect_ty, expect_val) in expected {
                match lexer.next_as_string() {
                    Some((tok, have_ty, have_val)) => {
                        if have_ty == "nl" {
                            continue;
                        }
                        if expect_ty != have_ty {
                            panic!("Type mismatch!\n{}\nWant T: {}\nHave T: {}", format_line(tok), expect_ty, have_ty);
                        }
                        if expect_val != have_val {
                            panic!("Value mismatch!\n{}\nWant V: {}\nHave V: {}", format_line(tok), expect_val, have_val);
                        }
                    },
                    None => panic!("No more tokens!\nWant T: {}\nWant V: {}", expect_ty, expect_val),
                }
            }
        }
    }

    #[test]
    fn empty_source() {
        let source = ConstantSource::new(String::from(""));

        let mut lexer = Lexer::new(&source);

        assert_eq!(lexer.next_token(), LexerStatus::Abort);
    }

    #[test]
    fn basic_ident_trivial() {
        let source = ConstantSource::new(String::from(" foo"));

        let expected = LexerStatus::Ok(Token {
            kind: TokenKind::Ident,
            trivia_start: 0,
            span: Span::new(1, 4),
        });

        let mut lexer = Lexer::new(&source);

        assert_eq!(lexer.next_token(), expected);
    }

    #[test]
    fn unicode_esc_string() {
        let source = ConstantSource::new(String::from(r#""\u{1fae5} lol""#));

        let expected = LexerStatus::Ok(Token {
            kind: TokenKind::String,
            trivia_start: 0,
            span: Span::new(0, 15),
        });

        let mut lexer = Lexer::new(&source);

        assert_eq!(lexer.next_token(), expected);
    }

    #[test]
    fn m_simple_expr() {
        string_compare!(
            "x + y - 3 / 2.5 * 0x5" =>
            ident "x";
            plus "+";
            ident "y";
            operator "-";
            int "3";
            operator "/";
            float "2.5";
            star "*";
            int "0x5";
        );
    }

    #[test]
    fn m_strings() {
        string_compare!(
            r#" "hello" b"foo" `c` b`a` "\u{1fae5}" b"\nbar" `\x65` b`\0`"# =>
            string "hello";
            bytestr "[102, 111, 111]";
            char "'c'";
            byte "97";
            string "🫥";
            bytestr "[10, 98, 97, 114]";
            char "'e'";
            byte "0";

        );
    }

    #[test]
    fn m_data_decl() {
        string_compare!(
            r#"
            -// doc
            decl data Point T where T: Numeric as
                pub x: T,
                y: T
            ;
            "# =>
            nl "\n";
            doc "-// doc"; nl "\n";
            kw "Decl"; kw "Data"; ident "Point"; ident "T"; kw "Where"; ident "T"; colon ":"; ident "Numeric"; kw "As"; nl "\n";
            kw "Pub"; ident "x"; colon ":"; ident "T"; comma ","; nl "\n";
            ident "y"; colon ":"; ident "T"; nl "\n";
            semi ";"; nl "\n";
        );
    }

    #[test]
    fn m_fn_decl() {
        string_compare!(
            r#"decl fn map f: fn T -> V, xs: Vec T -> Vec V as
                let mut vs = Vec'new;
                for x in xs do
                    vs.push f x
                ;
                vs
            ;
            "# =>
            kw "Decl"; kw "Fn"; ident "map"; ident "f"; colon ":"; kw "Fn"; ident "T"; arrow "->"; ident "V"; comma ","; ident "xs"; colon ":"; ident "Vec"; ident "T"; arrow "->"; ident "Vec"; ident "V"; kw "As"; nl "\n";
                kw "Let"; kw "Mut"; ident "vs"; equals "="; ident "Vec"; intrinsic "'"; ident "new"; semi ";"; nl "\n";
                kw "For"; ident "x"; kw "In"; ident "xs"; kw "Do"; nl "\n";
                    ident "vs"; dot "."; ident "push"; ident "f"; ident "x"; nl "\n";
                semi ";"; nl "\n";
                ident "vs"; nl "\n";
            semi ";"; nl "\n";
        );
    }

    #[test]
    fn m_op_decl() {
        string_compare!(
            r#"decl op (<$>) as infixl 7;"# =>
            kw "Decl";
            kw "Operator";
            lparen "(";
            operator "<$>";
            rparen ")";
            kw "As";
            kw "InfixL";
            int "7";
            semi ";";
        );
    }
}
