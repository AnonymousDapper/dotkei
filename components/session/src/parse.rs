// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

use symbol::{Interner, Symbol};

use structures::FxIndexMap;

/// temp placeholder
pub struct Span;

#[derive(Default)]
pub struct SymbolGallery {
    pub symbols: FxIndexMap<Symbol, Span>,
}

impl SymbolGallery {
    pub fn insert(&mut self, symbol: Symbol, span: Span) {
        self.symbols.entry(symbol).or_insert(span);
    }
}

pub struct ParseSess {
    // diag context
    // source map
    interner: Interner,
    gallery: SymbolGallery
}
