//! Dotkei compiler proc macros

use proc_macro::TokenStream;

mod symbol;

#[proc_macro]
pub fn symbols(input: TokenStream) -> TokenStream {
    symbol::make_symbols(input.into()).into()
}
