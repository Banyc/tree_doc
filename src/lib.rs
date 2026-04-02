#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    const SRC: &str = include_str!("example_doc.md");

    leanward::nest! {
        struct OkRoot {
            workspace: struct Workspace {
                servers: HashMap<String, WorkspaceServer>
            },
            crates: Vec<Crate>,
        }
    }
    struct WorkspaceServer {
        deploy_path: String,
    }
    leanward::nest! {
        struct Crate {
            versions: Vec<VersionStash>,
            meta: struct CrateMeta {
                servers: Vec<String>,
                versions: HashMap<String, VersionMeta>
            },
        }
    }
    struct VersionStash {
        name: String,
        path: String,
    }
    struct VersionMeta {
        path: String,
    }
}
