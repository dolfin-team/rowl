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
        by_relative_path.insert(discovered.relative_path.clone(), (discovered, parsed, comment_map));
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

    // Multi-part name: first part might be a prefix alias
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
}
