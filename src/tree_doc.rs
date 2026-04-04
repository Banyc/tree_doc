use bon::builder;
use facet::Facet;
use std::collections::HashMap;
use winnow::prelude::*;
use winnow::stream::{LocatingSlice, Location};

use crate::part::{ComplexKind, ItemInfo, Kind, NodeInfo, NodeName, ParsedLine, Span, parse_line};

leanward::nest! {
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub span: Span,
    pub kind_special:
        #[derive(Debug, Clone, PartialEq)]
        pub enum NodeKindSpecial {
            Leaf,
            Complex(
                #[derive(Debug, Clone, PartialEq)]
                pub enum NodeComplexKindSpecial {
                    Struct { children: HashMap<String, Node> },
                    Array { child_fragments: Vec<Node> },
                    Map { child_fragments: Vec<Node> },
                },
            ),
        },
}}

#[derive(Debug, Clone, PartialEq)]
pub struct RawNode {
    pub info: NodeInfo,
    pub children: Vec<RawNode>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    ParseError { span: Span, msg: String },
    ValidateError { span: Span, msg: String },
}

pub type Result<T> = core::result::Result<T, Error>;

/// ```
/// use tree_doc::{parse, validate};
/// #[derive(Debug, facet::Facet)]
/// struct Config { servers: std::collections::HashMap<String, Server> }
/// #[derive(Debug, facet::Facet)]
/// struct Server { host: String }
/// let input = r#"
/// - root { type: "struct" }
///   - servers { type: "map", name: "servers" }
/// "#;
/// let node = parse(input).unwrap();
/// assert!(validate::<Config>(&node).is_ok());
/// ```
pub fn validate<T: Facet<'static>>(tree_doc: &Node) -> Result<()> {
    fn validate_node(node: &Node, shape: &'static facet::Shape) -> Result<()> {
        match &node.kind_special {
            NodeKindSpecial::Leaf => Ok(()),
            NodeKindSpecial::Complex(NodeComplexKindSpecial::Struct { .. }) => {
                validate_struct_fields(node, shape)
            }
            NodeKindSpecial::Complex(NodeComplexKindSpecial::Array { child_fragments }) => {
                let facet::Def::List(list_def) = shape.def else {
                    return Err(Error::ValidateError {
                        span: node.span,
                        msg: format!("Expected array-like type, got {:?}", shape.def),
                    });
                };
                for fragment in child_fragments {
                    validate_node(fragment, list_def.t)?;
                }
                Ok(())
            }
            NodeKindSpecial::Complex(NodeComplexKindSpecial::Map { child_fragments }) => {
                let facet::Def::Map(map_def) = shape.def else {
                    return Err(Error::ValidateError {
                        span: node.span,
                        msg: format!("Expected map-like type, got {:?}", shape.def),
                    });
                };
                for fragment in child_fragments {
                    validate_node(fragment, map_def.v)?;
                }
                Ok(())
            }
        }
    }

    fn validate_struct_fields(node: &Node, shape: &'static facet::Shape) -> Result<()> {
        let struct_type = match shape.ty {
            facet::Type::User(facet::UserType::Struct(st)) => st,
            _ => {
                return Err(Error::ValidateError {
                    span: node.span,
                    msg: format!("Expected struct, got {:?}", shape.ty),
                });
            }
        };

        let field_map: HashMap<&str, &facet::Field> =
            struct_type.fields.iter().map(|f| (f.name, f)).collect();

        let children = match &node.kind_special {
            NodeKindSpecial::Complex(NodeComplexKindSpecial::Struct { children }) => children,
            _ => {
                return Err(Error::ValidateError {
                    span: node.span,
                    msg: "Expected struct node".to_string(),
                });
            }
        };

        for (child_name, child) in children {
            let field = field_map
                .get(child_name.as_str())
                .ok_or(Error::ValidateError {
                    span: node.span,
                    msg: format!("Field '{child_name}' not found in struct"),
                })?;
            validate_node(child, field.shape())?;
        }

        Ok(())
    }
    let shape = <T as Facet<'static>>::SHAPE;
    validate_struct_fields(tree_doc, shape)
}

pub fn parse(input: &str) -> Result<Node> {
    let raw = parse_raw(input)?;
    convert_raw(raw)
}

pub fn parse_raw(input: &str) -> Result<RawNode> {
    struct StackedNode {
        indent: usize,
        node: RawNode,
    }
    impl StackedNode {
        fn is_same_level_or_deeper(&self, than: usize) -> bool {
            self.indent >= than
        }
    }
    let mut shallow_to_deep_pending_nodes: Vec<StackedNode> = Vec::new();
    let mut offset = 0;
    let mut last_leaf_indent: Option<usize> = None;

    while offset < input.len() {
        let rest = &input[offset..];
        let found_newline = rest.find('\n');
        let line = match found_newline {
            Some(pos) => &rest[..pos],
            None => rest,
        };
        offset += line.len() + if found_newline.is_some() { 1 } else { 0 };

        let is_blank_line = line.trim().is_empty();
        if is_blank_line {
            continue;
        }

        let line_start_offset = input.len() - rest.len();
        let mut line_stream = LocatingSlice::new(line);
        let parsed = parse_line.parse_next(&mut line_stream).map_err(|e| {
            let err_offset = line_start_offset + line_stream.current_token_start();
            Error::ParseError {
                span: Span::from_offset(input, err_offset),
                msg: e.to_string(),
            }
        })?;

        let ParsedLine { indent, info } = parsed;
        let info = match info {
            ItemInfo::Bare => {
                last_leaf_indent = Some(indent);
                continue;
            }
            ItemInfo::Described(info) => info,
        };
        #[rustfmt::skip]
        check_disconnected_node().last_leaf_indent(&mut last_leaf_indent).indent(indent).input(input).line_start_offset(line_start_offset).line(line).call()?;

        while shallow_to_deep_pending_nodes
            .last()
            .is_some_and(|stacked| stacked.is_same_level_or_deeper(indent))
        {
            let child = shallow_to_deep_pending_nodes.pop().unwrap().node;
            let parent = shallow_to_deep_pending_nodes.last_mut().unwrap();
            parent.node.children.push(child);
        }

        let column = line.len() - line.trim_start().len() + 1;
        let line = input[..line_start_offset].lines().count() + 1;
        let span = Span { line, column };
        let node = RawNode {
            info,
            children: Vec::new(),
            span,
        };
        shallow_to_deep_pending_nodes.push(StackedNode { indent, node });
    }

    while shallow_to_deep_pending_nodes.len() > 1 {
        let child = shallow_to_deep_pending_nodes.pop().unwrap().node;
        let parent = shallow_to_deep_pending_nodes.last_mut().unwrap();
        parent.node.children.push(child);
    }

    shallow_to_deep_pending_nodes
        .into_iter()
        .next()
        .map(|stacked| stacked.node)
        .ok_or_else(|| Error::ParseError {
            span: Span { line: 1, column: 1 },
            msg: "No root node found".to_string(),
        })
}

fn convert_raw(raw: RawNode) -> Result<Node> {
    let kind = match raw.info.kind {
        Kind::Leaf => {
            if !raw.children.is_empty() {
                return Err(Error::ValidateError {
                    span: raw.span,
                    msg: "Leaf node should not have children".to_string(),
                });
            }
            return {
                Ok(Node {
                    span: raw.span,
                    kind_special: NodeKindSpecial::Leaf,
                })
            };
        }
        Kind::Complex(complex_kind) => complex_kind,
    };
    let convert_children = |raw: RawNode| {
        raw.children
            .into_iter()
            .map(convert_raw)
            .collect::<Result<Vec<Node>>>()
    };
    match kind {
        ComplexKind::Struct => {
            let fields: HashMap<String, Node> = collect_struct_fields(raw.children.into_iter())?;
            Ok(Node {
                span: raw.span,
                kind_special: NodeKindSpecial::Complex(NodeComplexKindSpecial::Struct {
                    children: fields,
                }),
            })
        }
        ComplexKind::Array => {
            let span = raw.span;
            let child_fragments = convert_children(raw)?;
            let kind_special =
                NodeKindSpecial::Complex(NodeComplexKindSpecial::Array { child_fragments });
            Ok(Node { span, kind_special })
        }
        ComplexKind::Map => {
            let span = raw.span;
            let child_fragments = convert_children(raw)?;
            let kind_special =
                NodeKindSpecial::Complex(NodeComplexKindSpecial::Map { child_fragments });
            Ok(Node { span, kind_special })
        }
    }
}

fn collect_struct_fields(fields: impl Iterator<Item = RawNode>) -> Result<HashMap<String, Node>> {
    fields
        .map(|mut field| {
            let name = match std::mem::replace(&mut field.info.name, NodeName::Anonymous) {
                NodeName::AsField(s) => s,
                NodeName::Anonymous => {
                    return Err(Error::ValidateError {
                        span: field.span,
                        msg: "struct field must be named".to_string(),
                    });
                }
            };
            let node = convert_raw(field)?;
            Ok((name, node))
        })
        .collect::<Result<_>>()
}

#[builder]
fn check_disconnected_node(
    last_leaf_indent: &mut Option<usize>,
    indent: usize,
    input: &str,
    line_start_offset: usize,
    line: &str,
) -> Result<()> {
    let is_deeper_than_leaf = last_leaf_indent.is_some_and(|leaf| indent > leaf);
    if is_deeper_than_leaf {
        let column = line.len() - line.trim_start().len() + 1;
        return Err(Error::ParseError {
            span: Span {
                line: input[..line_start_offset].lines().count() + 1,
                column,
            },
            msg: "Metadata block after leaf content".to_string(),
        });
    }
    *last_leaf_indent = None;
    Ok(())
}

#[cfg(test)] #[test] #[rustfmt::skip] fn test_parse_merge() {
    let tc = r#"
- list { type: "array" }
  - a { type: "struct" }
    - a { name: "a" }
  - b { type: "struct" }
    - b { name: "b" }
  - this is just a comment; ignored
    - a comment as well"#;
    let node = parse(tc).unwrap();
    let grand_children = |frag_i: usize| {
        let child = match &node.kind_special {
            NodeKindSpecial::Complex(NodeComplexKindSpecial::Array { child_fragments }) => &child_fragments[frag_i],
            _ => panic!(),
        };
        match &child.kind_special {
            NodeKindSpecial::Complex(NodeComplexKindSpecial::Struct { children }) => children,
            _ => panic!(),
        }
    };
    assert2::assert!(grand_children(0).get("a").is_some());
    assert2::assert!(grand_children(1).get("b").is_some());
}

#[cfg(test)] #[test] #[rustfmt::skip] fn test_parse_bad() {
    let inputs = [
        r#"- a { type: "struct" }
              - b
                - c { type: "struct", name: "c" }"#,
    ];
    for tc in inputs {
        assert2::check!(parse(tc).is_err());
    }
}
