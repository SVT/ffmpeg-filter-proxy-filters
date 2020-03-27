// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::{error, fmt, io};

use resvg::usvg;
use roxmltree::{Document, Node, NodeType};

use super::{BoxResult, Transition};

pub(crate) fn parse_tsvg<R: io::Read>(mut source: R) -> BoxResult<Vec<Transition>> {
    let mut s = String::new();
    source.read_to_string(&mut s)?;

    let doc = Document::parse(&s)?;
    let root = doc.root_element();
    let root_name = root.tag_name().name();
    if root_name != "transitions" {
        return parse_error(format!("unexpected root element {}", root_name));
    }

    let mut transitions = Vec::new();
    for c in root.children() {
        let node_type = c.node_type();
        match node_type {
            NodeType::Text => {
                let text = c.text().unwrap().trim();
                if text.is_empty() {
                    continue;
                }

                return parse_error("unexpected text node");
            }

            NodeType::Element => {
                let name = c.tag_name().name();
                if name != "transition" {
                    return parse_error(format!("unexpected element {}", name));
                }

                transitions.push(parse_transition(&c)?);
            }

            _ => {
                return parse_error(format!("unexpected node type {:?}", node_type));
            }
        }
    }

    Ok(transitions)
}

fn parse_transition(node: &Node) -> BoxResult<Transition> {
    let attr = |n: &str| -> super::BoxResult<u64> {
        node.attribute(n)
            .map(|v| v.parse())
            .transpose()?
            .ok_or(Box::new(ParseError(format!("no {} in transition", n))))
    };

    let time_in = attr("time-in")?;
    let time_out = attr("time-out")?;
    let tree = parse_svg(node)?;
    Ok(Transition {
        time_in,
        time_out,
        tree,
    })
}

fn parse_svg(transition_node: &Node) -> BoxResult<usvg::Tree> {
    match transition_node.first_child() {
        None => {
            return parse_error("missing SVG data in transition");
        }

        Some(c) => {
            let node_type = c.node_type();
            if node_type != NodeType::Text {
                return parse_error(format!(
                    "unexpected {:?} child node in transition",
                    node_type
                ));
            }

            if c.has_siblings() {
                return parse_error("unexpeted multiple children in transition");
            }

            let text = c.text().unwrap().trim();
            if text.is_empty() {
                return parse_error("empty SVG data in transition");
            }

            let tree = usvg::Tree::from_str(text, &super::RESVG_OPTIONS.usvg)?;
            Ok(tree)
        }
    }
}

fn parse_error<T: Into<String>, U>(msg: T) -> BoxResult<U> {
    Err(Box::new(ParseError(msg.into())))
}

#[derive(Debug)]
struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl error::Error for ParseError {}