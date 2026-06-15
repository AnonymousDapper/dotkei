// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

use string_interner::{StringInterner, backend::StringBackend, symbol::SymbolU16};

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::token::Index;

pub type SymbolId = SymbolU16;

pub type SymbolStorage = StringInterner<StringBackend<SymbolId>>;

pub type StringId = SymbolU16;

pub type StringStorage = StringInterner<StringBackend<StringId>>;

pub type OperatorId = SymbolU16;

pub type OperatorStorage = StringInterner<StringBackend<OperatorId>>;

pub struct InternedStorage {
    symbols: SymbolStorage,
    strings: StringStorage,
    operators: OperatorStorage,
}

impl InternedStorage {
    pub fn new() -> Self {
        Self {
            symbols: SymbolStorage::new(),
            strings: StringStorage::new(),
            operators: OperatorStorage::new(),
        }
    }

    pub fn store_symbol<T: AsRef<str>>(&mut self, symbol: T) -> SymbolId {
        self.symbols.get_or_intern(symbol)
    }

    pub fn check_symbol<T: AsRef<str>>(&self, symbol: T) -> Option<SymbolId> {
        self.symbols.get(symbol)
    }

    pub fn resolve_symbol(&self, id: SymbolId) -> Option<&str> {
        self.symbols.resolve(id)
    }

    pub fn store_string<T: AsRef<str>>(&mut self, string: T) -> StringId {
        self.strings.get_or_intern(string)
    }

    pub fn check_string<T: AsRef<str>>(&self, string: T) -> Option<StringId> {
        self.strings.get(string)
    }

    pub fn resolve_string(&self, id: StringId) -> Option<&str> {
        self.strings.resolve(id)
    }

    pub fn store_operator<T: AsRef<str>>(&mut self, oper: T) -> OperatorId {
        self.operators.get_or_intern(oper)
    }

    pub fn check_operator<T: AsRef<str>>(&self, oper: T) -> Option<OperatorId> {
        self.operators.get(oper)
    }

    pub fn resolve_operator(&self, id: OperatorId) -> Option<&str> {
        self.operators.resolve(id)
    }
}

pub enum LiteralType {
    String(String),
    Bytes(Vec<u8>),
    Char(char),
    Byte(u8),
    //Integer(u64),
    //Float(f64),
}

impl LiteralType {
    pub fn format_value(&self) -> String {
        match self {
            LiteralType::String(s) => s.to_owned(),
            LiteralType::Bytes(v) => {
                format!("{:?}", v)
            }
            LiteralType::Char(c) => {
                format!("{:?}", c)
            }
            LiteralType::Byte(b) => {
                format!("{:?}", b)
            }
        }
    }
}

pub struct LiteralStorage {
    literals: Vec<LiteralType>,
    indices: Vec<Index>,
}

impl LiteralStorage {
    pub fn new() -> Self {
        Self {
            literals: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn store_literal(&mut self, literal: LiteralType, source_index: Index) {
        self.literals.push(literal);
        self.indices.push(source_index);
    }

    pub fn get_literal(&self, id: Index) -> Option<&LiteralType> {
        let literal_idx = match self.indices.binary_search(&id) {
            Ok(i) => i,
            Err(_) => return None,
        };

        self.literals.get(literal_idx)
    }
}
