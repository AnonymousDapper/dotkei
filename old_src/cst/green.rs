// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::token::Index;

use super::SyntaxKind;

#[derive(Debug, Clone)]
pub struct GreenToken {
    pub kind: SyntaxKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct GreenNode {
    pub kind: SyntaxKind,
    pub children: Vec<GreenElement>,
    pub text_len: Index,
}

#[derive(Debug, Clone)]
pub enum GreenElement {
    Node(GreenNode),
    Token(GreenToken),
}

pub fn format_node(node: GreenNode, level: usize, filter_trivia: bool) -> String {
    let indent = " ".repeat(level * 2);

    let mut buf = format!("{}{:?}\n", indent, node.kind);

    for child in node.children.into_iter().filter(|e| {
        !filter_trivia || !matches!(e.kind(), SyntaxKind::Whitespace | SyntaxKind::Newline)
    }) {
        let sub_format = match child {
            GreenElement::Node(n) => format_node(n, level + 1, filter_trivia),
            GreenElement::Token(t) => {
                format!("{}  {:?} {:?}\n", indent, t.kind, t.text)
            }
        };

        buf.push_str(&sub_format);
    }

    //buf.push_str(&format!("{}]\n\n", indent));

    buf
}

impl GreenToken {
    pub fn new<S>(kind: SyntaxKind, text: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            kind,
            text: text.into(),
        }
    }

    pub fn len(&self) -> Index {
        self.text.len() as Index
    }
}

impl GreenNode {
    pub fn new(kind: SyntaxKind, children: Vec<GreenElement>) -> Self {
        let text_len = children.iter().map(GreenElement::text_len).sum();

        Self {
            kind,
            children,
            text_len,
        }
    }
}

impl GreenElement {
    pub fn text_len(&self) -> Index {
        match self {
            GreenElement::Node(n) => n.text_len,
            GreenElement::Token(t) => t.len(),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        match self {
            GreenElement::Node(n) => n.kind,
            GreenElement::Token(t) => t.kind,
        }
    }
}
