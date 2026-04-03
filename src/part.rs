use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::space0,
    combinator::{delimited, opt, separated, terminated},
    stream::Stream,
    token::{take, take_till, take_while},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}
impl Span {
    pub fn from_offset(input: &str, byte_offset: usize) -> Self {
        let up_to = &input[..byte_offset];
        let line = up_to.matches('\n').count() + 1;
        let column = byte_offset - up_to.rfind('\n').map(|p| p + 1).unwrap_or(0) + 1;
        Span { line, column }
    }
}
#[cfg(test)] #[test] #[rustfmt::skip] fn test_span_from_offset() { leanward::local! {
    let tc = struct Case<'a> {
        input:  &'a str = "hello\nworld",
        offset: usize   = 0,
        line:   usize   = 1,
        column: usize   = 1,
    };}
    let inputs = [
        tc,
        Case { input: "hello\nworld", offset: 0,  line: 1, column: 1  },
        Case { input: "hello\nworld", offset: 4,  line: 1, column: 5  },
        Case { input: "hello\nworld", offset: 6,  line: 2, column: 1  },
        Case { input: "hello\nworld", offset: 10, line: 2, column: 5  },
        Case { input: "a\nb\nc",      offset: 2,  line: 2, column: 1  },
        Case { input: "a\nb\nc",      offset: 4,  line: 3, column: 1  },
        Case { input: "",             offset: 0,  line: 1, column: 1  },
        Case { input: "\n",           offset: 1,  line: 2, column: 1  },
        Case { input: "hello\nworld", offset: 11, line: 2, column: 6  },
    ];
    for tc in inputs {
        let span = Span::from_offset(tc.input, tc.offset);
        assert2::check!(span.line == tc.line, "input: {:?}, offset: {}", tc.input, tc.offset);
        assert2::check!(span.column == tc.column, "input: {:?}, offset: {}", tc.input, tc.offset);
    }
}

leanward::nest! {
#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
    pub name:
        #[derive(Debug, Clone, PartialEq)]
        pub enum NodeName {
            AsField(String),
            Anonymous,
        },
    pub kind:
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum Kind {
            Leaf,
            Complex(
                #[derive(Debug, Clone, Copy, PartialEq)]
                pub enum ComplexKind {
                    Struct,
                    Array,
                    Map,
                }
            ),
        },
}}

pub type Input<'i> = LocatingSlice<&'i str>;

pub struct ParsedLine {
    pub indent: usize,
    pub info: Option<NodeInfo>,
}

pub fn parse_line(input: &mut Input<'_>) -> ModalResult<ParsedLine> {
    let start = input.checkpoint();
    let indent = take_while(0.., ' ')
        .map(|s: &str| s.len())
        .parse_next(input)?;
    ("-", space0).parse_next(input)?;
    let last_brace = input.rfind("{");
    let take_len = last_brace.unwrap_or(input.len());
    take(take_len).parse_next(input)?;
    let kind_and_name = opt(parse_meta_block).parse_next(input)?;
    let Some((kind, name)) = kind_and_name else {
        input.reset(&start);
        return Ok(ParsedLine { indent, info: None });
    };
    space0.parse_next(input)?;
    let info = NodeInfo { name, kind };
    Ok(ParsedLine {
        indent,
        info: Some(info),
    })
}
#[cfg(test)] #[test] #[rustfmt::skip] fn test_parse_line() { leanward::local! {
    let tc = struct Case<'a> {
        raw:    &'a str          = r#"- servers { type: "map", name: "servers" }"#,
        some:   bool             = true,
        indent: usize            = 0,
        name:   Option<&'a str>  = Some("servers"),
        kind:   Kind             = Kind::Complex(ComplexKind::Map),
    };}
    let inputs = [
        tc,
        Case { raw: r#"- root { type: "struct" }"#,                  some: true,  indent: 0, name: None,          kind: Kind::Complex(ComplexKind::Struct) },
        Case { raw: r#"  - child { type: "array", name: "items" }"#, some: true,  indent: 2, name: Some("items"), kind: Kind::Complex(ComplexKind::Array) },
        Case { raw: r#"- host"#,                                     some: false, indent: 0, name: None,          kind: Kind::Leaf },
        Case { raw: r#"- root{} { type: "struct" }"#,                some: true,  indent: 0, name: None,          kind: Kind::Complex(ComplexKind::Struct) },
    ];
    for tc in inputs {
        let mut input = LocatingSlice::new(tc.raw);
        let result = parse_line(&mut input).unwrap();
        assert2::check!(result.info.is_some() == tc.some, "input: {}", tc.raw);
        assert2::check!(result.indent == tc.indent, "input: {}", tc.raw);
        let Some(p) = result.info else { continue };
        let expected_name = match tc.name {
            Some(s) => NodeName::AsField(s.to_string()),
            None => NodeName::Anonymous,
        };
        assert2::check!(p.name == expected_name, "input: {}", tc.raw);
        assert2::check!(p.kind == tc.kind, "input: {}", tc.raw);
    }
}

pub fn parse_meta_block(input: &mut Input<'_>) -> ModalResult<(Kind, NodeName)> {
    let content = terminated(separated(0.., parse_kv_pair, ","), space0);
    let pairs: Vec<(String, String)> = delimited("{", content, "}").parse_next(input)?;
    let kind = match pairs
        .iter()
        .find(|(k, _)| k == "type")
        .map(|(_, v)| v.as_str())
    {
        Some("array") => Kind::Complex(ComplexKind::Array),
        Some("map") => Kind::Complex(ComplexKind::Map),
        Some("struct") => Kind::Complex(ComplexKind::Struct),
        None => Kind::Leaf,
        Some(_) => {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
    };
    let name = pairs
        .iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.clone());
    let name = match name {
        Some(name) => NodeName::AsField(name),
        None => NodeName::Anonymous,
    };
    Ok((kind, name))
}
#[cfg(test)] #[test] #[rustfmt::skip] fn test_parse_meta_block() { leanward::local! {
    let tc = struct Case<'a> {
        raw:      &'a str          = r#"{ type: "array", name: "crates" }"#,
        kind:     Kind             = Kind::Complex(ComplexKind::Array),
        name:     Option<&'a str>  = Some("crates"),
    };}
    let inputs = [
        tc,
        Case { raw: r#"{ type: "map", name: "servers" }"#, kind: Kind::Complex(ComplexKind::Map),    name: Some("servers") },
        Case { raw: r#"{ type: "struct" }"#,               kind: Kind::Complex(ComplexKind::Struct), name: None },
        Case { raw: r#"{ name: "foo", type: "array" }"#,   kind: Kind::Complex(ComplexKind::Array),  name: Some("foo") },
        Case { raw: r#"{ name: "foo" }"#,                  kind: Kind::Leaf,                          name: Some("foo") },
    ];
    for tc in inputs {
        let mut input = LocatingSlice::new(tc.raw);
        let (kind, name) = parse_meta_block(&mut input).unwrap();
        assert2::check!(kind == tc.kind, "input: {}", tc.raw);
        let expected_name = match tc.name {
            Some(s) => NodeName::AsField(s.to_string()),
            None => NodeName::Anonymous,
        };
        assert2::check!(name == expected_name, "input: {}", tc.raw);
    }
}

pub fn parse_kv_pair(input: &mut Input<'_>) -> ModalResult<(String, String)> {
    space0.parse_next(input)?;
    let key: &str = take_till(1.., ':').parse_next(input)?;
    (":", space0).parse_next(input)?;
    let value = parse_quoted_string.parse_next(input)?;
    Ok((key.trim().to_string(), value))
}
#[cfg(test)] #[test] #[rustfmt::skip] fn test_parse_kv_pair() { leanward::local! {
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

pub fn parse_quoted_string(input: &mut Input<'_>) -> ModalResult<String> {
    '\"'.parse_next(input)?;
    let mut result = String::new();
    loop {
        let checkpoint = input.checkpoint();
        let Some(ch) = input.next_token() else {
            input.reset(&checkpoint);
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        };
        match ch {
            '"' => return Ok(result),
            '\\' => {
                let Some(next) = input.next_token() else {
                    result.push('\\');
                    continue;
                };
                let is_escaped_quote = next == '"';
                if is_escaped_quote {
                    result.push('"');
                } else {
                    result.push('\\');
                    result.push(next);
                }
            }
            c => result.push(c),
        }
    }
}
#[cfg(test)] #[test] #[rustfmt::skip] fn test_parse_quoted_string() { leanward::local! {
    let tc = struct Case<'a> {
        raw: &'a str  = r#""hello world""#,
        exp: &'a str  = "hello world",
    };}
    let inputs = [
        tc,
        Case { raw: r#""""#, exp: "" },
        Case { raw: r#""path/to/crate.a""#, exp: "path/to/crate.a" },
        Case { raw: r#""v3""#, exp: "v3" },
    ];
    for tc in inputs {
        use winnow::LocatingSlice;

        let mut input = LocatingSlice::new(tc.raw);
        let s = parse_quoted_string(&mut input).unwrap();
        assert2::check!(s == tc.exp, "input: {}", tc.raw);
    }
}
#[cfg(test)] #[test] #[rustfmt::skip] fn test_parse_quoted_string_nasty() { leanward::local! {
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
