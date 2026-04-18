//! File discovery for Dolfin packages.

use super::PackageError;
use crate::ast::QualifiedName;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Information about a discovered ontology file.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Path relative to package root
    pub relative_path: PathBuf,
    /// Absolute path
    pub absolute_path: PathBuf,
    /// Namespace derived from the file path
    pub derived_namespace: QualifiedName,
}

/// Discover all .dlf ontology files in a package directory.
///
/// This function walks the directory tree and finds all `.dlf` files
/// except for `package.dlf`. It computes the namespace for each file
/// based on its path relative to the package root.
pub fn discover_ontology_files<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<DiscoveredFile>, PackageError> {
    let root = root.as_ref();
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip non-files
        if !path.is_file() {
            continue;
        }

        // Skip non-.dlf files
        let extension = path.extension().and_then(|e| e.to_str());
        if extension != Some("dlf") {
            continue;
        }

        // Skip package.dlf
        let file_name = path.file_name().and_then(|n| n.to_str());
        if file_name == Some("package.dlf") {
            continue;
        }

        // Compute relative path
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| {
                PackageError::InvalidStructure(format!(
                    "File '{}' is not under package root '{}'",
                    path.display(),
                    root.display()
                ))
            })?
            .to_path_buf();

        // Derive namespace from path
        let derived_namespace = path_to_namespace(&relative_path)?;

        files.push(DiscoveredFile {
            relative_path,
            absolute_path: path.to_path_buf(),
            derived_namespace,
        });
    }

    // Sort for deterministic ordering
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(files)
}

/// Convert a file path to a namespace.
///
/// Rules:
/// - Directory separators become dots
/// - The .dlf extension is removed
/// - Names are converted to lowercase
/// - Invalid characters are rejected
///
/// Example: `science/biology/Animal.dlf` -> `science.biology.animal`
pub(super) fn path_to_namespace(path: &Path) -> Result<QualifiedName, PackageError> {
    let mut parts = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => {
                let name_str = name.to_str().ok_or_else(|| {
                    PackageError::InvalidStructure(format!(
                        "Path component '{}' contains invalid UTF-8",
                        name.to_string_lossy()
                    ))
                })?;

                // Remove .dlf extension if present
                let name_str = name_str.strip_suffix(".dlf").unwrap_or(name_str);

                // Validate: must be valid identifier (ASCII alphanumeric + underscore)
                if !is_valid_namespace_part(name_str) {
                    return Err(PackageError::InvalidStructure(format!(
                        "Invalid namespace component '{}'. \
                         Use ASCII letters, digits, and underscores only. \
                         Use @iri_name annotation for non-ASCII IRI names.",
                        name_str
                    )));
                }

                // Convert to lowercase for consistency
                parts.push(name_str.to_lowercase());
            }
            _ => {
                // Skip other components (., .., prefix, root)
            }
        }
    }

    if parts.is_empty() {
        return Err(PackageError::InvalidStructure(
            "Empty namespace derived from path".to_string(),
        ));
    }

    Ok(QualifiedName::new(parts, None))
}

/// Check if a string is a valid namespace part.
///
/// Must start with a letter, followed by letters, digits, or underscores.
fn is_valid_namespace_part(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();

    // First character must be a letter
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }

    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_namespace() {
        let ns = path_to_namespace(Path::new("animal.dlf")).unwrap();
        assert_eq!(ns.full(), "animal");

        let ns = path_to_namespace(Path::new("science/biology/Animal.dlf")).unwrap();
        assert_eq!(ns.full(), "science.biology.animal");

        let ns = path_to_namespace(Path::new("hr/Employee.dlf")).unwrap();
        assert_eq!(ns.full(), "hr.employee");
    }

    #[test]
    fn test_invalid_namespace() {
        // Non-ASCII should fail
        assert!(path_to_namespace(Path::new("animé.dlf")).is_err());

        // Starting with number should fail
        assert!(path_to_namespace(Path::new("123abc.dlf")).is_err());
    }

    #[test]
    fn test_valid_namespace_part() {
        assert!(is_valid_namespace_part("animal"));
        assert!(is_valid_namespace_part("Animal"));
        assert!(is_valid_namespace_part("my_concept"));
        assert!(is_valid_namespace_part("Thing2"));

        assert!(!is_valid_namespace_part(""));
        assert!(!is_valid_namespace_part("123"));
        assert!(!is_valid_namespace_part("my-concept"));
        assert!(!is_valid_namespace_part("animé"));
    }
}
