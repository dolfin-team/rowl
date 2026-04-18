# rowl

**rowl** is a Rust parser for the [Dolfin Ontology Language](https://www.dolfin.fr/docs) (`.dlf` files).

It is the core parsing library used by CLI tools, code generators, and other crates in the Dolfin ecosystem.

## Overview

Dolfin is an indentation-based language for defining ontologies: concepts, properties, inference rules, and enumerations. `rowl` turns Dolfin source files into a typed AST that downstream crates can traverse, validate, or transform.

```dolfin
prefix com.example.common:
  Address

concept Department:
  one of:
    engineering
    sales
    marketing
    hr

concept Person:
  has firstName: string
  has lastName: string
  has email: string
  has address: Address

concept Employee:
  sub Person
  has key employeeId: string
  has department: Department
  has salary: float

property worksFor: Employee -> Organization
property manages: optional Manager -> some Employee
```

## Installation

Add `rowl` to your `Cargo.toml`. If you only need the Rust library (no Python bindings), disable the default features:

```toml
[dependencies]
rowl = { version = "0.1.3", default-features = false }
```

The `python` and `extension-module` features (enabled by default) are only needed when building a PyO3 extension module.

## Usage

### Parse a string

```rust
use rowl::parser::parse_ontology;

let source = r#"
concept Person:
    has name: string
    has age: optional int
"#;

let result = parse_ontology(source);

match result.ontology {
    Some(ontology) if !result.has_errors() => {
        println!("Parsed {} declarations", ontology.declarations.len());
        for concept in ontology.concepts() {
            println!("Concept: {}", concept.name.get());
        }
    }
    _ => eprintln!("{}", result.format_diagnostics(None, None)),
}
```

### Parse a file

```rust
use rowl::parser::parse_ontology_file;

let result = parse_ontology_file("path/to/ontology.dlf");
```

### Parse a package (directory of `.dlf` files)

```rust
use rowl::parser::parse_package;

let package = parse_package("path/to/package/").unwrap();
```

### Preserve comments (for formatters / linters)

```rust
use rowl::parser::parse_ontology_with_comments;

let parsed = parse_ontology_with_comments(source);
// parsed.result : the AST
// parsed.comments: comments with their source positions
```

## AST

The root node returned by a successful parse is `OntologyFile`:

| Field | Type | Description |
|---|---|---|
| `prefixes` | `Vec<PrefixDecl>` | Namespace prefix aliases |
| `declarations` | `Vec<Declaration>` | All top-level declarations |
| `iri_name` | `Option<String>` | Optional IRI override via `@iri_name` |

`Declaration` is an enum with three variants: `Concept`, `Property`, and `Rule`: each holding its own definition struct.

**Convenience accessors on `OntologyFile`:**

```rust
ontology.concepts()    // Vec<ConceptDef>
ontology.properties()  // Vec<PropertyDef>
ontology.rules()       // Vec<RuleDef>
```

### Concepts

```rust
pub struct ConceptDef {
    pub name: SpannedString,
    pub parents: Vec<TypeRef>,          // `sub` declarations
    pub has_declarations: Vec<HasDeclaration>,
    pub one_of: Option<Vec<OneOfVariant>>, // closed-world individuals
}
```

### Properties

```rust
pub struct PropertyDef {
    pub name: SpannedString,
    pub domain: TypeRef,
    pub domain_cardinality: Option<Cardinality>,
    pub range: TypeRef,
    pub range_cardinality: Option<Cardinality>,
}
```

### Cardinality

| Variant | Dolfin syntax |
|---|---|
| `One` | *(default, omitted)* |
| `Optional` | `optional` |
| `Some` | `some` |
| `Any` | `any` |
| `Exact(n)` | `3` |
| `Range(min, max)` | `2..5` / `2..*` |

### Rules

Rules express inference logic via `match`/`then` blocks:

```dolfin
rule EmployeeHasManager:
  match:
    ?e worksFor ?org
    ?e is Employee
  then:
    ?e reportsTo ?manager
```

```rust
pub struct RuleDef {
    pub name: String,
    pub match_block: MatchBlock,  // Vec<Pattern>
    pub then_block: ThenBlock,    // Vec<ThenItem>
}
```

## Diagnostics

Parsing always succeeds structurally: errors are collected into the `ParseResult` rather than panicking:

```rust
let result = parse_ontology(source);

if result.has_errors() {
    // Human-readable, with source context
    eprintln!("{}", result.format_diagnostics(Some("file.dlf"), Some(source)));
}
```

Each diagnostic carries a `Location` (line, column, offset) and an `ErrorCode`.

## Features

| Feature | Default | Description |
|---|---|---|
| `python` | yes | Enables PyO3 bindings |
| `extension-module` | yes | Enables PyO3 `extension-module` (required for `.so` builds) |

Disable both when using `rowl` as a pure Rust library.

## Python bindings

`rowl` ships with optional Python bindings built with [PyO3](https://pyo3.rs). The `python` and `extension-module` features (enabled by default) expose the full parser and AST to Python.

### Installation

```bash
pip install rowl
```

### Usage

```python
import rowl

source = """
concept Person:
  has name: string
  has age: optional int
"""

ontology = rowl.parse(source)

for decl in ontology.declarations:
    print(decl.kind, decl.name)
    if decl.kind == "concept":
        for prop in decl.concept.has_declarations:
            print(f"  has {prop.name}: {prop.type_ref}")
```

**Parse from a file:**

```python
ontology = rowl.parse_file("path/to/ontology.dlf")
```

**Tokenize (debugging):**

```python
tokens = rowl.tokenize("concept Person:\n  has name: string\n")
print(tokens)
# ['concept', 'Person', ':', 'NEWLINE', 'INDENT', 'has', 'name', ':', 'string', ...]
```

**Version:**

```python
print(rowl.version())  # e.g. "0.1.3"
```

### Available classes

All AST types are exposed as Python classes with read-only attributes mirroring the Rust structs: `OntologyFile`, `Declaration`, `ConceptDef`, `HasDeclaration`, `PropertyDef`, `RuleDef`, `MatchBlock`, `ThenBlock`, `ThenItem`, `Pattern`, `Subject`, `Object`, `Assertion`, `TypeRef`, `Cardinality`, `Quantifier`, `Literal`, `QualifiedName`, and more.

## Related crates

- [`dolfin-diagnostic`](https://crates.io/crates/dolfin-diagnostic): diagnostic types shared across the Dolfin toolchain

## Language documentation

Full language reference: [dolfin.fr/docs](https://www.dolfin.fr/docs)

## License

AGPL-3.0-or-later
