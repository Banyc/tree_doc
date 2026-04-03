#[cfg(test)]
mod tests {
    use crate::part::NodeName;
    use crate::tree_doc::{Node, RawNode, parse, parse_raw, validate};
    use std::collections::{HashMap, HashSet, VecDeque};

    const SRC: &str = include_str!("example.td");
    const NAMES: &[&str] = &[
        "workspace",
        "servers",
        "crates",
        "versions",
        "meta",
        "deploy_path",
        "install",
        "installed",
    ];
    const NON_NAMES: &[&str] = &["v1"];

    #[test]
    fn test_invalidate_badroot_against_example_doc() {
        let tree = parse(SRC).expect("Failed to parse example.td");
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
        let tree = parse(SRC).expect("Failed to parse example.td");
        println!("{:?}", &tree);
        println!("=== Parsed tree ===");
        let raw_tree = parse_raw(SRC).unwrap();
        print_raw_tree(&raw_tree, 0);

        fn print_raw_tree(node: &RawNode, indent: usize) {
            for _ in 0..indent {
                print!("  ");
            }
            println!("{:?} (children: {})", node.info, node.children.len());
            for child in &node.children {
                print_raw_tree(child, indent + 1);
            }
        }

        let names: HashSet<&str> = NAMES.iter().copied().collect();
        let non_names: HashSet<&str> = NON_NAMES.iter().copied().collect();
        let mut found_names: HashSet<&str> = HashSet::new();
        let mut visiting = VecDeque::new();
        visiting.push_back(&raw_tree);
        while let Some(node) = visiting.pop_front() {
            for child in &node.children {
                visiting.push_back(child);
            }
            if let NodeName::AsField(name) = &node.info.name {
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

        fn is_name_in_tree(node: &Node, name: &str) -> bool {
            match &node.kind_special {
                crate::NodeKindSpecial::Struct { children } => {
                    for (n, c) in children {
                        if n == name {
                            return true;
                        }
                        if let Some(c) = c
                            && is_name_in_tree(c, name)
                        {
                            return true;
                        }
                    }
                    false
                }
                crate::NodeKindSpecial::Array { child } | crate::NodeKindSpecial::Map { child } => {
                    match child {
                        Some(child) => is_name_in_tree(child, name),
                        None => false,
                    }
                }
            }
        }
        for name in NAMES {
            assert2::check!(is_name_in_tree(&tree, name));
        }

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
        random_stuff: String,
        install: String,
        installed: bool,
    }
}
