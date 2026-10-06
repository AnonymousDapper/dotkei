//! Dotkei symbol macros

#![allow(unused_imports, dead_code)]

use std::collections::HashMap;

use quote::quote;

use proc_macro2::{Span, TokenStream};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, Lit, LitStr, Macro, Token, braced};

mod kw {
    syn::custom_keyword!(Keywords);
    syn::custom_keyword!(Operators);
    syn::custom_keyword!(Symbols);
}

struct Keyword {
    name: Ident,
    value: LitStr,
}

impl Parse for Keyword {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value: LitStr = input.parse()?;

        Ok(Keyword { name, value })
    }
}

struct Operator {
    name: Ident,
    value: LitStr,
}

impl Parse for Operator {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value: LitStr = input.parse()?;

        Ok(Operator { name, value })
    }
}

struct Symbol {
    name: Ident,
    value: Value,
}

impl Parse for Symbol {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        let colon_tok: Option<Token![:]> = input.parse()?;
        let value = if colon_tok.is_some() {
            input.parse()?
        } else {
            Value::SameAsName
        };

        Ok(Symbol { name, value })
    }
}

enum Value {
    SameAsName,
    String(LitStr),
    Unsupported(Expr),
}

impl Parse for Value {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let expr: Expr = input.parse()?;
        if let Expr::Lit(expr) = &expr
            && let Lit::Str(lit) = &expr.lit
        {
            Ok(Value::String(lit.clone()))
        } else {
            Ok(Value::Unsupported(expr))
        }
    }
}

struct Input {
    keywords: Punctuated<Keyword, Token![,]>,
    operators: Punctuated<Operator, Token![,]>,
    symbols: Punctuated<Symbol, Token![,]>,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::Keywords>()?;
        let content;
        braced!(content in input);
        let keywords = Punctuated::parse_terminated(&content)?;

        input.parse::<kw::Operators>()?;
        let content;
        braced!(content in input);
        let operators = Punctuated::parse_terminated(&content)?;

        input.parse::<kw::Symbols>()?;
        let content;
        braced!(content in input);
        let symbols = Punctuated::parse_terminated(&content)?;

        Ok(Input {
            keywords,
            operators,
            symbols,
        })
    }
}

#[derive(Default)]
struct Errors {
    list: Vec<syn::Error>,
}

impl Errors {
    fn error(&mut self, span: Span, message: String) {
        self.list.push(syn::Error::new(span, message));
    }
}

struct Predefined {
    idx: u32,
    span_of_name: Span,
}

struct Entries {
    map: HashMap<String, Predefined>,
}

impl Entries {
    fn with_capacity(capacity: usize) -> Self {
        Entries {
            map: HashMap::with_capacity(capacity),
        }
    }

    fn insert(&mut self, span: Span, s: &str, errors: &mut Errors) -> u32 {
        if let Some(prev) = self.map.get(s) {
            errors.error(span, format!("Duplicated definition of symbol '{s}'"));
            errors.error(
                prev.span_of_name,
                "location of previous definition".to_string(),
            );
            prev.idx
        } else {
            let idx = self.len();
            self.map.insert(
                s.to_string(),
                Predefined {
                    idx,
                    span_of_name: span,
                },
            );
            idx
        }
    }

    fn len(&self) -> u32 {
        u32::try_from(self.map.len()).expect("too many symbols for u32")
    }
}

pub(super) fn make_symbols(input: TokenStream) -> TokenStream {
    let (mut output, errors) = make_symbols_with_errors(input);

    output.extend(errors.into_iter().map(|e| e.to_compile_error()));

    output
}

fn make_symbols_with_errors(input: TokenStream) -> (TokenStream, Vec<syn::Error>) {
    let mut errors = Errors::default();

    let input: Input = match syn::parse2(input) {
        Ok(input) => input,
        Err(e) => {
            errors.list.push(e);
            Input {
                keywords: Default::default(),
                operators: Default::default(),
                symbols: Default::default(),
            }
        }
    };

    let mut keywords_stream = quote! {};
    let mut operators_stream = quote! {};
    let mut symbols_stream = quote! {};
    let mut prefill_stream = quote! {};

    let prefill_ints = 0..=9;
    let prefill_letters = ('A'..='Z').chain('a'..='z');
    let mut entries = Entries::with_capacity(
        input.keywords.len()
            + input.operators.len()
            + input.symbols.len()
            + prefill_ints.clone().count()
            + prefill_letters.clone().count(),
    );

    for keyword in input.keywords.iter() {
        let name = &keyword.name;
        let value = &keyword.value;
        let value_str = value.value();
        let idx = entries.insert(keyword.name.span(), &value_str, &mut errors);
        prefill_stream.extend(quote! {
            #value,
        });
        keywords_stream.extend(quote! {
            pub const #name: Symbol = Symbol::new(#idx);
        });
    }

    for operator in input.operators.iter() {
        let name = &operator.name;
        let value = &operator.value;
        let value_str = value.value();
        let idx = entries.insert(operator.name.span(), &value_str, &mut errors);
        prefill_stream.extend(quote! {
            #value,
        });
        operators_stream.extend(quote! {
            pub const #name: Symbol = Symbol::new(#idx);
        });
    }

    for symbol in input.symbols.iter() {
        let name = &symbol.name;
        let value = match &symbol.value {
            Value::SameAsName => name.to_string(),
            Value::String(lit) => lit.value(),
            Value::Unsupported(expr) => {
                errors.list.push(syn::Error::new_spanned(
                    expr,
                    "unsupported expression for symbol",
                ));
                continue;
            }
        };

        let idx = entries.insert(symbol.name.span(), &value, &mut errors);
        prefill_stream.extend(quote! {
            #value,
        });
        symbols_stream.extend(quote! {
            pub const #name: Symbol = Symbol::new(#idx);
        });
    }

    for s in prefill_ints
        .map(|n| n.to_string())
        .chain(prefill_letters.map(|c| c.to_string()))
    {
        entries.insert(Span::call_site(), &s, &mut errors);
        prefill_stream.extend(quote! {
            #s,
        });
    }

    let symbol_digits_base = entries.map["0"].idx;
    let symbol_uppercase_base = entries.map["A"].idx;
    let symbol_lowercase_base = entries.map["a"].idx;
    let predefined_symbols_count = entries.len();

    let output = quote! {
        const SYMBOL_DIGITS_BASE: u32 = #symbol_digits_base;
        const SYMBOL_UPPERCASE_BASE: u32 = #symbol_uppercase_base;
        const SYMBOL_LOWERCASE_BASE: u32 = #symbol_lowercase_base;

        pub const PREDEFINED_SYMBOL_COUNT: u32 = #predefined_symbols_count;

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        mod kw_generated {
            use super::Symbol;
            #keywords_stream
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        mod op_generated {
            use super::Symbol;
            #operators_stream
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub mod sym_generated {
            use super::Symbol;
            #symbols_stream
        }

        impl Interner {
            pub(crate) fn with_extra_symbols(extra_symbols: &[&'static str]) -> Self {
                Interner::prefill(&[#prefill_stream], extra_symbols)
            }
        }
    };

    (output, errors.list)
}
