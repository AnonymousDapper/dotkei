// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use super::SyntaxKind;
use super::green::{GreenElement, GreenNode, GreenToken};

pub struct GreenBuilder {
    stack: Vec<(SyntaxKind, Vec<GreenElement>)>,
}

impl GreenBuilder {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn start_node(&mut self, kind: SyntaxKind) {
        self.stack.push((kind, Vec::new()));

        debug!(target:"start_node", "Starting node {:?}", kind);
    }

    pub fn token<S>(&mut self, kind: SyntaxKind, src: S)
    where
        S: Into<String>,
    {
        let tok = GreenElement::Token(GreenToken::new(kind, src));

        self.stack
            .last_mut()
            .expect("token called outside open node")
            .1
            .push(tok);
    }

    pub fn finish_node(&mut self) {
        let (kind, children) = self.stack.pop().expect("finish_node on empty stack");
        let node = GreenNode::new(kind, children);
        if let Some(parent) = self.stack.last_mut() {
            parent.1.push(GreenElement::Node(node));
        } else {
            self.stack.push((kind, vec![GreenElement::Node(node)]));
        }
    }

    pub fn finish(mut self) -> GreenNode {
        assert_eq!(self.stack.len(), 1, "unfinished nodes remain");

        let (_, mut children) = self.stack.pop().unwrap();

        match children.pop().unwrap() {
            GreenElement::Node(n) => n,
            GreenElement::Token(_) => panic!("root must be node"),
        }
    }

    pub fn checkpoint(&self) -> usize {
        self.stack
            .last()
            .map(|(_, children)| children.len())
            .unwrap_or(0)
    }

    pub fn wrap_checkpoint(&mut self, checkpoint: usize, kind: SyntaxKind) {
        let (_, children) = self.stack.last_mut().expect("wrap on empty stack");
        let tail: Vec<GreenElement> = children.drain(checkpoint..).collect();

        let node = GreenNode::new(kind, tail);
        children.push(GreenElement::Node(node));
    }
}
