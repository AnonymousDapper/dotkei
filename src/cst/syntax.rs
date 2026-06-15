// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::token::{Index, Span};

use super::green::{GreenElement, GreenNode, GreenToken};

use super::SyntaxKind;

#[derive(Debug, Clone, Copy)]
pub struct SyntaxNode<'g> {
    green: &'g GreenNode,
    //parent: &'g SyntaxNode<'g>,
    pub offset: Index,
}

#[derive(Debug, Clone, Copy)]
pub struct SyntaxToken<'g> {
    green: &'g GreenToken,
    pub offset: Index,
}

#[derive(Debug, Clone, Copy)]
pub enum SyntaxElement<'g> {
    Node(SyntaxNode<'g>),
    Token(SyntaxToken<'g>),
}

impl<'g> SyntaxNode<'g> {
    pub fn new(green: &'g GreenNode) -> Self {
        Self { green, offset: 0 }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.green.kind
    }

    pub fn text_len(&self) -> Index {
        self.green.text_len
    }

    pub fn span(&self) -> Span {
        Span::new(self.offset, self.offset + self.green.text_len)
    }

    pub fn children(&self) -> impl Iterator<Item = SyntaxElement<'g>> {
        let mut child_offset = self.offset;

        self.green.children.iter().map(move |child| {
            let at = child_offset;
            child_offset += child.text_len();

            match child {
                GreenElement::Node(n) => SyntaxElement::Node(SyntaxNode {
                    green: n,
                    offset: at,
                }),
                GreenElement::Token(t) => SyntaxElement::Token(SyntaxToken {
                    green: t,
                    offset: at,
                }),
            }
        })
    }

    pub fn nontrivial_children(&self) -> impl Iterator<Item = SyntaxElement<'g>> {
        self.children().filter(|e| !e.is_trivia())
    }
}

impl<'g> SyntaxElement<'g> {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            SyntaxElement::Node(n) => n.kind(),
            SyntaxElement::Token(t) => t.green.kind,
        }
    }

    pub fn is_trivia(&self) -> bool {
        matches!(self.kind(), SyntaxKind::Whitespace | SyntaxKind::Comment)
    }
}
