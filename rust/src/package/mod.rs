//! Package loading and management for Dolfin.
//!
//! A Dolfin package is a directory containing a `package.dlf` manifest
//! and zero or more `.dlf` ontology files organized in subdirectories.

mod discovery;
mod resolver;

pub use discovery::*;
pub use resolver::*;

use crate::ast::{self, OntologyFile as ParsedOntologyFile, PackageFile, QualifiedName};
use crate::comment::CommentMap;
use crate::error::ParseError;
use std::collections::HashMap;
#[cfg(feature = "fs")]
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during package operations.
#[derive(Debug, Error)]
pub enum PackageError {
    /// Package manifest (package.dlf) not found
    #[error("Package manifest not found: {0}")]
    ManifestNotFound(PathBuf),

    /// I/O error while reading a file
    #[error("Failed to read file '{path}': {source}")]
    IoError {
        /// Path to the file that couldn't be read
        path: PathBuf,
        #[source]
        /// The underlying I/O error
        source: std::io::Error,
    },

    /// Parse error in a source file
    #[error("Parse error in '{path}': {source}")]
    ParseError {
        /// Path to the file with the parse error
        path: PathBuf,
        #[source]
        /// The underlying parse error
        source: Box<ParseError>,
    },

    /// Invalid package structure
    #[error("Invalid package structure: {0}")]
    InvalidStructure(String),

    /// Unresolved prefix reference
    #[error("Unresolved prefix '{prefix}' in file '{path}'")]
    UnresolvedPrefix {
        /// The unresolved prefix
        prefix: String,
        /// File containing the unresolved prefix
        path: PathBuf,
    },

    /// Circular dependency detected
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Duplicate definition across files
    #[error("Duplicate definition '{name}' in '{path1}' and '{path2}'")]
    DuplicateDefinition {
        /// Name of the duplicate definition
        name: String,
        /// First file containing the definition
        path1: PathBuf,
        /// Second file containing the definition
        path2: PathBuf,
    },
}

/// A loaded and resolved Dolfin package.
#[derive(Debug, Clone)]
pub struct Package {
    /// Package manifest information
    pub manifest: PackageFile,
    /// Root directory of the package
    pub root: PathBuf,
    /// All ontology files, keyed by their resolved namespace
    pub ontologies: HashMap<QualifiedName, OntologyFile>,
}

impl Package {
    /// Get the base namespace for this package.
    pub fn namespace(&self) -> &QualifiedName {
        &self.manifest.name
    }

    /// Get the Dolfin language version this package targets.
    pub fn dolfin_version(&self) -> &str {
        &self.manifest.dolfin_version
    }

    /// Get the package version.
    pub fn version(&self) -> &str {
        &self.manifest.version
    }

    /// Iterate over all ontology files in the package.
    pub fn iter_ontologies(&self) -> impl Iterator<Item = (&QualifiedName, &OntologyFile)> {
        self.ontologies.iter()
    }

    /// Get an ontology by its namespace.
    pub fn get_ontology(&self, namespace: &QualifiedName) -> Option<&OntologyFile> {
        self.ontologies.get(namespace)
    }

    /// Get all concepts across all ontologies.
    pub fn all_concepts(&self) -> Vec<(&QualifiedName, &ast::ConceptDef)> {
        self.ontologies
            .iter()
            .flat_map(|(ns, onto)| onto.ast.concepts_as_ref().into_iter().map(move |c| (ns, c)))
            .collect()
    }

    /// Get all properties across all ontologies.
    pub fn all_properties(&self) -> Vec<(&QualifiedName, &ast::PropertyDef)> {
        self.ontologies
            .iter()
            .flat_map(|(ns, onto)| {
                onto.ast
                    .properties_as_ref()
                    .into_iter()
                    .map(move |p| (ns, p))
            })
            .collect()
    }

    /// Get all enums across all ontologies.
    pub fn all_enums(&self) -> Vec<(&QualifiedName, &ast::ConceptDef)> {
        self.ontologies
            .iter()
            .flat_map(|(ns, onto)| {
                onto.ast.concepts_as_ref().into_iter().filter_map(move |c| {
                    if let Some(_) = c.one_of {
                        Some((ns, c))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Work out which file a concept *reference* should be declared in.
    ///
    /// Powers the "Declare concept" quick-fix. Given the namespace of the file
    /// where the undeclared reference appears (`current_ns`) and the reference
    /// itself, returns the target file, the local concept name, and whether the
    /// file already exists. See [`resolver::resolve_concept_file`] for the
    /// mapping rules (prefix → file, alias expansion, case folding).
    pub fn concept_file_target(
        &self,
        current_ns: &QualifiedName,
        reference: &QualifiedName,
    ) -> resolver::ConceptFileTarget {
        // Empty alias map when the current file is unknown (e.g. an in-flight
        // buffer not yet part of the package) — only affects alias expansion.
        static NO_PREFIXES: std::sync::OnceLock<HashMap<String, QualifiedName>> =
            std::sync::OnceLock::new();
        let resolved_prefixes = self
            .get_ontology(current_ns)
            .map(|o| &o.resolved_prefixes)
            .unwrap_or_else(|| NO_PREFIXES.get_or_init(HashMap::new));

        resolver::resolve_concept_file(
            reference,
            self.namespace(),
            resolved_prefixes,
            |ns| self.get_ontology(ns).map(|o| o.relative_path.clone()),
        )
    }

    /// Get all rules across all ontologies.
    pub fn all_rules(&self) -> Vec<(&QualifiedName, &ast::RuleDef)> {
        self.ontologies
            .iter()
            .flat_map(|(ns, onto)| onto.ast.rules_as_ref().into_iter().map(move |r| (ns, r)))
            .collect()
    }
}

/// A single ontology file within a package.
#[derive(Debug, Clone)]
pub struct OntologyFile {
    /// File path relative to package root
    pub relative_path: PathBuf,
    /// Absolute file path
    pub absolute_path: PathBuf,
    /// Resolved namespace for this file (derived from path)
    pub namespace: QualifiedName,
    /// IRI name override (from @iri_name annotation), if any
    pub iri_name: Option<crate::ast::IriNameValue>,
    /// Resolved prefix mappings (alias -> full namespace)
    pub resolved_prefixes: HashMap<String, QualifiedName>,
    /// Parsed AST
    pub ast: ParsedOntologyFile,
    /// Comments attached to AST nodes: enables plugin annotation processing.
    pub comment_map: CommentMap,
}

impl OntologyFile {
    /// Get the IRI segment name for this ontology.
    /// Uses the @iri_name override if present, otherwise the last part of the namespace.
    pub fn iri_segment(&self) -> &str {
        self.iri_name.as_ref()
            .map(|v| v.as_str())
            .unwrap_or_else(|| {
                self.namespace
                    .parts
                    .last()
                    .map(|s| s.as_str())
                    .unwrap_or("")
            })
    }
}

/// Load a package from a directory.
///
/// This function:
/// 1. Reads the package.dlf manifest
/// 2. Discovers all .dlf files in the package
/// 3. Parses each file
/// 4. Resolves namespaces and prefixes
///
/// # Arguments
/// * `path` : Path to the package root directory (containing package.dlf)
///
/// # Returns
/// A fully loaded and resolved Package, or an error.
#[cfg(feature = "fs")]
pub fn load_package<P: AsRef<Path>>(path: P) -> Result<Package, Box<PackageError>> {
    let root = path.as_ref().to_path_buf();

    // Load and parse manifest
    let manifest_path = root.join("package.dlf");
    if !manifest_path.exists() {
        return Err(Box::new(PackageError::ManifestNotFound(manifest_path)));
    }

    let manifest_source =
        std::fs::read_to_string(&manifest_path).map_err(|e| PackageError::IoError {
            path: manifest_path.clone(),
            source: e,
        })?;

    let manifest =
        crate::parser::parse_package(&manifest_source).map_err(|e| PackageError::ParseError {
            path: manifest_path,
            source: e,
        })?;

    // Discover all .dlf files
    let discovered_files = discovery::discover_ontology_files(&root)?;

    // Parse all files
    let mut parsed_files = Vec::new();
    for file_info in discovered_files {
        let source = std::fs::read_to_string(&file_info.absolute_path).map_err(|e| {
            PackageError::IoError {
                path: file_info.absolute_path.clone(),
                source: e,
            }
        })?;

        let parsed = crate::parser::parse_ontology_with_comments(&source);
        let ast = parsed.result.map_err(|e| PackageError::ParseError {
            path: file_info.absolute_path.clone(),
            source: Box::new(e),
        })?;
        let comment_map = CommentMap::build(&ast, parsed.comments);

        parsed_files.push((file_info, ast, comment_map));
    }

    // Resolve namespaces and prefixes
    let ontologies = resolver::resolve_package(&manifest, parsed_files)?;

    Ok(Package {
        manifest,
        root,
        ontologies,
    })
}

/// Load a package from an in-memory file map (WASM-compatible).
///
/// Keys are slash-separated relative paths (`"package.dlf"`, `"main.dlf"`, …).
/// Values are the file contents as strings.
///
/// This mirrors [`load_package`] but skips all filesystem access, making it
/// safe to call from a WASM context.
pub fn load_package_from_memory(
    files: &HashMap<String, String>,
) -> Result<Package, Box<PackageError>> {
    // Parse manifest
    let manifest_source = files
        .get("package.dlf")
        .ok_or_else(|| Box::new(PackageError::ManifestNotFound(PathBuf::from("package.dlf"))))?;

    let manifest =
        crate::parser::parse_package(manifest_source).map_err(|e| PackageError::ParseError {
            path: PathBuf::from("package.dlf"),
            source: e,
        })?;

    // Build (DiscoveredFile, ParsedOntologyFile) pairs from the map
    let mut parsed_files = Vec::new();
    for (path_str, content) in files {
        if path_str == "package.dlf" || !path_str.ends_with(".dlf") {
            continue;
        }

        // Reconstruct a sanitised relative PathBuf from the slash-separated key
        let relative_path: PathBuf = path_str
            .split('/')
            .filter(|s| !s.is_empty() && *s != "." && *s != "..")
            .collect();

        let derived_namespace = discovery::path_to_namespace(&relative_path).map_err(Box::new)?;

        let file_info = discovery::DiscoveredFile {
            // No real filesystem; absolute_path is only used in error messages
            absolute_path: relative_path.clone(),
            relative_path,
            derived_namespace,
        };

        let parsed = crate::parser::parse_ontology_with_comments(content);
        let ast = parsed.result.map_err(|e| PackageError::ParseError {
            path: PathBuf::from(path_str),
            source: Box::new(e),
        })?;
        let comment_map = CommentMap::build(&ast, parsed.comments);

        parsed_files.push((file_info, ast, comment_map));
    }

    let ontologies = resolver::resolve_package(&manifest, parsed_files)?;

    Ok(Package {
        manifest,
        root: PathBuf::from("."),
        ontologies,
    })
}

/// Check a package for errors without fully loading it.
///
/// This performs validation but may skip some expensive operations.
#[cfg(feature = "fs")]
pub fn check_package<P: AsRef<Path>>(path: P) -> Result<Vec<String>, Box<PackageError>> {
    let package = load_package(path)?;
    let mut warnings = Vec::new();

    // Check for empty ontologies
    for (ns, onto) in &package.ontologies {
        if onto.ast.declarations.is_empty() {
            warnings.push(format!(
                "Ontology '{}' ({}) has no declarations",
                ns.full(),
                onto.relative_path.display()
            ));
        }
    }

    // Additional checks can be added here

    Ok(warnings)
}
