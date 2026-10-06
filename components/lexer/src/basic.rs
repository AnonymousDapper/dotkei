// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

/// Basic tokenizer to extract token size and type,
/// along with limited metadata
/// (ie. string termination status, integer literal radix)
///
/// Further parse passes will "enrich" these tokens
/// and perform interning.
// simple pre-lexer to extract token size and basic type
use std::str::Chars;

use unicode_ident;
use unicode_properties::UnicodeEmoji;

use LiteralKind::*;
use TokenKind::*;

pub struct Token {
    pub kind: TokenKind,
    pub len: u32,
}

impl Token {
    fn new(kind: TokenKind, len: u32) -> Self {
        Self { kind, len }
    }
}

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    /// `-/` comments only
    Comment,
    /// `-/[!/]` doc comments
    DocComment {
        style: DocStyle,
    },

    /// any name token (identifiers and keywords)
    Ident,

    /// contiguous whitespace runs
    Whitespace,

    Literal {
        kind: LiteralKind,
    },

    /// {
    OpenBrace,
    /// (
    OpenParen,
    /// [
    OpenSquare,
    /// }
    CloseBrace,
    /// )
    CloseParen,
    /// ]
    CloseSquare,

    /// :
    Colon,
    //// ,
    Comma,
    /// #
    Hash,
    /// '
    Quote,
    /// ;
    Semi,

    /// ::
    Fence,

    /// any sequence of valid operator characters
    Operator,

    /// invalid identifier token (used for nice reporting)
    InvalidIdent,

    /// any other character
    Unknown,

    Eof,
}

#[derive(Debug, PartialEq)]
pub enum DocStyle {
    Inner,
    Outer,
}

#[derive(Debug, PartialEq)]
pub enum LiteralKind {
    Int { base: Base, empty: bool },
    Float { base: Base, empty: bool },
    Char { terminated: bool },
    Byte { terminated: bool },
    Str { terminated: bool },
    ByteStr { terminated: bool },
    FmtStr { terminated: bool },
    RawStr { terminated: bool },
    RawByteStr { terminated: bool },
    RawFmtStr { terminated: bool },
}

#[derive(Debug, PartialEq)]
pub enum Base {
    /// "0b .."
    Binary = 2,
    /// "0o .."
    Octal = 8,
    /// no prefix
    Decimal = 10,
    /// "0x .."
    Hexadecimal = 16,
}

/// Whitespace predicate based on the Rust specification
fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        | '\u{000A}' // \n
        | '\u{000B}' // vertical tab
        | '\u{000C}' // form feed
        | '\u{000D}' // \r
        | '\u{0085}' // latin1 next line
        | '\u{2028}' // line separator
        | '\u{2029}' // paragraph separator

        | '\u{200E}' // LTR mark
        | '\u{200F}' // RTL mark

        | '\u{0009}' // \t
        | '\u{0020}' // space
    )
}

fn is_id_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic() || unicode_ident::is_xid_start(c)
}

fn is_id_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || unicode_ident::is_xid_continue(c)
}

fn is_operator(c: char) -> bool {
    matches!(
        c,
        // ASCII operator chars
        |'~'| '!'
        | '@'
        | '$'
        | '%'
        | '^'
        | '&'
        | '*'
        | '-'
        | '+'
        | '='
        | '|'
        | '<'
        | '>'
        | '.'
        | '/'
        | '?'
        | '\\'

        // allowable Unicode subset
        | '∀'     // FOR ALL
        | '∃'   // THERE EXISTS
        | '∄'   // THERE DOES NOT EXIST
        | '∅'   // EMPTY SET
        | '∈'   // ELEMENT OF
        | '∉'   // NOT AN ELEMENT OF
        | '∋'   // CONTAINS AS MEMBER
        | '∌'   // DOES NOT CONTAIN AS MEMBER
        | '√'   // SQUARE ROOT
        | '⊕'   // CIRCLED PLUS
        | '⊖'   // CIRCLED MINUS
        | '⊗'   // CIRCLED TIMES
        | '∧'   // LOGICAL AND
        | '∨'   // LOGICAL OR
        | '∩'   // INTERSECTION
        | '∪'   // UNION
        | '∫'   // INTEGRAL
        | '∬'   // DOUBLE INTEGRAL
        | '∭'   // TRIPLE INTEGRAL
        | '∮'   // CONTOUR INTEGRAL
        | '≠'   // NOT EQUAL TO
        | '≡'   // IDENTICAL TO
        | '≢'   // NOT IDENTICAL TO
        | '≤'   // LESS-THAN OR EQUAL TO
        | '≥'   // GREATER-THAN OR EQUAL TO
        | '≮'   // NOT LESS-THAN
        | '≯'   // NOT GREATER-THAN
        | '⊂'   // SUBSET OF
        | '⊃'   // SUPERSET OF
        | '⊄'   // NOT A SUBSET OF
        | '⊅'   // NOT A SUPERSET OF
        | '⊆'   // SUBSET OF OR EQUAL TO
        | '⊇'   // SUPERSET OF OR EQUAL TO
        | '⊈'   // NEITHER A SUBSET OF NOR EQUAL TO
        | '⊉'   // NEITHER A SUPERSET OF NOR EQUAL TO

        | '×'   // MULTIPLICATION SIGN
        | '÷'   // DIVISION SIGN

        | '∂'   // PARTIAL DIFFERENTIAL
        | '∆'   // INCREMENT
        | '∇' // NABLA
    )
}

/// main entry point for pre-lexer
pub fn tokenize(input: &str) -> impl Iterator<Item = Token> {
    let mut lexer = BasicLexer::new(input);
    std::iter::from_fn(move || {
        let token = lexer.next_token();
        if token.kind != TokenKind::Eof {
            Some(token)
        } else {
            None
        }
    })
}

const EOF_CHAR: char = '\0';

/// non-validating lexer adapted from rustc_lexer::Cursor
/// for dotkei syntax
pub struct BasicLexer<'a> {
    len_remains: usize,
    chars: Chars<'a>,
}

impl<'a> BasicLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            len_remains: input.len(),
            chars: input.chars(),
        }
    }

    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    fn peek(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    fn peek_next(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    fn current_pos(&self) -> u32 {
        (self.len_remains - self.chars.as_str().len()) as u32
    }

    fn reset_pos(&mut self) {
        self.len_remains = self.chars.as_str().len();
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;

        Some(c)
    }

    fn advance_by(&mut self, n: usize) {
        self.chars = self.as_str()[n..].chars();
    }

    fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while predicate(self.peek()) && !self.is_eof() {
            self.advance();
        }
    }

    fn eat_until(&mut self, byte: u8) {
        self.chars = match memchr::memchr(byte, self.as_str().as_bytes()) {
            Some(index) => self.as_str()[index..].chars(),
            None => "".chars(),
        }
    }

    pub fn next_token(&mut self) -> Token {
        let Some(first) = self.advance() else {
            return Token::new(TokenKind::Eof, 0);
        };

        let kind = match first {
            '-' if self.peek() == '/' => self.comment(),

            c if is_whitespace(c) => self.whitespace(),

            // b`a` or b"foo"
            'b' => self.byte_or_byte_string(),

            // f"foo"
            'f' if self.peek() == '"' => self.format_string(),

            c if is_id_start(c) => self.ident(),

            c if is_operator(c) => self.operator(),

            c @ '0'..='9' => {
                let kind = self.number(c);
                Literal { kind }
            }

            '(' => OpenParen,
            ')' => CloseParen,
            '{' => OpenBrace,
            '}' => CloseBrace,
            '[' => OpenSquare,
            ']' => CloseSquare,

            ':' if self.peek() == ':' => {
                self.advance();
                Fence
            }

            ',' => Comma,
            ':' => Colon,
            '#' => Hash,
            '\'' => Quote,
            ';' => Semi,

            '`' => self.char_literal(),
            '"' => self.string_literal(),

            c if !c.is_ascii() && c.is_emoji_char() => self.invalid_ident(),

            _ => Unknown,
        };

        let tok = Token::new(kind, self.current_pos());
        self.reset_pos();

        tok
    }

    // Item lexers
    // --------------------------------------------------------

    fn comment(&mut self) -> TokenKind {
        self.advance();

        let style = match self.peek() {
            '!' => DocComment {
                style: DocStyle::Inner,
            },
            '/' if self.peek_next() != '/' => DocComment {
                style: DocStyle::Outer,
            },
            _ => Comment,
        };

        self.eat_until(b'\n');
        style
    }

    fn whitespace(&mut self) -> TokenKind {
        self.eat_while(is_whitespace);
        Whitespace
    }

    fn byte_or_byte_string(&mut self) -> TokenKind {
        match self.peek() {
            '`' => {
                self.advance();
                let terminated = self.single_quoted();
                let kind = Byte { terminated };
                Literal { kind }
            }
            '"' if self.peek_next() == '"' => {
                self.advance();
                self.advance();
                let terminated = self.raw_quoted();
                let kind = RawByteStr { terminated };
                Literal { kind }
            }
            '"' => {
                self.advance();
                let terminated = self.double_quoted();
                let kind = ByteStr { terminated };
                Literal { kind }
            }
            _ => self.ident(),
        }
    }

    fn format_string(&mut self) -> TokenKind {
        match self.peek() {
            '"' if self.peek_next() == '"' => {
                self.advance();
                self.advance();
                let terminated = self.raw_quoted();
                let kind = RawFmtStr { terminated };
                Literal { kind }
            }
            '"' => {
                self.advance();
                let terminated = self.double_quoted();
                let kind = FmtStr { terminated };
                Literal { kind }
            }
            _ => self.ident(),
        }
    }

    fn ident(&mut self) -> TokenKind {
        self.eat_while(is_id_continue);
        match self.peek() {
            c if !c.is_ascii() && c.is_emoji_char() => self.invalid_ident(),
            _ => Ident,
        }
    }

    fn operator(&mut self) -> TokenKind {
        self.eat_while(is_operator);
        Operator
    }

    fn char_literal(&mut self) -> TokenKind {
        let terminated = self.single_quoted();

        Literal {
            kind: LiteralKind::Char { terminated },
        }
    }

    fn string_literal(&mut self) -> TokenKind {
        let kind = if self.peek() == '"' {
            RawStr {
                terminated: self.raw_quoted(),
            }
        } else {
            Str {
                terminated: self.double_quoted(),
            }
        };

        Literal { kind }
    }

    fn invalid_ident(&mut self) -> TokenKind {
        self.eat_while(|c| {
            const ZWJ: char = '\u{200D}';
            is_id_continue(c) || (!c.is_ascii() && c.is_emoji_char()) || c == ZWJ
        });

        InvalidIdent
    }

    // Consumer helpers
    // --------------------------------------------------------

    fn single_quoted(&mut self) -> bool {
        // one-symbol literal
        if self.peek_next() == '`' && self.peek() != '\\' {
            self.advance();
            self.advance();
            return true;
        }

        // otherwise, enter parse loop
        loop {
            match self.peek() {
                '`' => {
                    self.advance();
                    return true;
                }
                //'\n' if self.peek_next() != '`' => break,
                '\n' => break,
                EOF_CHAR if self.is_eof() => break,
                '\\' => {
                    self.advance();
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        // hit break instead of return, this means
        // the string is unterminated
        false
    }

    fn double_quoted(&mut self) -> bool {
        while let Some(c) = self.advance() {
            match c {
                '"' => {
                    return true;
                }
                '\n' => break,
                '\\' if self.peek() == '\\' || self.peek() == '"' => {
                    self.advance();
                }
                _ => (),
            }
        }
        // hit Eof
        false
    }

    /// multi-line string lexer.
    /// the only way to exit this context is to
    /// terminate the string with 2 double-quotes
    fn raw_quoted(&mut self) -> bool {
        while let Some(c) = self.advance() {
            match c {
                '"' if self.peek() == '"' => {
                    self.advance();
                    self.advance();
                    return true;
                }
                _ => (),
            }
        }
        false
    }

    fn number(&mut self, start: char) -> LiteralKind {
        let mut base = Base::Decimal;

        if start == '0' {
            match self.peek() {
                'b' => {
                    base = Base::Binary;
                    self.advance();
                    if !self.eat_decimal_digits() {
                        return Int { base, empty: true };
                    }
                }
                'o' => {
                    base = Base::Octal;
                    self.advance();
                    if !self.eat_decimal_digits() {
                        return Int { base, empty: true };
                    }
                }
                'x' => {
                    base = Base::Hexadecimal;
                    self.advance();
                    if !self.eat_hexadecimal_digits() {
                        return Int { base, empty: true };
                    }
                }
                '0'..='9' | '_' => {
                    self.eat_decimal_digits();
                }
                '.' | 'e' | 'E' => {}

                _ => return Int { base, empty: false },
            }
        } else {
            self.eat_decimal_digits();
        }

        match self.peek() {
            // make sure this isn't a range operator or a method call
            '.' if self.peek_next() != '.' && !is_id_start(self.peek_next()) => {
                self.advance();
                let mut empty_exp = false;
                if self.peek().is_ascii_digit() {
                    self.eat_decimal_digits();
                    match self.peek() {
                        'e' | 'E' => {
                            self.advance();
                            empty_exp = !self.eat_float_exp();
                        }
                        _ => (),
                    }
                }
                Float {
                    base,
                    empty: empty_exp,
                }
            }
            'e' | 'E' => {
                self.advance();
                let empty_exp = !self.eat_float_exp();
                Float {
                    base,
                    empty: empty_exp,
                }
            }
            _ => Int { base, empty: false },
        }
    }

    fn eat_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.peek() {
                '_' => {
                    self.advance();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.advance();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn eat_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.peek() {
                '_' => {
                    self.advance();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.advance();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn eat_float_exp(&mut self) -> bool {
        if self.peek() == '-' || self.peek() == '+' {
            self.advance();
        }

        self.eat_decimal_digits()
    }
}
