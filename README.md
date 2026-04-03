# tree_doc

A lightweight doc format for defining and validating nested tree structures — so your implementation stays in sync with your design, automatically.

---

## The Problem

When working with deeply nested structs — config schemas, data models, API response shapes, file system layouts — you typically write them out flat, one field after another. Most languages and editors don't give you a live, hierarchical view of the tree you're building.

This means:

- You have to mentally simulate the entire tree every time you read or modify the structure
- It's easy to miss a level, misplace a field, or lose track of what's nested under what
- Every code review and debugging session requires jumping across files to reconstruct the full shape
- When teammates or AI agents edit the code, there's no authoritative reference to check against — structural drift goes undetected until it breaks something

---

## What `tree_doc` Does

`tree_doc` lets you write a **human-readable, tree-structured document** that describes your nested structure's intended shape — with types, names, and examples — and then **automatically validates** that your actual implementation matches it.

Think of it as a schema doc that doubles as a live correctness check. No more eyeballing. No more mental stack traces through nested types.

---

## Benefits

- **Visual clarity** — See your entire nested structure as an indented tree, not a flat wall of field definitions
- **Automated validation** — Catch missing fields, wrong nesting, or structural drift without manual review
- **AI-agent-safe** — When LLMs refactor your code, `tree_doc` catches any unintended shape changes before they ship
- **Design-first workflow** — Draft the doc first, write the implementation to match, then keep them in sync forever
- **Living documentation** — The doc isn't just for humans; it's a machine-checkable contract for your structure's shape

---

## Example

The following `tree_doc` describes a deployment workspace configuration:

```
- root { type: "struct" }
  - workspace.toml { type: "struct", name: "workspace" }
    - servers: { type: "map", name: "servers" }
      - server.a { type: "struct" }
        - deploy_path
  - crates { type: "array", name: "crates" }
    - path/to/crate.a { type: "struct" }
      - versions { type: "array", name: "versions" }
        - v1
        - v2
          - ...
        - v3
          - install.nu
          - uninstall.nu
          - exe.git_hash
          - config.toml
          - unit.service
      - crate.toml { type: "struct", name: "meta" }
        - servers: { type: "array", name: "servers" }
          - server.a
        - default install: nu install.nu
        - default uninstall: nu uninstall.nu
        - versions { type: "map", name: "versions" }
          - v2 { type: "struct" }
            - install optional: nu install.nu
            - uninstall optional: nu uninstall.nu
            - installed: false
          - v3
            - installed: true
```

At a glance, you can see the full tree — every level, every field, every type — without jumping through code files.

---

## Scenarios

### 1. Designing a complex nested config

You need to define a deeply nested configuration — a deployment manifest, a game settings schema, a compiler options file. The structure has maps, arrays, and optional fields at multiple levels.

**Without `tree_doc`:** You write the structs incrementally, lose track of nesting levels, and only discover missing fields when something breaks at runtime.

**With `tree_doc`:** You draft the tree doc first — laying out the full hierarchy with types and examples — then implement the structs to match. The validator tells you immediately if your code is missing a level or has a field in the wrong place.

### 2. Keeping implementation in sync with design over time

Your data model was correct last month. Since then, several people (and a few AI agents) have touched it. Is it still correct?

**Without `tree_doc`:** You diff the code, re-read scattered design notes, and hope nothing drifted.

**With `tree_doc`:** Run the validator. It checks your live implementation against the doc and reports any structural divergence — no human eyes needed.

### 3. Working with AI-assisted refactoring

You ask an AI agent to refactor your parsing or serialization code. It helpfully reorganizes some structs. Did it preserve the nested shape you intended?

**Without `tree_doc`:** Hard to tell without a careful manual review of the full type hierarchy.

**With `tree_doc`:** The doc is the ground truth. The validator catches any shape changes the agent introduced — whether intentional or accidental — before they reach production.

### 4. Onboarding into an unfamiliar codebase

You're new to a project with a large, deeply nested domain model. Understanding the full structure means tracing through dozens of type definitions across multiple files.

**Without `tree_doc`:** You spend hours building a mental model by jumping between files, and you're still not sure you got it right.

**With `tree_doc`:** Read the doc. The entire tree is right there — every node, every type, every relationship — in one place.
