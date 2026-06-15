// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::source::Source;

use crate::frontend::token::{Index, KeywordType, Span, Token, TokenKind};

use crate::cst::SyntaxKind;
use crate::cst::builder::GreenBuilder;
use crate::cst::green::GreenNode;

pub struct Parser<'src, S: Source> {
    source: &'src S,
    builder: GreenBuilder,
    tokens: Vec<Token>,
    doc_stack: Vec<(SyntaxKind, &'src str)>,
    last_indent: Option<&'src str>,
    length: Index,
    current: Index,
}

impl<'src, S: Source> Parser<'src, S> {
    pub fn new(tokens: Vec<Token>, source: &'src S) -> Parser<'src, S> {
        let length = tokens.len() as Index;

        Self {
            source,
            builder: GreenBuilder::new(),
            tokens,
            doc_stack: Vec::new(),
            last_indent: None,
            length,
            current: 0,
        }
    }

    // Parser internals
    // ----------------------------------------------------

    fn peek(&self) -> TokenKind {
        match self.tokens.get(self.current as usize) {
            Some(t) => t.kind,
            None => TokenKind::Eof,
        }
    }

    fn peek_next(&self) -> TokenKind {
        match self.tokens.get(self.current as usize + 1) {
            Some(t) => t.kind,
            None => TokenKind::Eof,
        }
    }

    fn advance(&mut self) -> TokenKind {
        let kind = self.peek();

        self.current += 1;

        kind
    }

    fn kind_at(&self, index: Index) -> TokenKind {
        match self.tokens.get(index as usize) {
            Some(t) => t.kind,
            None => TokenKind::Eof,
        }
    }

    fn current_token(&self) -> Token {
        self.tokens[self.current as usize]
    }

    fn current_text(&self) -> &'src str {
        self.source.read_text_span(self.current_token().span)
    }

    fn text_at(&self, span: Span) -> &'src str {
        self.source.read_text_span(span)
    }

    fn eat(&mut self, kind: SyntaxKind) {
        self.eat_trivia();

        let src = self.current_text();

        self.builder.token(kind, src);

        self.advance();
    }

    fn eat_if(&mut self, token: TokenKind, kind: SyntaxKind) -> bool {
        if self.peek() == token {
            self.eat(kind);
            return true;
        }
        false
    }

    fn eat_kw(&mut self, kw: KeywordType, kind: SyntaxKind) -> bool {
        if self.peek() == TokenKind::Keyword(kw) {
            self.eat(kind);
            return true;
        }
        false
    }

    fn try_collect_outer_doc(&mut self) {
        if !self.doc_stack.is_empty() {
            warn!(target:"collect_outer_doc", "doc stack not empty!");
        }

        while let TokenKind::OuterDoc = self.peek() {
            self.doc_stack
                .push((SyntaxKind::DocComment, self.current_text()));

            self.advance();

            trace!(target:"collect_outer_doc", "next: {:?}", self.peek());

            if let TokenKind::Newline = self.advance() {
                self.doc_stack.push((SyntaxKind::Newline, "\n"));
            }

            trace!(target:"collect_outer_doc", "current: {:?}", self.peek());
        }

        trace!(target:"collect_outer_doc", "Collected {} doc nodes", self.doc_stack.len());
    }

    fn try_collect_inner_doc(&mut self) {
        // if !self.doc_stack.is_empty() {
        //     warn!(target:"collect_inner_doc", "doc stack not empty!");
        // }

        while let TokenKind::InnerDoc = self.peek() {
            self.doc_stack
                .push((SyntaxKind::DocComment, self.current_text()));

            self.advance();

            trace!(target:"collect_inner_doc", "next: {:?}", self.peek());

            if let TokenKind::Newline = self.advance() {
                self.doc_stack.push((SyntaxKind::Newline, "\n"));
            }

            trace!(target:"collect_inner_doc", "current: {:?}", self.peek());
        }

        trace!(target:"collect_inner_doc", "Collected {} doc nodes", self.doc_stack.len());
    }

    fn emit_doc_comments(&mut self) {
        for (kind, src) in self.doc_stack.drain(..) {
            self.builder.token(kind, src);
        }
    }

    fn skip_comments(&mut self) {
        let mut n = 0;
        while let TokenKind::Comment = self.peek() {
            n += 1;

            self.eat(SyntaxKind::Comment);

            self.skip_newlines();
        }

        trace!(target:"skip_comments", "skipped {} comments", n);
    }

    fn eat_trivia(&mut self) {
        let tok = self.current_token();
        //trace!(target:"eat_trivia", "Eating trivia before {:?}", tok.kind);

        if tok.trivia_start < tok.span.start {
            let trivia_span = Span::new(tok.trivia_start, tok.span.start);

            self.builder
                .token(SyntaxKind::Whitespace, self.text_at(trivia_span));
        }
    }

    fn skip_newlines(&mut self) {
        let mut n = 0;
        while self.peek() == TokenKind::Newline {
            self.eat(SyntaxKind::Newline);
            n += 1;
        }

        trace!(target:"skip_newlines", "skipped {} newlines", n);
    }

    // Tree output
    // ----------------------------------------------------
    pub fn finish(self) -> GreenNode {
        self.builder.finish()
    }

    // Module root
    // ----------------------------------------------------

    // Module = Item*
    pub fn parse_module(&mut self) {
        self.builder.start_node(SyntaxKind::SourceFile);

        self.try_collect_inner_doc();
        self.emit_doc_comments();

        //self.skip_newlines();
        //self.skip_comments();

        while self.peek() != TokenKind::Eof {
            self.parse_item();
            //self.skip_newlines();
            //self.skip_comments();
        }

        self.builder.finish_node();
    }

    // Item construct
    // ----------------------------------------------------

    // Item = Visibility?
    //      ( UseItem
    //      | FnItem
    //      | TypeItem
    //      | DataItem
    //      | EnumItem
    //      | ConstItem
    //      | IfaceItem
    //      | ImplItem
    //      )
    fn parse_item(&mut self) {
        use KeywordType as KW;

        self.skip_newlines();
        self.skip_comments();
        self.try_collect_outer_doc();

        debug!(target:"item", "looking at item: {:?}", self.peek());

        let mut c = self.peek();
        if let TokenKind::Keyword(KW::Pub) = c {
            c = self.peek_next();
        }

        match c {
            TokenKind::Keyword(KW::Fn) => todo!(),
            TokenKind::Keyword(KW::Type) => todo!(),
            TokenKind::Keyword(KW::Data) => todo!(),
            TokenKind::Keyword(KW::Enum) => todo!(),
            TokenKind::Keyword(KW::Const) => todo!(),
            TokenKind::Keyword(KW::Interface) => todo!(),
            TokenKind::Keyword(KW::Operator) => todo!(),
            TokenKind::Keyword(KW::Use) => self.parse_use_item(),
            TokenKind::Keyword(KW::Impl) => todo!(),

            //TokenKind::Comment { doc: false } => {
            //    self.eat(SyntaxKind::Comment);
            //}
            //TokenKind::Comment { doc: true } => {
            //    self.eat(SyntaxKind::DocComment);
            //}
            TokenKind::Eof => {}
            t => todo!("error: unexpected item ({:?})", t),
        }
    }

    // Module use
    // ----------------------------------------------------

    // UseItem = `use` UseTree `;`
    fn parse_use_item(&mut self) {
        self.builder.start_node(SyntaxKind::UseDecl);
        if !self.doc_stack.is_empty() {
            self.emit_doc_comments();
        }

        self.eat_kw(KeywordType::Pub, SyntaxKind::KwPub);

        self.eat_kw(KeywordType::Use, SyntaxKind::KwUse);

        self.parse_use_tree();

        self.eat_if(TokenKind::Semi, SyntaxKind::TokSemi);

        self.builder.finish_node();
    }

    // UseTree = SimplePath
    //         ( `as` ( IDENT | `_` )
    //         | `with` `*`
    //         | `with` UseTree
    //         | `with` `[` UseTree ( `,` UseTree )* `]`
    //         )
    fn parse_use_tree(&mut self) {
        self.builder.start_node(SyntaxKind::UseTree);
        self.builder.start_node(SyntaxKind::UsePath);

        self.skip_newlines();
        self.skip_comments();

        self.parse_simple_path();

        self.builder.finish_node();

        let item_check = self.builder.checkpoint();

        let has_item = match self.peek() {
            TokenKind::Keyword(KeywordType::With) => {
                trace!(target:"parse_use_tree", "got with");
                self.eat(SyntaxKind::KwWith);

                match self.peek() {
                    TokenKind::Star => {
                        trace!(target:"parse_use_tree", " => glob");
                        self.eat(SyntaxKind::UseGlob);
                    }
                    TokenKind::LSquare => {
                        trace!(target:"parse_use_tree", " => list");
                        self.builder.start_node(SyntaxKind::UseList);

                        self.eat(SyntaxKind::TokLSquare);
                        //self.skip_newlines();

                        self.parse_use_tree();

                        while self.peek() == TokenKind::Comma {
                            self.eat(SyntaxKind::TokComma);

                            self.parse_use_tree();
                        }

                        self.skip_newlines();

                        self.eat_if(TokenKind::RSquare, SyntaxKind::TokRSquare);

                        self.builder.finish_node();
                    }
                    TokenKind::Ident => {
                        debug!(target:"parse_use_tree", "Nested at {:?}", self.peek());
                        self.parse_use_tree();
                    }
                    _ => todo!("error: expected wildcard or name list after with in use"),
                }

                true
            }
            TokenKind::Keyword(KeywordType::As) => {
                trace!(target:"parse_use_tree", "got as");
                self.builder.start_node(SyntaxKind::UseAlias);

                self.eat(SyntaxKind::KwAs);
                match self.peek() {
                    TokenKind::Ident => self.eat(SyntaxKind::Ident),
                    _ => todo!("error: expected ident after as in use"),
                }

                self.builder.finish_node();

                true
            }
            _ => false,
        };

        if has_item {
            debug!(target:"parse_use_tree", "\t|| wrapped to UseItem");
            self.builder
                .wrap_checkpoint(item_check, SyntaxKind::UseItem);
        }

        self.builder.finish_node();
    }

    // Namespace paths
    // ----------------------------------------------------

    // SimplePath = SimplePathSegment (`::` SimplePathSegment)*
    fn parse_simple_path(&mut self) {
        self.parse_simple_path_segment();

        while self.peek() == TokenKind::Fence {
            self.eat(SyntaxKind::TokFence);
            self.parse_simple_path_segment();
        }
    }

    // SimplePathSegment = IDENT | `super` | `self`
    fn parse_simple_path_segment(&mut self) {
        match self.peek() {
            TokenKind::Ident => self.eat(SyntaxKind::Ident),
            TokenKind::Keyword(KeywordType::SelfValue) => self.eat(SyntaxKind::Ident),
            TokenKind::Keyword(KeywordType::Super) => self.eat(SyntaxKind::Ident),
            t => todo!(
                "error: expected path segment ident / self / super ({:?})",
                t
            ),
        }
    }
}
