use macros::symbols;

use super::{Interner, Symbol};

symbols! {
    Keywords {
        // Special keywords
        Underscore: "_",

        // Standard (strong) keywords
        And: "and",
        As: "as",
        Break: "break",
        By: "by",
        Const: "const",
        Continue: "continue",
        Data: "data",
        Do: "do",
        Else: "else",
        End: "end",
        Enum: "enum",
        False: "false",
        For: "for",
        Fun: "fn",
        If: "if",
        Impl: "impl",
        In: "in",
        Interface: "iface",
        Is: "is",
        Let: "let",
        Loop: "loop",
        Match: "match",
        Mut: "mut",
        Operator: "op",
        Or: "or",
        Pub: "pub",
        Return: "return",
        SelfType: "Self",
        True: "true",
        Type: "type",
        Until: "until",
        Use: "use",
        When: "when",
        Where: "where",
        While: "while",
        With: "with",

        // Weak keywords
        // these only have meaning in specific contexts
        Infix: "infix",
        InfixLeft: "infixl",
        InfixRight: "infixr",
        Postfix: "postfix",
        Prefix: "prefix",
        SelfValue: "self",
        Super: "super",
    }

    // built-in and blessed operators
    Operators {
        // single character builtin operators
        Assign: "=",
        Dot: ".",


        // two character sequence - builtin
        Arrow: "->",
        BackArrow: "<-",
        BigArrow: "=>",
        Unit: "()",

        // blessed and builtin
        Pipe: "|",
        Plus: "+",

        // blessed only - single
        // Minus: "-",
        // Mod: "%",
        // Slash: "/",
        // Star: "*",

        // Gt: ">",
        // Lt: "<",

        // Bang: "!",

        // BInv: "~",
        // BLsh: "<<",
        // BRsh: ">>",

        // blessed only - multiple
        // Equals: "==",
        // Flow: "|>",
        // GtE: ">=",
        // LtE: "<=",
        // NotEqual: "!=",
        StepDot: "..",
        StepDotEq: "..=",
    }

    Symbols {
        Placeholder,
    }
}

pub mod kw {
    pub use super::kw_generated::*;
}

pub mod op {
    pub use super::op_generated::*;
}

pub mod sym {
    use super::{Interner, Symbol};

    pub use super::sym_generated::*;

    pub fn integer<N: TryInto<usize> + Copy + itoa::Integer>(
        n: N,
        interner: &mut Interner,
    ) -> Symbol {
        if let Result::Ok(idx) = n.try_into()
            && idx < 10
        {
            return Symbol::new(super::SYMBOL_DIGITS_BASE + idx as u32);
        }

        let mut buffer = itoa::Buffer::new();
        let printed = buffer.format(n);
        //Symbol::intern(printed)
        interner.intern_str(printed)
    }

    pub const fn ascii_letter_digit(c: char) -> Option<Symbol> {
        let i = c as u32;
        Option::Some(Symbol::new(match c {
            '0'..='9' => super::SYMBOL_DIGITS_BASE + (i - '0' as u32),
            'A'..='Z' => super::SYMBOL_UPPERCASE_BASE + (i - 'A' as u32),
            'a'..='z' => super::SYMBOL_LOWERCASE_BASE + (i - 'a' as u32),
            _ => return Option::None,
        }))
    }

    pub fn character(c: char, interner: &mut Interner) -> Symbol {
        ascii_letter_digit(c).unwrap_or_else(|| {
            let mut buf: [u8; char::MAX_LEN_UTF8] = Default::default();
            //Symbol::intern(c.encode_utf8(&mut buf))
            interner.intern_str(c.encode_utf8(&mut buf))
        })
    }
}
