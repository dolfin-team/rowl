//! Dolfin - A parser for the Dolfin ontology language.
//!
//! This crate provides a parser for the Dolfin language, which is an
//! indentation-based language for defining ontologies.
//!
//! # Example (Rust)
//!
//! ```rust
//! use rowl::parser;
//!
//! let source = r#"
//! concept Person:
//!     has name: string
//!     has age: optional int
//! "#;
//! let result = parser::parse_ontology(source);
//! match result.ontology {
//!     Some(ontology) if !result.has_errors() => println!("Parsed {} statements", ontology.declarations.len()),
//!     _ => eprintln!("Parse error: {}", result.format_diagnostics(None, None)),
//! }
//! ```
//!
//! # Example (Python)
//!
//! ```python
//! import rowl
//!
//! source = """
//! namespace com.example
//!
//! ontology MyOntology:
//!   concept Person:
//!     has name: string
//! """
//!
//! program = rowl.parse(source)
//! for stmt in program.statements:
//!     print(stmt)
//! ```

pub mod annotation;
pub mod ast;
pub mod comment;
pub mod error;
pub mod lexer;
pub mod macros;
pub mod package;
pub mod parser;

// Re-export main types at crate root
pub use ast::*;
pub use error::{Location, ParseError};
pub use package::{OntologyFile as LoadedOntologyFile, Package, PackageError};
pub use parser::{
    parse_ontology, parse_ontology_file, parse_ontology_with_comments, parse_package,
};

// PyO3 Python module
#[cfg(feature = "python")]
use pyo3::prelude::*;

use crate::{
    error::{DolfinParseError, ParseResult},
    parser::parse_result_strict,
};

/// Parse Dolfin source code from a string.
///
/// Args:
///     source: The Dolfin source code as a string.
///
/// Returns:
///     A Program object containing the parsed AST.
///
/// Raises:
///     ValueError: If the source code cannot be parsed.
///
/// Example:
///     >>> import rowl
///     >>> program = rowl.parse("namespace com.example\\n")
///     >>> print(program)
///     Program(statements=1)
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse")]
fn py_parse(source: &str) -> PyResult<OntologyFile> {
    parse_result_strict(parse_ontology(source)).map_err(|e| (*e).into())
}

/// Parse Dolfin source code from a file.
///
/// Args:
///     path: Path to the Dolfin source file.
///
/// Returns:
///     A Program object containing the parsed AST.
///
/// Raises:
///     ValueError: If the file cannot be read or parsed.
///
/// Example:
///     >>> import rowl
///     >>> program = rowl.parse_file("ontology.dolfin")
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse_file")]
fn py_parse_file(path: &str) -> PyResult<OntologyFile> {
    parse_result_strict(parse_ontology_file(path)).map_err(|e| (*e).into())
}

/// Tokenize Dolfin source code and return a list of tokens.
///
/// This is primarily useful for debugging the lexer.
///
/// Args:
///     source: The Dolfin source code as a string.
///
/// Returns:
///     A list of token strings.
///
/// Example:
///     >>> import rowl
///     >>> tokens = rowl.tokenize("namespace com.example\\n")
///     >>> print(tokens)
///     ['namespace', 'com', '.', 'example', 'NEWLINE']
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "tokenize")]
fn py_tokenize(source: &str) -> PyResult<Vec<String>> {
    let lexer = lexer::Lexer::new(source);
    let mut tokens = Vec::new();

    for result in lexer {
        match result {
            Ok((_, tok, _)) => tokens.push(format!("{}", tok)),
            Err(e) => return Err(pyo3::exceptions::PyValueError::new_err(e.message)),
        }
    }

    Ok(tokens)
}

/// Get version information about the rowl parser.
///
/// Returns:
///     A string containing the version number.
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "version")]
fn py_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Dolfin - A parser for the Dolfin ontology language.
///
/// This module provides functions and classes for parsing Dolfin source code
/// into an abstract syntax tree (AST).
///
/// Functions:
///     parse(source): Parse Dolfin source code from a string.
///     parse_file(path): Parse Dolfin source code from a file.
///     tokenize(source): Tokenize source code (for debugging).
///     version(): Get the parser version.
///
/// Classes:
///     Program: The root AST node representing a Dolfin file.
///     Statement: A top-level statement (namespace, import, or ontology).
///     QualifiedName: A dot-separated name (e.g., "com.example.Person").
///     ImportStatement: An import with path and alias.
///     OntologyDef: An ontology definition with its members.
///     ConceptDef: A concept definition with properties.
///     PropertyDef: A property definition with domain and range.
///     RuleDef: An inference rule with match/then blocks.
///     And many more...
///
/// Example:
///     >>> import rowl
///     >>> source = '''
///     ... namespace com.example
///     ...
///     ... ontology Test:
///     ...   concept Person:
///     ...     has name: string
///     ... '''
///     >>> program = rowl.parse(source)
///     >>> for stmt in program.statements:
///     ...     print(stmt.kind)
///     namespace
///     ontology
#[cfg(feature = "python")]
#[pymodule]
fn rowl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Functions
    m.add_function(wrap_pyfunction!(py_parse, m)?)?;
    m.add_function(wrap_pyfunction!(py_parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(py_tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(py_version, m)?)?;

    // AST Classes
    m.add_class::<OntologyFile>()?;
    m.add_class::<QualifiedName>()?;
    m.add_class::<PrefixDecl>()?;
    m.add_class::<Declaration>()?;
    m.add_class::<ConceptDef>()?;
    m.add_class::<ConceptMember>()?;
    m.add_class::<HasDeclaration>()?;
    m.add_class::<PropertyDef>()?;
    m.add_class::<RuleDef>()?;
    m.add_class::<MatchBlock>()?;
    m.add_class::<ThenBlock>()?;
    m.add_class::<ThenItem>()?;
    m.add_class::<Pattern>()?;
    m.add_class::<Subject>()?;
    m.add_class::<Object>()?;
    m.add_class::<Assertion>()?;
    m.add_class::<ConstraintBlock>()?;
    m.add_class::<Constraint>()?;
    m.add_class::<Literal>()?;
    m.add_class::<TypeRef>()?;
    m.add_class::<Cardinality>()?;
    m.add_class::<CardinalityValue>()?;
    m.add_class::<Quantifier>()?;
    m.add_class::<ComparisonOp>()?;

    // Error classes
    m.add_class::<ParseResult>()?;
    m.add_class::<Location>()?;

    m.add_class::<DolfinParseError>()?;
    Ok(())
}
