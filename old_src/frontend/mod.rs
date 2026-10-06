// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

pub mod lexing;
pub mod parsing;
pub mod reporting;
pub mod source;
pub mod text;
pub mod token;

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use lexing::Lexer;

use source::{Source, SourceFile};

use token::Token;

pub type FileId = u32;

pub struct SingleFrontend<'src> {
    source: &'src SourceFile,
    lexer: Lexer<'src, SourceFile>,
}

impl<'src> SingleFrontend<'src> {
    pub fn from_source(source: &'src SourceFile) -> SingleFrontend<'src> {
        let lexer = Lexer::new(source);

        SingleFrontend { source, lexer }
    }
}

pub fn lex_from_path(path: std::path::PathBuf) -> Result<Vec<(Token, String, String)>, String> {
    match SourceFile::from_path(path) {
        Ok(sf) => {
            let mut frontend = SingleFrontend::from_source(&sf);

            let mut toks = Vec::new();

            let tokens = frontend.lexer.collect_all();

            if frontend.lexer.has_errors() {
                let errs = frontend.lexer.get_errors();
                for e in errs {
                    reporting::print_syntax_error(e, &sf);
                }

                error!("{} syntax errors", errs.len());
            } else {
                //println!("{:#?}", tokens);
                let mut parser = parsing::Parser::new(tokens.clone(), &sf);

                parser.parse_module();

                //info!("{:?}", parser.finish());
                println!(
                    "{}",
                    crate::cst::green::format_node(parser.finish(), 0, true)
                );
            }

            for tok in tokens {
                let src = sf.read_text_span(tok.span).to_owned();

                let triv = sf
                    .read_text_span(token::Span::new(tok.trivia_start, tok.span.start))
                    .to_owned();
                toks.push((tok, src, triv));
            }

            Ok(toks)
        }
        Err(e) => Err(format!("error loading: {}", e)),
    }
}
