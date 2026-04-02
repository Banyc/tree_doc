mod tree_doc;

pub use tree_doc::{
    parse, parse_kv_pair, parse_line, parse_meta_block, parse_quoted_string, validate, Error, Kind,
    Node, ParsedLine, Result, Span,
};

#[cfg(test)]
mod tests {
    use crate::tree_doc::{parse, validate, Node};
    use std::collections::{HashMap, HashSet, VecDeque};

    const SRC: &str = include_str!("example_doc.md");
    const NAMES: &[&str] = &["workspace", "servers", "crates", "versions", "meta"];
    const NON_NAMES: &[&str] = &["v1"];

    #[test]
    fn test_invalidate_badroot_against_example_doc() {
        let tree = parse(SRC).expect("Failed to parse example_doc.md");
        fn f<T: facet::Facet<'static>>(tree: &Node) {
            let result = validate::<T>(tree);
            println!("\nValidation: {:?}", result);
            assert2::check!(result.is_err(), "Validation failed: {:?}", result);
        }
        f::<MissingFieldRoot>(&tree);
        f::<BadKindRoot>(&tree);
    }

    #[test]
    fn test_validate_okroot_against_example_doc() {
        let tree = parse(SRC).expect("Failed to parse example_doc.md");
        println!("=== Parsed tree ===");
        print_tree(&tree, 0);

        fn print_tree(node: &Node, indent: usize) {
            for _ in 0..indent {
                print!("  ");
            }
            println!("{:?} (children: {})", node.name, node.children.len());
            for child in &node.children {
                print_tree(child, indent + 1);
            }
        }

        let names: HashSet<&str> = NAMES.iter().copied().collect();
        let non_names: HashSet<&str> = NON_NAMES.iter().copied().collect();
        let mut found_names: HashSet<&str> = HashSet::new();
        let mut visiting = VecDeque::new();
        visiting.push_back(&tree);
        while let Some(node) = visiting.pop_front() {
            for child in &node.children {
                visiting.push_back(child);
            }
            if let Some(name) = &node.name {
                println!("Found node: '{}'", name);
                found_names.insert(name.as_str());
                assert2::check!(!non_names.contains(name.as_str()));
            }
        }
        assert2::check!(
            found_names == names,
            "Expected names: {:?}, found: {:?}",
            names,
            found_names
        );

        let result = validate::<OkRoot>(&tree);
        println!("\nValidation: {:?}", result);
        assert2::check!(result.is_ok(), "Validation failed: {:?}", result);
    }

    #[derive(Debug, Default, facet::Facet)]
    struct MissingFieldRoot {
        crates: Vec<Crate>,
    }

    #[derive(Debug, Default, facet::Facet)]
    struct BadKindRoot {
        workspace: Workspace,
        crates: HashMap<String, Crate>,
    }

    leanward::nest! {
    #[derive(Debug, Default, facet::Facet)]
    struct OkRoot {
        workspace:
            #[derive(Debug, Default, facet::Facet)]
            struct Workspace {
                servers: HashMap<String, WorkspaceServer>,
            },
        crates: Vec<Crate>,
    }}

    #[derive(Debug, Default, facet::Facet)]
    struct WorkspaceServer {
        deploy_path: String,
    }

    leanward::nest! {
    #[derive(Debug, Default, facet::Facet)]
    struct Crate {
        versions: Vec<VersionStash>,
        meta:
            #[derive(Debug, Default, facet::Facet)]
            struct CrateMeta {
                servers: Vec<String>,
                versions: HashMap<String, VersionMeta>,
            },
    }}

    #[derive(Debug, Default, facet::Facet)]
    struct VersionStash {
        name: String,
        path: String,
    }

    #[derive(Debug, Default, facet::Facet)]
    struct VersionMeta {
        path: String,
    }
}
