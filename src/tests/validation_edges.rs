#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{parse, validate};

    leanward::nest! {
    #[derive(Debug, Default, facet::Facet)]
    struct RecurseArrayRoot {
        items: Vec<
            #[derive(Debug, Default, facet::Facet)]
            struct RecurseArrayItem {
                name: String,
                nested:
                    #[derive(Debug, Default, facet::Facet)]
                    struct RecurseArrayNested {
                        value: i32,
                    },
            }
        >,
    }}

    #[test]
    fn test_validate_recurse_into_array_children() {
        let src = r#"
- root { type: "struct" }
  - items { type: "array", name: "items" }
    - item.a { type: "struct" }
      - name { name: "name" }
      - nested { type: "struct", name: "nested" }
        - value { name: "value" }
"#;
        let tree = parse(src).unwrap();
        assert!(validate::<RecurseArrayRoot>(&tree).is_ok());
    }

    leanward::nest! {
    #[derive(Debug, Default, facet::Facet)]
    struct RecurseMapRoot {
        items: HashMap<String,
            #[derive(Debug, Default, facet::Facet)]
            struct RecurseMapItem {
                name: String,
                nested:
                    #[derive(Debug, Default, facet::Facet)]
                    struct RecurseMapNested {
                        value: i32,
                    },
            }
        >,
    }}

    #[test]
    fn test_validate_recurse_into_map_children() {
        let src = r#"
- root { type: "struct" }
  - items { type: "map", name: "items" }
    - item.a { type: "struct" }
      - name { name: "name" }
      - nested { type: "struct", name: "nested" }
        - value { name: "value" }
"#;
        let tree = parse(src).unwrap();
        assert!(validate::<RecurseMapRoot>(&tree).is_ok());
    }

    #[derive(Debug, Default, facet::Facet)]
    struct StructToLeafFieldRoot {
        name: String,
    }

    #[test]
    fn test_validate_reject_complex_type_mapped_to_leaf_field() {
        let src = [
            r#"
- root { type: "struct" }
  - name { type: "struct", name: "name" }
    - value { name: "value" }
"#,
            r#"
- root { type: "struct" }
  - name { type: "array", name: "name" }
"#,
            r#"
- root { type: "struct" }
  - name { type: "map", name: "name" }
"#,
        ];
        for src in src {
            let tree = parse(src).unwrap();
            let result = validate::<StructToLeafFieldRoot>(&tree);
            assert!(
                result.is_err(),
                "Struct-typed doc node should not validate against String field"
            );
        }
    }
}
