// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

use unicode_ident::{is_xid_continue, is_xid_start};

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::token::Index;

// byte-level checks for character classes (ASCII only)

pub fn is_whitespace_no_nl(c: u8) -> bool {
    matches!(c, b'\t' | b'\x0C' | b'\r' | b' ')
}

pub fn is_operator_ascii(c: u8) -> bool {
    matches!(
        c,
        b'~' | b'!'
            | b'@'
            | b'$'
            | b'%'
            | b'^'
            | b'&'
            | b'*'
            | b'-'
            | b'+'
            | b'='
            | b'|'
            | b'<'
            | b'>'
            | b'.'
            | b'/'
            | b'?'
            | b'\\'
    )
}

pub fn is_ident_continue_ascii(c: u8) -> bool {
    matches!(c, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_')
}

pub fn is_numeric_with_sep_ascii(c: u8) -> bool {
    matches!(c, b'0'..=b'9' | b'_')
}

pub fn get_check_for_based_int(c: u8) -> Option<fn(u8) -> bool> {
    match c {
        b'x' | b'X' => Some(|b: u8| b.is_ascii_hexdigit() || b == b'_'),
        b'b' | b'B' => Some(|b: u8| matches!(b, b'0' | b'1' | b'_')),
        b'o' | b'O' => Some(|b: u8| matches!(b, b'0'..b'7' | b'_')),
        _ => None,
    }
}

/// Attempts a match against single-character, simple substitution
/// escape sequences, returning None if the supplied sequence is not one of:
///
/// ``" ` a n r t \ 0``
///
/// this does not check for `\xNN`, as that requires lexer state
pub fn test_basic_escape(c: u8) -> Option<u8> {
    match c {
        b'`' => Some(b'`'),
        b'\"' => Some(b'\"'),
        b'a' => Some(7),      // BEL
        b'n' => Some(b'\n'),  // LF
        b'r' => Some(b'\r'),  // CR
        b't' => Some(b'\t'),  // TAB
        b'\\' => Some(b'\\'), // '\'
        b'0' => Some(b'\0'),  // NUL
        _ => None,
    }
}

// char-level checks

// these should all be monospace-safe
pub fn is_operator_unicode(c: char) -> bool {
    matches!(
        c,
        '∀'     // FOR ALL
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
        | '∇'   // NABLA
        //| '∙'   // BULLET OPERATOR
    )
}

pub fn is_ident_start_unicode(c: char) -> bool {
    c == '_' || is_xid_start(c)
}

pub fn is_ident_continue_unicode(c: char) -> bool {
    is_xid_continue(c)
}

/// non-strict utf-8 decoder
/// re-implemented to avoid std checks
/// on input we've already validated on file load
pub fn decode_utf8_char(bytes: &[u8]) -> Option<(char, Index)> {
    let byte0 = *bytes.first()?;

    // the value of the first byte in a codepoint
    // determines the total length of the codepoint

    // 0xxxxxxx => 1 (ASCII)
    // 110xxxxx => 2 bytes
    // 1110xxxx => 3
    // 11110xxx => 4

    // every continuation byte follows the pattern
    // 10xxxxxx

    // bytes  | utf8 sequence  | codepoint number
    // -------+----------------+-------------------------------
    //    1   | 1st. 0xxxxxxx  | 0xxxxxxx
    // -------+----------------+-------------------------------
    //    2   | 1st. 110xxxxx  | 0xxx xxyyyyyy
    //        | 2nd. 10yyyyyy  |
    // -------+----------------+-------------------------------
    //    3   | 1st. 1110xxxx  | xxxxyyyy yyzzzzzz
    //        | 2nd. 10yyyyyy  |
    //        | 3rd. 10zzzzzz  |
    // -------+----------------+-------------------------------
    //    4   | 1st. 11110xxx  | 000xxxyy yyyyzzzz zzwwwwww
    //        | 2nd. 10yyyyyy  |
    //        | 3rd. 10zzzzzz  |
    //        | 4th. 10wwwwww  |

    // Wang Chao has a great post on this
    // https://hit9.dev/post/en/utf8

    trace!(target: "utf-8", "start: {:05b}", byte0);

    // ASCII
    if byte0 < 0x80 {
        trace!(target: "utf-8", "=> ASCII");
        return Some((byte0 as char, 1));
    }

    // 2-byte codepoint
    if (byte0 & 0b1110_0000) == 0b1100_0000 {
        trace!(target: "utf-8", "2-byte seq");

        let byte1 = *bytes.get(1)?;

        trace!(target: "utf-8", "byte 1: {:05b}", byte1);

        if (byte1 & 0b1100_0000) != 0b1000_0000 {
            trace!(target: "utf-8", "=> invalid continuation");
            return None;
        }

        let codepoint = ((byte0 & 0x1F) as u32) << 6 | ((byte1 & 0x3F) as u32);

        return char::from_u32(codepoint).map(|c| (c, 2));
    }

    // 3-byte codepoint
    if (byte0 & 0b1111_0000) == 0b1110_0000 {
        trace!(target: "utf-8", "3-byte seq");
        let byte1 = *bytes.get(1)?;
        trace!(target: "utf-8", "byte 1: {:05b}", byte1);

        let byte2 = *bytes.get(2)?;
        trace!(target: "utf-8", "byte 2: {:05b}", byte2);

        if (byte1 & 0b1100_0000) != 0b1000_0000 || (byte2 & 0b1100_0000) != 0b1000_0000 {
            trace!(target: "utf-8", "=> invalid continuation");
            return None;
        }

        let codepoint =
            ((byte0 & 0x0F) as u32) << 12 | ((byte1 & 0x3F) as u32) << 6 | ((byte2 & 0x3F) as u32);

        return char::from_u32(codepoint).map(|c| (c, 3));
    }

    // 4-byte sequence
    if (byte0 & 0b1111_1000) == 0b1111_0000 {
        trace!(target: "utf-8", "4-byte seq");

        let byte1 = *bytes.get(1)?;
        trace!(target: "utf-8", "byte 1: {:05b}", byte1);

        let byte2 = *bytes.get(2)?;
        trace!(target: "utf-8", "byte 2: {:05b}", byte2);

        let byte3 = *bytes.get(3)?;
        trace!(target: "utf-8", "byte 3: {:05b}", byte3);

        if (byte1 & 0b1100_0000) != 0b1000_0000
            || (byte2 & 0b1100_0000) != 0b1000_0000
            || (byte3 & 0b1100_0000) != 0b1000_0000
        {
            trace!(target: "utf-8", "=> invalid continuation");
            return None;
        }

        let codepoint = ((byte0 & 0x07) as u32) << 18
            | ((byte1 & 0x3F) as u32) << 12
            | ((byte2 & 0x3F) as u32) << 6
            | ((byte3 & 0x3F) as u32);

        return char::from_u32(codepoint).map(|c| (c, 4));
    }

    trace!(target: "utf-8", "=> unknown value");

    None
}

/// vendored implementation of the experimental int_from_ascii
/// tailored for safety conditions here:
/// - radix is always 16
/// - result type is always u32
/// - shouldn't need to parse more than 6 digits; caller
///   assumes responsibility for possible overflow conditions
pub fn ascii_to_hex(src: &[u8]) -> Option<u32> {
    let mut result: u32 = 0;
    let mut digits = src;

    if src.is_empty() {
        return None;
    }

    while let [c, rest @ ..] = digits {
        result *= 16;

        let x = ascii_to_hexdigit(*c)?;

        result += x as u32;
        digits = rest;
    }

    Some(result)
}

pub fn ascii_to_hexdigit(src: u8) -> Option<u8> {
    match src {
        b'0' => Some(0),
        b'1' => Some(1),
        b'2' => Some(2),
        b'3' => Some(3),
        b'4' => Some(4),
        b'5' => Some(5),
        b'6' => Some(6),
        b'7' => Some(7),
        b'8' => Some(8),
        b'9' => Some(9),
        b'a' | b'A' => Some(10),
        b'b' | b'B' => Some(11),
        b'c' | b'C' => Some(12),
        b'd' | b'D' => Some(13),
        b'e' | b'E' => Some(14),
        b'f' | b'F' => Some(15),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_byte_utf8_extra() {
        let bytes = vec![0x61, 0x62, 0x63];

        let uc1 = decode_utf8_char(&bytes);

        assert_eq!(uc1, Some(('a', 1)));

        let uc2 = decode_utf8_char(&bytes[1..]);

        assert_eq!(uc2, Some(('b', 1)));

        let uc3 = decode_utf8_char(&bytes[2..]);

        assert_eq!(uc3, Some(('c', 1)));
    }

    #[test]
    fn one_byte_utf8() {
        let bytes = vec![0x61, 0xff, 0x00, 0xff];

        let uc = decode_utf8_char(&bytes);

        assert_eq!(uc, Some(('a', 1)));
    }

    #[test]
    fn two_byte_utf8() {
        let bytes = vec![0xce, 0xb6, 0x00, 0xff];

        let uc = decode_utf8_char(&bytes);

        assert_eq!(uc, Some(('ζ', 2)));
    }

    #[test]
    fn three_byte_utf8() {
        let bytes = vec![0xe3, 0x8e, 0x94, 0x60];

        let uc = decode_utf8_char(&bytes);

        assert_eq!(uc, Some(('㎔', 3)));
    }

    #[test]
    fn four_byte_utf8() {
        let bytes = vec![0xf0, 0x9f, 0xab, 0xa5];

        let uc = decode_utf8_char(&bytes);

        assert_eq!(uc, Some(('🫥', 4)));
    }

    #[test]
    fn invalid_utf8() {
        let bytes = vec![0xff, 0xff, 0xff, 0xff];

        let uc = decode_utf8_char(&bytes);

        assert_eq!(uc, None);
    }

    #[test]
    fn empty_ascii_hex() {
        let bytes = Vec::new();

        let hex = ascii_to_hex(&bytes);

        assert_eq!(hex, None);
    }

    #[test]
    fn saturating_ascii_hex() {
        let bytes = vec![b'f', b'f', b'f', b'f', b'f', b'f', b'f', b'f'];

        let hex = ascii_to_hex(&bytes);

        assert_eq!(hex, Some(0xffff_ffff));
    }

    #[test]
    fn invalid_ascii_hex() {
        let bytes = vec![b'd', b'e', b'a', b'd', b'g'];

        let hex = ascii_to_hex(&bytes);

        assert_eq!(hex, None);
    }
}
