// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

pub mod builder;
pub mod green;
pub mod syntax;

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyntaxKind {
    // -- Trivia -----------------------
    Whitespace,
    Newline,
    Comment,
    DocComment,

    // -- Literals ---------------------
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    ByteStrLiteral,
    CharLiteral,
    ByteLiteral,
    FmtString,  // entire expression
    FmtContent, // text segment
    FmtExpr,    // { expr } segment

    // -- Punctuation ------------------
    TokComma,     // ,
    TokSemi,      // ;
    TokColon,     // :
    TokFence,     // ::
    TokDot,       // .
    TokStepDot,   // ..
    TokStepDotEq, // ..=
    TokEquals,    // =
    TokArrow,     // ->
    TokBigArrow,  // =>
    TokBackArrow, // <-
    TokPipe,      // |
    TokPlus,      // +
    TokLParen,    // (
    TokRParen,    // )
    TokLSquare,   // [
    TokRSquare,   // ]
    TokLBrace,    // {
    TokRBrace,    // }
    TokUnit,      // ()
    TokAmp,       // & (unary ref)
    TokStar,      // *
    TokIntrinsic, // '

    // -- Keywords ---------------------
    KwAs,
    KwBreak,
    KwBy,
    //KwCondition,
    KwConst,
    KwContinue,
    KwData,
    //KwDecl,
    KwDo,
    //KwDone,
    KwElse,
    //KwElsif
    KwEnum,
    KwEnd,
    KwFn,
    KwFor,
    //KwForAll,
    KwIf,
    KwImpl,
    KwIn,
    KwInfix,
    KwInfixL,
    KwInfixR,
    KwInterface,
    //KwInto,
    KwIs,
    //KwLength,
    KwLet,
    KwLoop,
    KwMatch,
    KwMut,
    KwOperator,
    KwPostfix,
    KwPrefix,
    KwPub,
    //KwRange,
    KwReturn,
    KwType,
    KwUntil,
    KwUse,
    KwWhere,
    KwWhile,
    KwWhen,
    KwWith,

    // -- Names ------------------------
    Ident,
    Operator,

    // -- Top level --------------------
    SourceFile,

    // -- Use items --------------------
    UseDecl,
    UseTree,
    UseAlias,
    UseGlob,
    UseItem,
    UseList,
    UsePath,

    // -- Data Decl --------------------
    DataDecl,

}
