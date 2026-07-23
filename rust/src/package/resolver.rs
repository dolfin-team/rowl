// dolfin-core/src/package/resolver.rs
//! Namespace and prefix resolution for Dolfin packages.

use super::{DiscoveredFile, OntologyFile, PackageError};
use crate::ast::{OntologyFile as ParsedOntologyFile, PackageFile, QualifiedName};
use crate::comment::CommentMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Resolve all ontology files in a package.
///
/// This function:
/// 1. Computes the full namespace for each file
/// 2. Resolves prefix declarations to full namespaces
/// 3. Validates that all references can be resolved
pub fn resolve_package(
    manifest: &PackageFile,
    files: Vec<(DiscoveredFile, ParsedOntologyFile, CommentMap)>,
) -> Result<HashMap<QualifiedName, OntologyFile>, PackageError> {
    let base_namespace = &manifest.name;
    info!("Found base_namespace: {}", base_namespace.full());
    // First pass: build namespace index
    let mut namespace_index: HashMap<QualifiedName, PathBuf> = HashMap::new();
    let mut by_relative_path: HashMap<PathBuf, (DiscoveredFile, ParsedOntologyFile, CommentMap)> =
        HashMap::new();

    for (discovered, parsed, comment_map) in files {
        // Full namespace = base + derived
        let full_namespace = base_namespace.join(&discovered.derived_namespace);

        // Check for duplicates
        if let Some(existing_path) = namespace_index.get(&full_namespace) {
            return Err(PackageError::DuplicateDefinition {
                name: full_namespace.full(),
                path1: existing_path.clone(),
                path2: discovered.absolute_path.clone(),
            });
        }

        namespace_index.insert(full_namespace.clone(), discovered.absolute_path.clone());
        by_relative_path.insert(
            discovered.relative_path.clone(),
            (discovered, parsed, comment_map),
        );
    }
    info!("Found namespace_index: {:?}", namespace_index);
    info!("Found by_relative_path: {:?}", by_relative_path);

    // Build alias index for resolution
    // Maps last component -> full namespace for local resolution
    let local_alias_index: HashMap<String, QualifiedName> = namespace_index
        .keys()
        .map(|ns| (ns.last().to_string(), ns.clone()))
        .collect();

    info!("Found local_alias_index: {:?}", local_alias_index);

    // Second pass: resolve prefixes and build final ontologies
    let mut ontologies = HashMap::new();

    for (relative_path, (discovered, parsed, comment_map)) in by_relative_path {
        let full_namespace = base_namespace.join(&discovered.derived_namespace);

        // Resolve prefixes
        let resolved_prefixes = resolve_prefixes(
            &parsed.prefixes,
            base_namespace,
            &namespace_index,
            &local_alias_index,
            &discovered.absolute_path,
        )?;

        let ontology = OntologyFile {
            relative_path,
            absolute_path: discovered.absolute_path,
            namespace: full_namespace.clone(),
            iri_name: parsed.iri_name.clone(),
            resolved_prefixes,
            ast: parsed,
            comment_map,
        };

        ontologies.insert(full_namespace, ontology);
    }

    Ok(ontologies)
}

/// Resolve prefix declarations to full namespaces.
fn resolve_prefixes(
    prefixes: &[crate::ast::PrefixDecl],
    base_namespace: &QualifiedName,
    namespace_index: &HashMap<QualifiedName, PathBuf>,
    local_alias_index: &HashMap<String, QualifiedName>,
    file_path: &Path,
) -> Result<HashMap<String, QualifiedName>, PackageError> {
    let mut resolved = HashMap::new();

    for prefix in prefixes {
        let full_namespace = resolve_single_prefix(
            &prefix.path,
            base_namespace,
            namespace_index,
            local_alias_index,
        )
        .ok_or_else(|| PackageError::UnresolvedPrefix {
            prefix: prefix.path.full(),
            path: file_path.to_path_buf(),
        })?;

        resolved.insert(prefix.alias.clone(), full_namespace);
    }

    Ok(resolved)
}

/// Resolve a single prefix path to a full namespace.
///
/// Resolution order:
/// 1. If it's already a full path that exists in the index, use it
/// 2. If it's a relative path (single name), look up in local alias index
/// 3. If it starts with the base namespace, try direct lookup
/// 4. Otherwise, prepend base namespace and try again
fn resolve_single_prefix(
    path: &QualifiedName,
    base_namespace: &QualifiedName,
    namespace_index: &HashMap<QualifiedName, PathBuf>,
    local_alias_index: &HashMap<String, QualifiedName>,
) -> Option<QualifiedName> {
    // URI literal (e.g. <http://example.com/>) — pass through without resolution
    if path.parts.len() == 1 && path.parts[0].contains("://") {
        return Some(path.clone());
    }

    // Direct lookup
    if namespace_index.contains_key(path) {
        return Some(path.clone());
    }

    // Single-part name: local resolution
    if path.parts.len() == 1
        && let Some(full) = local_alias_index.get(&path.parts[0])
    {
        return Some(full.clone());
    }

    // Try with base namespace prepended
    let with_base = base_namespace.join(path);
    if namespace_index.contains_key(&with_base) {
        return Some(with_base);
    }

    error!("Failed single prefix\n -> {:?}\n -> {:?}", path, with_base);
    // Not found
    None
}

/// Resolve a type reference within an ontology file.
///
/// This is used during code generation to get the full IRI for a type.
pub fn resolve_type_ref(
    type_name: &QualifiedName,
    current_namespace: &QualifiedName,
    resolved_prefixes: &HashMap<String, QualifiedName>,
    all_namespaces: &HashMap<QualifiedName, OntologyFile>,
) -> Option<QualifiedName> {
    // Single-part name: could be local or prefixed
    if type_name.parts.len() == 1 {
        let name = &type_name.parts[0];

        // Check if it's defined locally (in current namespace)
        if let Some(onto) = all_namespaces.get(current_namespace) {
            for decl in &onto.ast.declarations {
                if decl.name() == *name {
                    return Some(current_namespace.join(type_name));
                }
            }
        }

        // Check prefixes
        // (Would need more context to know which prefix applies)
    }

    // Prefixed name (`alias:Local`): resolve alias strictly
    if type_name.is_prefixed {
        let alias = &type_name.parts[0];
        let rest = QualifiedName::new(type_name.parts[1..].to_vec(), None);
        return resolved_prefixes.get(alias).map(|ns| ns.join(&rest));
    }

    // Dot-separated multi-part: first part might be a package-namespace alias
    if type_name.parts.len() >= 2 {
        let maybe_alias = &type_name.parts[0];
        if let Some(prefix_ns) = resolved_prefixes.get(maybe_alias) {
            let rest = QualifiedName::new(type_name.parts[1..].to_vec(), None);
            return Some(prefix_ns.join(&rest));
        }
    }

    // Already fully qualified?
    if all_namespaces.contains_key(type_name) {
        return Some(type_name.clone());
    }

    None
}

/// The `.dlf` file that a concept *declaration* should be written into, derived
/// from a concept *reference*'s prefix.
///
/// This is what powers the editor's "Declare concept" quick-fix: when a
/// reference like `science.biology.Animal` is used but never declared, the
/// concept `Animal` should be created in `science/biology.dlf` — not in the
/// current file, and not under the fully-qualified name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptFileTarget {
    /// Path (relative to the package root) of the file the declaration belongs
    /// in, or `None` when it belongs in the *current* file (a bare,
    /// unprefixed reference such as `Animal`).
    pub relative_path: Option<PathBuf>,
    /// Full resolved namespace of the target file, when it could be determined.
    pub namespace: Option<QualifiedName>,
    /// The concept's local name — i.e. what to write after `concept `.
    pub concept_name: String,
    /// `true` when the target file already exists in the package (append to it);
    /// `false` when the path was derived and the file must be created.
    pub exists: bool,
}

/// Work out which file a concept reference should be declared in.
///
/// Mapping (the prefix names the file, the last component names the concept):
/// - `Animal`                 → current file (`relative_path: None`)
/// - `biology.Animal`         → `biology.dlf`
/// - `science.biology.Animal` → `science/biology.dlf`
///
/// Two things can decouple the *seen* name from the *file* name, and both are
/// handled here:
/// 1. **Prefix aliases.** `ex:Animal` (or a dotted `ex.Animal` whose first part
///    is a declared alias) expands through `resolved_prefixes` to the aliased
///    namespace rather than a literal `ex.dlf`.
/// 2. **Case folding.** [`path_to_namespace`](super::path_to_namespace)
///    lowercases path components, so a namespace never uniquely reconstructs the
///    on-disk filename. When the target file already exists, `file_lookup`
///    returns its real (correctly-cased) path; only genuinely new files fall
///    back to a lowercased derived path — which round-trips back to this
///    namespace by construction.
///
/// `base_namespace` is the package's base namespace (used to strip the base off
/// a resolved namespace when deriving a relative path). `file_lookup` maps a
/// full namespace to an existing file's relative path, if any.
pub fn resolve_concept_file(
    reference: &QualifiedName,
    base_namespace: &QualifiedName,
    resolved_prefixes: &HashMap<String, QualifiedName>,
    file_lookup: impl Fn(&QualifiedName) -> Option<PathBuf>,
) -> ConceptFileTarget {
    let concept_name = reference.last();

    // Bare, unprefixed name (`Animal`) → same file as the usage.
    if reference.parts.len() <= 1 {
        return ConceptFileTarget {
            relative_path: None,
            namespace: None,
            concept_name,
            exists: false,
        };
    }

    // Everything before the last component names the target file's namespace.
    let prefix_parts = &reference.parts[..reference.parts.len() - 1];
    let first = &prefix_parts[0];
    let rest = &prefix_parts[1..];

    // Resolve the prefix to a full namespace.
    //
    // A prefixed reference (`ex:Animal`) always expands its alias. A dotted
    // reference (`a.b.Animal`) expands its first component *only if* it is a
    // declared alias; otherwise the whole prefix is a namespace path under the
    // package base.
    let namespace = if reference.is_prefixed || resolved_prefixes.contains_key(first) {
        resolved_prefixes.get(first).map(|alias_ns| {
            if rest.is_empty() {
                alias_ns.clone()
            } else {
                alias_ns.join(&QualifiedName::new(
                    rest.iter().map(|s| s.to_lowercase()).collect(),
                    None,
                ))
            }
        })
    } else {
        // Namespace path relative to the base. Lowercase to match the
        // path→namespace folding so existing-file lookup and round-trip hold.
        let path_ns = QualifiedName::new(
            prefix_parts.iter().map(|s| s.to_lowercase()).collect(),
            None,
        );
        Some(base_namespace.join(&path_ns))
    };

    let Some(namespace) = namespace else {
        // Alias could not be resolved — best effort: no file, caller decides.
        return ConceptFileTarget {
            relative_path: None,
            namespace: None,
            concept_name,
            exists: false,
        };
    };

    // Existing file → use its real, correctly-cased path.
    if let Some(existing) = file_lookup(&namespace) {
        return ConceptFileTarget {
            relative_path: Some(existing),
            namespace: Some(namespace),
            concept_name,
            exists: true,
        };
    }

    // New file → derive `<ns minus base>/…​.dlf` (already lowercased).
    let relative = namespace.strip_prefix(base_namespace);
    let derived = if relative.parts.is_empty() {
        PathBuf::from(format!("{}.dlf", namespace.last()))
    } else {
        PathBuf::from(format!("{}.dlf", relative.full_slashed()))
    };

    ConceptFileTarget {
        relative_path: Some(derived),
        namespace: Some(namespace),
        concept_name,
        exists: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::QualifiedName;

    #[test]
    fn test_resolve_single_prefix() {
        let base = QualifiedName::new(vec!["com".into(), "example".into()], None);

        let mut index = HashMap::new();
        index.insert(
            QualifiedName::new(vec!["com".into(), "example".into(), "common".into()], None),
            PathBuf::from("common.dlf"),
        );
        index.insert(
            QualifiedName::new(
                vec![
                    "com".into(),
                    "example".into(),
                    "hr".into(),
                    "employee".into(),
                ],
                None,
            ),
            PathBuf::from("hr/employee.dlf"),
        );

        let local_alias: HashMap<String, QualifiedName> = index
            .keys()
            .map(|ns| (ns.last().to_string(), ns.clone()))
            .collect();

        // Single name resolution
        let result = resolve_single_prefix(
            &QualifiedName::new(vec!["common".into()], None),
            &base,
            &index,
            &local_alias,
        );
        assert_eq!(result.unwrap().full(), "com.example.common");

        // Relative path resolution
        let result = resolve_single_prefix(
            &QualifiedName::new(vec!["hr".into(), "employee".into()], None),
            &base,
            &index,
            &local_alias,
        );
        assert_eq!(result.unwrap().full(), "com.example.hr.employee");
    }

    fn empty_prefixes() -> HashMap<String, QualifiedName> {
        HashMap::new()
    }

    #[test]
    fn concept_file_bare_name_is_current_file() {
        let base = QualifiedName::new(vec!["com".into(), "example".into()], None);
        let r = QualifiedName::new(vec!["Animal".into()], None);
        let t = resolve_concept_file(&r, &base, &empty_prefixes(), |_| None);
        assert_eq!(t.relative_path, None);
        assert_eq!(t.concept_name, "Animal");
        assert!(!t.exists);
    }

    #[test]
    fn concept_file_single_prefix_new_file() {
        let base = QualifiedName::new(vec!["com".into(), "example".into()], None);
        let r = QualifiedName::new(vec!["biology".into(), "Animal".into()], None);
        let t = resolve_concept_file(&r, &base, &empty_prefixes(), |_| None);
        assert_eq!(t.relative_path, Some(PathBuf::from("biology.dlf")));
        assert_eq!(t.concept_name, "Animal");
        assert_eq!(t.namespace.unwrap().full(), "com.example.biology");
        assert!(!t.exists);
    }

    #[test]
    fn concept_file_nested_prefix_new_file() {
        let base = QualifiedName::new(vec!["com".into(), "example".into()], None);
        let r = QualifiedName::new(
            vec!["science".into(), "biology".into(), "Animal".into()],
            None,
        );
        let t = resolve_concept_file(&r, &base, &empty_prefixes(), |_| None);
        assert_eq!(t.relative_path, Some(PathBuf::from("science/biology.dlf")));
        assert_eq!(t.concept_name, "Animal");
        assert!(!t.exists);
    }

    #[test]
    fn concept_file_existing_uses_real_casing() {
        let base = QualifiedName::new(vec!["com".into(), "example".into()], None);
        let r = QualifiedName::new(vec!["biology".into(), "Animal".into()], None);
        // File exists on disk with capitalised name.
        let t = resolve_concept_file(&r, &base, &empty_prefixes(), |ns| {
            (ns.full() == "com.example.biology").then(|| PathBuf::from("Biology.dlf"))
        });
        assert_eq!(t.relative_path, Some(PathBuf::from("Biology.dlf")));
        assert!(t.exists);
    }

    #[test]
    fn concept_file_alias_expands_not_literal() {
        let base = QualifiedName::new(vec!["com".into(), "example".into()], None);
        let mut prefixes = HashMap::new();
        // `bio` aliases the science.biology namespace.
        prefixes.insert(
            "bio".to_string(),
            QualifiedName::new(
                vec!["com".into(), "example".into(), "science".into(), "biology".into()],
                None,
            ),
        );
        // Dotted form `bio.Animal`.
        let r = QualifiedName::new(vec!["bio".into(), "Animal".into()], None);
        let t = resolve_concept_file(&r, &base, &prefixes, |_| None);
        assert_eq!(t.relative_path, Some(PathBuf::from("science/biology.dlf")));
        assert_eq!(t.concept_name, "Animal");

        // Prefixed form `bio:Animal`.
        let r = QualifiedName::new_prefixed("bio".into(), "Animal".into(), None);
        let t = resolve_concept_file(&r, &base, &prefixes, |_| None);
        assert_eq!(t.relative_path, Some(PathBuf::from("science/biology.dlf")));
    }
}
