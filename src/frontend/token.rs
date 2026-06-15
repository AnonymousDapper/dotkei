// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

macro_rules! keyword_build {
    ($kw_name:ident, $fn_name:ident, $kind:ident, $( $variant:ident : $name:literal ),+ ) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum $kw_name {
            $(
            $variant,
            )+
        }

        pub fn $fn_name(bytes: &[u8]) -> $kind {
            match bytes {
                $(
                $name => $kind::Keyword($kw_name::$variant),
                )+
                _ => $kind::Ident
            }
        }
    }
}

pub type Index = u32;

pub type ErrorId = u16;

//pub type LiteralId = Index;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: Index,
    pub end: Index,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub trivia_start: Index,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Ident,
    Int,
    Float,

    Keyword(KeywordType),

    // Single byte operators
    Comma,     // ,
    Semi,      // ;
    Plus,      // +
    Star,      // *
    Intrinsic, // '
    Pipe,      // |
    LParen,    // (
    RParen,    // )
    LSquare,   // [
    RSquare,   // ]
    LBrace,    // {
    RBrace,    // }
    Amp,       // &

    Dot,    // .
    Colon,  // :
    Equals, // =

    // Double byte operators
    Unit,      // ()
    Arrow,     // ->
    BackArrow, // <-
    BigArrow,  // =>
    Fence,     // ::
    StepDot,   // ..
    StepDotEq, // ..=

    Operator,

    // String-like tokens
    String, // " ... "
    ByteString,

    Char, // ` . `
    Byte, // b` . `

    // String interpolation tokens
    FStrStart,     // f"
    FStrContent,   // ...
    FStrExprStart, // {
    FStrExprEnd,   // }
    FStrEnd,       // "

    // Trivial operators
    Comment,
    OuterDoc,
    InnerDoc,
    Newline,

    Eof,
}

// KeywordType token -> literal text match
keyword_build!(KeywordType, match_keyword_slice, TokenKind,
    As:         b"as",
    //Begin:      b"begin",
    Break:      b"break",
    By:         b"by",
    //Condition:  b"condition",
    Const:      b"const",
    Continue:   b"continue",
    Data:       b"data",
    //Decl:       b"decl",
    Do:         b"do",
    //Done:       b"done",
    Else:       b"else",
    //Elsif:      b"elsif",
    Enum:       b"enum",
    End:        b"end",
    r#Fn:       b"fn",
    For:        b"for",
    //ForAll:     b"forall",
    If:         b"if",
    Impl:       b"impl",
    In:         b"in",
    Infix:      b"infix",
    InfixL:     b"infixl",
    InfixR:     b"infixr",
    Interface:  b"iface",
    Is:         b"is",
    //Into:     b"into",
    //Length:   b"length",
    Let:        b"let",
    Loop:       b"loop",
    Match:      b"match",
    Mut:        b"mut",
    Operator:   b"op",
    Postfix:    b"postfix",
    Prefix:     b"prefix",
    Pub:        b"pub",
    //Range:    b"range",
    Return:     b"return",
    Root:       b"root",
    SelfType:   b"Self",
    SelfValue:  b"self",
    Super:      b"super",
    Type:       b"type",
    Until:      b"until",
    Use:        b"use",
    Where:      b"where",
    While:      b"while",
    When:       b"when",
    With:       b"with"

);

impl Span {
    pub fn new(start: Index, end: Index) -> Span {
        Span { start, end }
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl std::ops::Add<Span> for Span {
    type Output = Span;

    fn add(self, rhs: Span) -> Self::Output {
        Span {
            start: self.start,
            end: rhs.end,
        }
    }
}
