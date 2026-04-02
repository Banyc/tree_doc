use facet::Facet;
use std::collections::HashMap;
use winnow::ascii::space0;
use winnow::combinator::{delimited, opt, separated, terminated};
use winnow::error::ModalResult;
use winnow::prelude::*;
use winnow::stream::{LocatingSlice, Location};
use winnow::token::{take_till, take_while};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    fn from_offset(input: &str, byte_offset: usize) -> Self {
        let up_to = &input[..byte_offset];
        let line = up_to.lines().count();
        let column = up_to.lines().last().map(|l| l.len()).unwrap_or(0) + 1;
        Span { line, column }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: Option<String>,
    pub kind: Kind,
    pub children: Vec<Node>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Struct,
    Array,
    Map,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    ParseError { span: Span, msg: String },
    ValidateError { span: Span, msg: String },
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct ParsedLine {
    pub indent: usize,
    pub name: Option<String>,
    pub kind: Kind,
}

type Input<'i> = LocatingSlice<&'i str>;

/// ```
/// use winnow::stream::LocatingSlice;
/// let mut input = LocatingSlice::new(r#"- servers { type: "map", name: "servers" }"#);
/// let result = tree_doc::parse_line(&mut input).unwrap().unwrap();
/// assert2::check!(result.indent == 0);
/// assert2::check!(result.name == Some("servers".to_string()));
/// assert2::check!(result.kind == tree_doc::Kind::Map);
/// let mut input2 = LocatingSlice::new(r#"- host"#);
/// let result2 = tree_doc::parse_line(&mut input2).unwrap();
/// assert2::check!(result2.is_none());
/// ```
pub fn parse_line(input: &mut Input<'_>) -> ModalResult<Option<ParsedLine>> {
    let start = input.checkpoint();
    let indent = take_while(0.., ' ')
        .map(|s: &str| s.len() / 2)
        .parse_next(input)?;
    ("-", space0).parse_next(input)?;
    take_till(0.., '{').parse_next(input)?;
    let kind_and_name = opt(parse_meta_block).parse_next(input)?;
    let Some((kind, name)) = kind_and_name else {
        input.reset(&start);
        return Ok(None);
    };
    space0.parse_next(input)?;
    Ok(Some(ParsedLine { indent, name, kind }))
}
#[cfg(test)] #[test] #[rustfmt::skip]
fn test_parse_line() {
    leanward::local! {
    let tc = struct Case<'a> {
        raw:    &'a str          = r#"- servers { type: "map", name: "servers" }"#,
        some:   bool             = true,
        indent: usize            = 0,
        name:   Option<&'a str>  = Some("servers"),
        kind:   Kind             = Kind::Map,
    };}
    let inputs = [
        tc,
        Case { raw: r#"- root { type: "struct" }"#,                  some: true,  indent: 0, name: None,          kind: Kind::Struct },
        Case { raw: r#"  - child { type: "array", name: "items" }"#, some: true,  indent: 1, name: Some("items"), kind: Kind::Array },
        Case { raw: r#"- host"#,                                     some: false, indent: 0, name: None,          kind: Kind::Struct },
    ];
    for tc in inputs {
        let mut input = LocatingSlice::new(tc.raw);
        let result = parse_line(&mut input).unwrap();
        assert2::check!(result.is_some() == tc.some, "input: {}", tc.raw);
        if let Some(p) = result {
            assert2::check!(p.indent == tc.indent, "input: {}", tc.raw);
            assert2::check!(p.name == tc.name.map(|s| s.to_string()), "input: {}", tc.raw);
            assert2::check!(p.kind == tc.kind, "input: {}", tc.raw);
        }
    }
}

/// ```
/// use winnow::stream::LocatingSlice;
/// let mut input = LocatingSlice::new(r#"{ type: "array", name: "crates" }"#);
/// let (kind, name) = tree_doc::parse_meta_block(&mut input).unwrap();
/// assert_eq!(kind, tree_doc::Kind::Array);
/// assert_eq!(name, Some("crates".to_string()));
/// let mut input2 = LocatingSlice::new(r#"{ type: "struct" }"#);
/// let (kind2, name2) = tree_doc::parse_meta_block(&mut input2).unwrap();
/// assert_eq!(kind2, tree_doc::Kind::Struct);
/// assert!(name2.is_none());
/// ```
pub fn parse_meta_block(input: &mut Input<'_>) -> ModalResult<(Kind, Option<String>)> {
    let content = terminated(separated(0.., parse_kv_pair, ","), space0);
    let pairs: Vec<(String, String)> = delimited("{", content, "}").parse_next(input)?;
    let kind = match pairs
        .iter()
        .find(|(k, _)| k == "type")
        .map(|(_, v)| v.as_str())
    {
        Some("array") => Kind::Array,
        Some("map") => Kind::Map,
        _ => Kind::Struct,
    };
    let name = pairs
        .iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.clone());
    Ok((kind, name))
}
#[cfg(test)] #[test] #[rustfmt::skip]
fn test_parse_meta_block() {
    leanward::local! {
    let tc = struct Case<'a> {
        raw:      &'a str          = r#"{ type: "array", name: "crates" }"#,
        kind:     Kind             = Kind::Array,
        name:     Option<&'a str>  = Some("crates"),
    };}
    let inputs = [
        tc,
        Case { raw: r#"{ type: "map", name: "servers" }"#, kind: Kind::Map, name: Some("servers") },
        Case { raw: r#"{ type: "struct" }"#, kind: Kind::Struct, name: None },
        Case { raw: r#"{ name: "foo", type: "array" }"#, kind: Kind::Array, name: Some("foo") },
    ];
    for tc in inputs {
        let mut input = LocatingSlice::new(tc.raw);
        let (kind, name) = parse_meta_block(&mut input).unwrap();
        assert2::check!(kind == tc.kind, "input: {}", tc.raw);
        assert2::check!(name == tc.name.map(|s| s.to_string()), "input: {}", tc.raw);
    }
}

/// ```
/// use winnow::stream::LocatingSlice;
/// let mut input = LocatingSlice::new(r#"type: "struct""#);
/// let (k, v) = tree_doc::parse_kv_pair(&mut input).unwrap();
/// assert_eq!(k, "type");
/// assert_eq!(v, "struct");
/// ```
pub fn parse_kv_pair(input: &mut Input<'_>) -> ModalResult<(String, String)> {
    space0.parse_next(input)?;
    let key: &str = take_till(1.., ':').parse_next(input)?;
    (":", space0).parse_next(input)?;
    let value = parse_quoted_string.parse_next(input)?;
    Ok((key.trim().to_string(), value))
}
#[cfg(test)] #[test] #[rustfmt::skip]
fn test_parse_kv_pair() {
    leanward::local! {
    let tc = struct Case<'a> {
        raw:     &'a str  = r#"type: "struct""#,
        key:     &'a str  = "type",
        val:     &'a str  = "struct",
    };}
    let inputs = [
        tc,
        Case { raw: r#"name: "servers""#,   key: "name", val: "servers" },
        Case { raw: r#"  type: "array"  "#, key: "type", val: "array" },
    ];
    for tc in inputs {
        let mut input = LocatingSlice::new(tc.raw);
        let (k, v) = parse_kv_pair(&mut input).unwrap();
        assert2::check!(k == tc.key, "input: {}", tc.raw);
        assert2::check!(v == tc.val, "input: {}", tc.raw);
    }
}

/// ```
/// use winnow::stream::LocatingSlice;
/// let mut input = LocatingSlice::new(r#""hello world""#);
/// let s = tree_doc::parse_quoted_string(&mut input).unwrap();
/// assert_eq!(s, "hello world");
/// ```
pub fn parse_quoted_string(input: &mut Input<'_>) -> ModalResult<String> {
    '\"'.parse_next(input)?;
    let mut result = String::new();
    loop {
        let checkpoint = input.checkpoint();
        if input.is_empty() {
            input.reset(&checkpoint);
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
        let ch = input.next_token();
        match ch {
            Some('"') => return Ok(result),
            Some('\\') => {
                if let Some(next) = input.next_token() {
                    if next == '"' {
                        result.push('"');
                    } else {
                        result.push('\\');
                        result.push(next);
                    }
                } else {
                    result.push('\\');
                }
            }
            Some(c) => result.push(c),
            None => {
                input.reset(&checkpoint);
                return Err(winnow::error::ErrMode::Backtrack(
                    winnow::error::ContextError::new(),
                ));
            }
        }
    }
}
#[cfg(test)] #[test] #[rustfmt::skip]
fn test_parse_quoted_string() {
    leanward::local! {
    let tc = struct Case<'a> {
        raw: &'a str  = r#""hello world""#,
        exp: &'a str  = "hello world",
    };}
    #[rustfmt::skip]
    let inputs = [
        tc,
        Case { raw: r#""""#, exp: "" },
        Case { raw: r#""path/to/crate.a""#, exp: "path/to/crate.a" },
        Case { raw: r#""v3""#, exp: "v3" },
    ];
    for tc in inputs {
        let mut input = LocatingSlice::new(tc.raw);
        let s = parse_quoted_string(&mut input).unwrap();
        assert2::check!(s == tc.exp, "input: {}", tc.raw);
    }
}
#[cfg(test)] #[test] #[rustfmt::skip]
fn test_parse_quoted_string_nasty() {
    leanward::local! {
    let tc = struct Case<'a> {
        raw:  &'a str  = r#"""trailing"#,
        exp:  &'a str  = "",
        rest: &'a str  = "trailing",
    };}
    let inputs = [
        tc,
        Case { raw: r#""a"b""#,           exp: "a",   rest: r#"b""#        },
        Case { raw: r#""foo"bar"baz""#,   exp: "foo", rest: r#"bar"baz""#  },
        Case { raw: r#""  spaces  ""#,    exp: "  spaces  ", rest: ""      },
        Case { raw: r#""a""#,             exp: "a",   rest: ""             },
        Case { raw: r#""{ type: ""struct"" }""#, exp: "{ type: ", rest: r#""struct"" }""# },
        Case { raw: r#""colons:here:and:here""#, exp: "colons:here:and:here", rest: "" },
        Case { raw: r#""commas,and,stuff""#, exp: "commas,and,stuff", rest: "" },
        Case { raw: r#""日本語""#,         exp: "日本語", rest: ""          },
        Case { raw: r#""\n\t""#,          exp: r#"\n\t"#, rest: ""         },
        Case { raw: r#""   ""#,           exp: "   ", rest: ""             },
        Case { raw: r#""{}[]()<>""#,      exp: "{}[]()<>", rest: ""        },
        Case { raw: r#""a""b""c""#,       exp: "a",   rest: r#""b""c""#    },
        Case { raw: r#""""""""#,          exp: "",    rest: r#""""""#      },
        Case { raw: r#""hello"world"#,    exp: "hello", rest: "world"      },
        Case { raw: r#""\"hello"world"#,  exp: "\"hello", rest: "world"    },
        Case { raw: r#""hello\""world"#,  exp: "hello\"", rest: "world"    },
        Case { raw: r#""""#,             exp: "",    rest: ""             },
        Case { raw: r#""  ""#,            exp: "  ",  rest: ""             },
        Case { raw: r#""a b c d e f g""#, exp: "a b c d e f g", rest: ""  },
    ];
    for tc in inputs {
        let mut input = LocatingSlice::new(tc.raw);
        let s = parse_quoted_string(&mut input).unwrap();
        assert2::check!(s == tc.exp, "input: {}", tc.raw);
        assert2::check!(*input == tc.rest, "input: {}", tc.raw);
    }
    let bad_inputs: &[&str] = &[r#"no quotes"#, r#""unclosed"#, "", r#"""#];
    for raw in bad_inputs {
        let mut input = LocatingSlice::new(*raw);
        assert2::check!(
            parse_quoted_string(&mut input).is_err(),
            "should fail: {raw}"
        );
    }
}

/// ```
/// use tree_doc::{parse, validate, Kind};
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
    fn validate_node(tree_node: &Node, shape: &'static facet::Shape) -> Result<()> {
        let struct_type = match shape.ty {
            facet::Type::User(facet::UserType::Struct(st)) => st,
            _ => {
                return Err(Error::ValidateError {
                    span: tree_node.span,
                    msg: format!("Expected struct, got {:?}", shape.ty),
                });
            }
        };
        let field_map: HashMap<&str, &facet::Field> =
            struct_type.fields.iter().map(|f| (f.name, f)).collect();
        for child in &tree_node.children {
            let child_name = child.name.as_deref().ok_or(Error::ValidateError {
                span: child.span,
                msg: "Child node missing name".to_string(),
            })?;
            let field = field_map.get(child_name).ok_or(Error::ValidateError {
                span: child.span,
                msg: format!("Field '{}' not found in struct", child_name),
            })?;
            let child_shape = field.shape();
            match child.kind {
                Kind::Struct => {
                    if !matches!(
                        child_shape.ty,
                        facet::Type::User(facet::UserType::Struct(_))
                    ) {
                        return Err(Error::ValidateError {
                            span: child.span,
                            msg: format!("Field '{}' should be struct", child_name),
                        });
                    }
                    validate_node(child, child_shape)?;
                }
                Kind::Array => {
                    let type_name = child_shape.type_identifier;
                    if !type_name.contains("Vec") {
                        return Err(Error::ValidateError {
                            span: child.span,
                            msg: format!(
                                "Field '{}' should be Vec-like, got {}",
                                child_name, type_name
                            ),
                        });
                    }
                }
                Kind::Map => {
                    let type_name = child_shape.type_identifier;
                    if !type_name.contains("Map") {
                        return Err(Error::ValidateError {
                            span: child.span,
                            msg: format!(
                                "Field '{}' should be Map-like, got {}",
                                child_name, type_name
                            ),
                        });
                    }
                }
            }
        }
        Ok(())
    }
    let shape = <T as Facet<'static>>::SHAPE;
    validate_node(tree_doc, shape)
}

/// ```
/// use tree_doc::{parse, Kind};
/// let input = r#"
/// - root { type: "struct" }
///   - servers { type: "map", name: "servers" }
///     - web { type: "struct" }
///       - host
///   - plugins { type: "array", name: "plugins" }
///     - auth { type: "struct" }
/// "#;
/// let node = parse(input).unwrap();
/// assert_eq!(node.kind, Kind::Struct);
/// assert_eq!(node.children.len(), 2);
/// let servers = &node.children[0];
/// assert_eq!(servers.name, Some("servers".to_string()));
/// assert_eq!(servers.kind, Kind::Map);
/// assert_eq!(servers.children.len(), 1);
/// let plugins = &node.children[1];
/// assert_eq!(plugins.name, Some("plugins".to_string()));
/// assert_eq!(plugins.kind, Kind::Array);
/// ```
///
/// ```should_panic
/// use tree_doc::parse;
/// parse("not a valid line").unwrap();
/// ```
pub fn parse(input: &str) -> Result<Node> {
    let mut nodes: Vec<(usize, Node)> = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let rest = &input[offset..];
        let line_end = rest.find('\n').unwrap_or(rest.len());
        let line = &rest[..line_end];
        offset += line_end + if line_end < rest.len() { 1 } else { 0 };
        if line.trim().is_empty() {
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
        let ParsedLine { indent, name, kind } = match parsed {
            Some(p) => p,
            None => continue,
        };
        let column = line.len() - line.trim_start().len() + 1;
        let span = Span {
            line: input[..line_start_offset].lines().count() + 1,
            column,
        };
        let node = Node {
            name,
            kind,
            children: Vec::new(),
            span,
        };
        while nodes.last().is_some_and(|(i, _)| *i >= indent) {
            let (_, child) = nodes.pop().unwrap();
            if let Some((_, parent)) = nodes.last_mut() {
                parent.children.push(child);
            }
        }
        nodes.push((indent, node));
    }
    while nodes.len() > 1 {
        let (_, child) = nodes.pop().unwrap();
        if let Some((_, parent)) = nodes.last_mut() {
            parent.children.push(child);
        }
    }
    nodes
        .into_iter()
        .next()
        .map(|(_, n)| n)
        .ok_or_else(|| Error::ParseError {
            span: Span { line: 1, column: 1 },
            msg: "No root node found".to_string(),
        })
}
