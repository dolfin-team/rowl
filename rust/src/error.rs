//! Error management for the Dolfin parser.
//!
//! This module defines a rich error hierarchy with:
//! - Source spans (not just points) for precise error highlighting
//! - Error codes for programmatic handling
//! - Severity levels (error, warning, hint)
//! - Contextual information (source lines, suggestions)
//! - Multiple error accumulation

#[cfg(feature = "python")]
use pyo3::exceptions::PyValueError;
#[cfg(feature = "python")]
use pyo3::prelude::*;
use std::{cmp::Ordering, fmt};

use crate::{
    OntologyFile,
    macros::impl_python,
};

// ============================================================================
// SOURCE LOCATION & SPAN
// ============================================================================

/// A single point in source code.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Location {
    /// 1-based line number
    pub line: usize,
    /// 1-based column number
    pub column: usize,
    /// 0-based byte offset from start of source
    pub offset: usize,
}

impl_python! {
#[pymethods]
impl Location {
    /// Create a new source location
    #[new]
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Location(line={}, column={}, offset={})",
            self.line, self.column, self.offset
        )
    }

    fn __str__(&self) -> String {
        format!("{}:{}", self.line, self.column)
    }
}
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.offset.cmp(&other.offset)
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A span covering a range in source code.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// Start location of the span
    pub start: Location,
    /// End location of the span
    pub end: Location,
}

impl_python! {
#[pymethods]
impl Span {
    /// Create a new span
    #[new]
    pub fn new(start: Location, end: Location) -> Self {
        Self { start, end }
    }

    /// Create a zero-width span at a single location (for insertion points).
    #[staticmethod]
    pub fn at(location: Location) -> Self {
        Self {
            start: location,
            end: location,
        }
    }

    /// Merge two spans into one that covers both.
    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Span({}:{} -> {}:{})",
            self.start.line, self.start.column, self.end.line, self.end.column
        )
    }

    fn __str__(&self) -> String {
        if self.start.line == self.end.line {
            format!(
                "{}:{}-{}",
                self.start.line, self.start.column, self.end.column
            )
        } else {
            format!(
                "{}:{}-{}:{}",
                self.start.line, self.start.column, self.end.line, self.end.column
            )
        }
    }
}
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start.line == self.end.line {
            write!(
                f,
                "{}:{}-{}",
                self.start.line, self.start.column, self.end.column
            )
        } else {
            write!(
                f,
                "{}:{}-{}:{}",
                self.start.line, self.start.column, self.end.line, self.end.column
            )
        }
    }
}

impl Ord for Span {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let first_ordering = self.start.offset.cmp(&other.start.offset);
        if first_ordering == Ordering::Equal {
            self.end.offset.cmp(&other.end.offset)
        } else {
            first_ordering
        }
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedString {
    inner: String,
    pub span: Option<Span>,
}

impl SpannedString {
    pub fn new(inner: String, span: Option<Span>) -> Self {
        Self { inner, span }
    }

    pub fn get(&self) -> &String {
        &self.inner
    }
}

impl fmt::Display for SpannedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

// ============================================================================
// ERROR CODES
// ============================================================================

/// Categorized error codes for programmatic handling.
///
/// Error codes follow the pattern:
/// - E1xx: Lexer errors
/// - E2xx: Parser (syntax) errors
/// - E3xx: Semantic errors (structure validation)
#[cfg_attr(feature = "python", pyclass(frozen, eq, eq_int, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // -- Lexer errors (E1xx) --
    /// Unrecognized character in input.
    InvalidCharacter = 100,
    /// Unterminated string literal.
    UnterminatedString = 101,
    /// Invalid numeric literal.
    InvalidNumber = 102,
    /// Invalid escape sequence in string.
    InvalidEscape = 103,
    /// Inconsistent indentation (mixed tabs/spaces or unexpected level).
    InvalidIndentation = 104,

    // -- Parser / syntax errors (E2xx) --
    /// An expected token was not found.
    UnexpectedToken = 200,
    /// Unexpected end of file.
    UnexpectedEof = 201,
    /// Extra token after valid input.
    ExtraToken = 202,
    /// Missing required block (e.g., ontology body).
    MissingBlock = 203,
    /// Missing required field or declaration.
    MissingDeclaration = 204,
    /// Invalid cardinality specification.
    InvalidCardinality = 205,
    /// Invalid type reference.
    InvalidTypeRef = 206,
    /// Malformed qualified name.
    InvalidQualifiedName = 207,
    /// Missing arrow in property definition.
    MissingArrow = 208,
    /// Invalid pattern in match block.
    InvalidPattern = 209,
    /// Invalid assertion in then block.
    InvalidAssertion = 210,

    // -- Semantic / structural errors (E3xx) --
    /// Duplicate definition name.
    DuplicateDefinition = 300,
    /// Reference to an undefined name.
    UndefinedReference = 301,
    /// Circular inheritance detected.
    CircularInheritance = 302,
    /// Type mismatch in property or has declaration.
    TypeMismatch = 303,
    /// Invalid enum variant (e.g., starts with uppercase).
    InvalidEnumVariant = 304,

    // -- Unknown error
    UnknownError = 999,
}

impl_python! {
#[pymethods]
impl ErrorCode {
    /// Get the string code like "E100".
    #[getter]
    fn code(&self) -> String {
        format!("E{}", *self as u16)
    }

    /// Get a short human-readable label.
    #[getter]
    fn label(&self) -> &str {
        match self {
            ErrorCode::InvalidCharacter => "invalid character",
            ErrorCode::UnterminatedString => "unterminated string",
            ErrorCode::InvalidNumber => "invalid number",
            ErrorCode::InvalidEscape => "invalid escape sequence",
            ErrorCode::InvalidIndentation => "invalid indentation",
            ErrorCode::UnexpectedToken => "unexpected token",
            ErrorCode::UnexpectedEof => "unexpected end of file",
            ErrorCode::ExtraToken => "extra token",
            ErrorCode::MissingBlock => "missing block",
            ErrorCode::MissingDeclaration => "missing declaration",
            ErrorCode::InvalidCardinality => "invalid cardinality",
            ErrorCode::InvalidTypeRef => "invalid type reference",
            ErrorCode::InvalidQualifiedName => "invalid qualified name",
            ErrorCode::MissingArrow => "missing arrow",
            ErrorCode::InvalidPattern => "invalid pattern",
            ErrorCode::InvalidAssertion => "invalid assertion",
            ErrorCode::DuplicateDefinition => "duplicate definition",
            ErrorCode::UndefinedReference => "undefined reference",
            ErrorCode::CircularInheritance => "circular inheritance",
            ErrorCode::TypeMismatch => "type mismatch",
            ErrorCode::InvalidEnumVariant => "invalid enum variant",
            ErrorCode::UnknownError => "unknown error",
        }
    }

    fn __repr__(&self) -> String {
        format!("ErrorCode.{} ({})", self.label(), self.code())
    }

    fn __str__(&self) -> String {
        self.code()
    }
}
}
// ============================================================================
// SEVERITY
// ============================================================================

/// Severity level of a diagnostic.
#[cfg_attr(feature = "python", pyclass(frozen, eq, eq_int, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational hint.
    Hint = 0,
    /// Warning – code is valid but may be problematic.
    Warning = 1,
    /// Error – code is invalid.
    HardError = 2,
}

impl_python! {
#[pymethods]
impl Severity {
    fn __repr__(&self) -> &str {
        match self {
            Severity::Hint => "Severity.Hint",
            Severity::Warning => "Severity.Warning",
            Severity::HardError => "Severity.Error",
        }
    }

    fn __str__(&self) -> &str {
        match self {
            Severity::Hint => "hint",
            Severity::Warning => "warning",
            Severity::HardError => "error",
        }
    }
}
}
impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Hint => write!(f, "hint"),
            Severity::Warning => write!(f, "warning"),
            Severity::HardError => write!(f, "error"),
        }
    }
}

// ============================================================================
// DIAGNOSTIC (single error/warning/hint)
// ============================================================================

/// A single diagnostic message with full context.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: Severity,
    /// Machine-readable error code.
    pub code: ErrorCode,
    /// Human-readable primary message.
    pub message: String,
    /// Source span where the error occurred.
    pub span: Option<Span>,
    /// Optional help text suggesting how to fix the issue.
    pub help: Option<String>,
    /// Additional labeled spans providing context.
    pub labels: Vec<DiagnosticLabel>,
}

impl_python! {
#[pymethods]
impl Diagnostic {
    /// Create a new diagnostic
    #[new]
    #[pyo3(signature = (severity, code, message, span=None, help=None, labels=None))]
    pub fn new(
        severity: Severity,
        code: ErrorCode,
        message: String,
        span: Option<Span>,
        help: Option<String>,
        labels: Option<Vec<DiagnosticLabel>>,
    ) -> Self {
        Self {
            severity,
            code,
            message,
            span,
            help,
            labels: labels.unwrap_or_default(),
        }
    }

    /// Is this an error (as opposed to a warning or hint)?
    #[getter]
    fn is_error(&self) -> bool {
        self.severity == Severity::HardError
    }

    /// Format the diagnostic for display, optionally with source context.
    #[pyo3(signature = (source=None, filename=None))]
    pub(crate) fn display(&self, source: Option<&str>, filename: Option<&str>) -> String {
        format_diagnostic(self, source, filename)
    }

    fn __repr__(&self) -> String {
        format!(
            "Diagnostic({}, {}, '{}')",
            self.severity,
            self.code.code(),
            self.message,
        )
    }

    fn __str__(&self) -> String {
        format_diagnostic(self, None, None)
    }
}
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_diagnostic(self, None, None))
    }
}

impl Diagnostic {
    /// Convert from a diagnostic back into a ParseError.
    pub fn into_parse_error(self) -> ParseError {
        let (location, end_location) = match self.span {
            Some(span) => (Some(span.start), Some(span.end)),
            None => (None, None),
        };

        ParseError {
            message: self.message,
            location,
            end_location,
            code: self.code,
            expected: Vec::new(),
            help: self.help,
        }
    }
}

/// A labeled secondary span within a diagnostic.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone)]
pub struct DiagnosticLabel {
    /// The span being labeled
    pub span: Span,
    /// The label message
    pub message: String,
}

impl_python! {
#[pymethods]
impl DiagnosticLabel {
    /// Create a new diagnostic label
    #[new]
    pub fn new(span: Span, message: String) -> Self {
        Self { span, message }
    }

    fn __repr__(&self) -> String {
        format!("DiagnosticLabel({}, '{}')", self.span, self.message)
    }
}
}
// ============================================================================
// DIAGNOSTIC BUILDER (fluent API)
// ============================================================================

/// Builder for constructing diagnostics with a fluent API.
///
/// # Example (Rust side)
/// ```ignore
/// DiagnosticBuilder::error(ErrorCode::UnexpectedToken, "Expected ':' after concept name")
///     .span(span)
///     .help("Add a colon after the concept name")
///     .label(some_span, "concept name starts here")
///     .build()
/// ```
pub struct DiagnosticBuilder {
    severity: Severity,
    code: ErrorCode,
    message: String,
    span: Option<Span>,
    help: Option<String>,
    labels: Vec<DiagnosticLabel>,
}

impl DiagnosticBuilder {
    /// Create a new diagnostic builder
    pub fn new(severity: Severity, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            span: None,
            help: None,
            labels: vec![],
        }
    }

    /// Create an error diagnostic
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(Severity::HardError, code, message)
    }

    /// Create a warning diagnostic
    pub fn warning(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    /// Create a hint diagnostic
    pub fn hint(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(Severity::Hint, code, message)
    }

    /// Set the span for this diagnostic
    pub fn span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Set the span (optional) for this diagnostic
    pub fn span_opt(mut self, span: Option<Span>) -> Self {
        self.span = span;
        self
    }

    /// Set a single-point location for this diagnostic
    pub fn at(mut self, location: Location) -> Self {
        self.span = Some(Span::at(location));
        self
    }

    /// Add help text to this diagnostic
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add a labeled secondary span to this diagnostic
    pub fn label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            message: message.into(),
        });
        self
    }

    /// Build the final diagnostic
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            severity: self.severity,
            code: self.code,
            message: self.message,
            span: self.span,
            help: self.help,
            labels: self.labels,
        }
    }
}

// ============================================================================
// PARSE RESULT (accumulates multiple diagnostics)
// ============================================================================

/// Result of a parse operation, holding both the (possibly partial) AST
/// and all accumulated diagnostics.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The parsed ontology, if parsing succeeded (possibly with warnings).
    pub ontology: Option<crate::ast::OntologyFile>,
    /// All diagnostics collected during parsing.
    pub diagnostics: Vec<Diagnostic>,
}

impl_python! {
#[pymethods]
impl ParseResult {
    /// Were there any errors (not just warnings)?
    #[getter]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::HardError)
    }

    /// Were there any warnings?
    #[getter]
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning)
    }

    /// Get only error-level diagnostics.
    #[getter]
    pub fn errors(&self) -> Vec<Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::HardError)
            .cloned()
            .collect()
    }

    /// Get only warning-level diagnostics.
    #[getter]
    pub fn warnings(&self) -> Vec<Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .cloned()
            .collect()
    }

    /// Number of errors.
    #[getter]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::HardError)
            .count()
    }

    /// Number of warnings.
    #[getter]
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Is the parse result successful (has an ontology and no errors)?
    #[getter]
    pub fn is_ok(&self) -> bool {
        self.ontology.is_some() && !self.has_errors()
    }

    /// Format all diagnostics for display.
    #[pyo3(signature = (source=None, filename=None))]
    pub fn format_diagnostics(&self, source: Option<&str>, filename: Option<&str>) -> String {
        self.diagnostics
            .iter()
            .map(|d| format_diagnostic(d, source, filename))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn __repr__(&self) -> String {
        format!(
            "ParseResult(ok={}, errors={}, warnings={})",
            self.is_ok(),
            self.error_count(),
            self.warning_count(),
        )
    }

    fn __str__(&self) -> String {
        if self.is_ok() {
            format!("ParseResult: OK ({} warnings)", self.warning_count())
        } else {
            format!(
                "ParseResult: FAILED ({} errors, {} warnings)",
                self.error_count(),
                self.warning_count()
            )
        }
    }
}
}

impl ParseResult {
    /// Map errors to a different error type
    pub fn map_err<F, O: FnOnce(ParseError) -> F>(self, op: O) -> Result<OntologyFile, F> {
        match self.ontology {
            Some(ontology) if self.is_ok() => Ok(ontology),
            _ => Err(op(self
                .errors()
                .into_iter()
                .next()
                .unwrap()
                .into_parse_error())),
        }
    }
}

impl ParseResult {
    /// Create a successful result.
    pub fn success(ontology: crate::ast::OntologyFile, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            ontology: Some(ontology),
            diagnostics,
        }
    }

    /// Create a failed result.
    pub fn failure(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            ontology: None,
            diagnostics,
        }
    }
}

// ============================================================================
// LEGACY ERROR TYPES (for backward compatibility with lexer/parser internals)
// ============================================================================

/// Error produced by the lexer. Carries enough information to convert
/// into a `Diagnostic`.

#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    /// Error message
    pub message: String,
    /// Location where the error occurred
    pub location: Location,
    /// Error code
    pub code: ErrorCode,
}

impl LexerError {
    /// Create a new lexer error
    pub fn new(message: impl Into<String>, location: Location, code: ErrorCode) -> Self {
        Self {
            message: message.into(),
            location,
            code,
        }
    }

    /// Convert to a rich diagnostic.
    pub fn into_diagnostic(self) -> Diagnostic {
        DiagnosticBuilder::error(self.code, self.message)
            .at(self.location)
            .build()
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] at {}: {}",
            self.code.code(),
            self.location,
            self.message
        )
    }
}

impl std::error::Error for LexerError {}

/// Error produced during parsing. Can be converted into a `Diagnostic`.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Start location of the error
    pub location: Option<Location>,
    /// End location of the error
    pub end_location: Option<Location>,
    /// Error code
    pub code: ErrorCode,
    /// Expected tokens (for syntax errors)
    pub expected: Vec<String>,
    /// Help text
    pub help: Option<String>,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: impl Into<String>, code: ErrorCode) -> Self {
        Self {
            message: message.into(),
            location: None,
            end_location: None,
            code,
            expected: vec![],
            help: None,
        }
    }

    /// Add a location to this error
    pub fn with_location(mut self, loc: Location) -> Self {
        self.location = Some(loc);
        self
    }

    /// Add a span to this error
    pub fn with_span(mut self, start: Location, end: Location) -> Self {
        self.location = Some(start);
        self.end_location = Some(end);
        self
    }

    /// Add expected tokens to this error
    pub fn with_expected(mut self, expected: Vec<String>) -> Self {
        self.expected = expected;
        self
    }

    /// Add help text to this error
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Convert to a rich diagnostic.
    pub fn into_diagnostic(self) -> Diagnostic {
        let mut builder = DiagnosticBuilder::error(self.code, &self.message);

        // Set span
        if let Some(start) = self.location {
            let end = self.end_location.unwrap_or(start);
            builder = builder.span(Span { start, end });
        }

        // Add help about expected tokens if available
        if !self.expected.is_empty() {
            let expected_str = humanize_expected(&self.expected);
            let help_msg = if let Some(existing_help) = &self.help {
                format!("{}. Expected {}", existing_help, expected_str)
            } else {
                format!("Expected {}", expected_str)
            };
            builder = builder.help(help_msg);
        } else if let Some(help) = &self.help {
            builder = builder.help(help.as_str());
        }

        builder.build()
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(loc) = &self.location {
            write!(f, "[{}] at {}: {}", self.code.code(), loc, self.message)?;
        } else {
            write!(f, "[{}]: {}", self.code.code(), self.message)?;
        }
        if !self.expected.is_empty() {
            write!(f, " (expected {})", humanize_expected(&self.expected))?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

#[cfg(feature = "python")]
impl From<ParseError> for PyErr {
    fn from(error: ParseError) -> PyErr {
        PyValueError::new_err(error.to_string())
    }
}

// ============================================================================
// FORMATTING
// ============================================================================

/// Format a list of expected tokens into a human-readable string.
fn humanize_expected(expected: &[String]) -> String {
    // Clean up LALRPOP-style token names
    let cleaned: Vec<String> = expected.iter().map(|e| humanize_token_name(e)).collect();

    match cleaned.len() {
        0 => "nothing".to_string(),
        1 => cleaned[0].clone(),
        2 => format!("{} or {}", cleaned[0], cleaned[1]),
        _ => {
            let (last, rest) = cleaned.split_last().unwrap();
            format!("{}, or {}", rest.join(", "), last)
        }
    }
}

/// Clean up a token name from LALRPOP's internal representation.
fn humanize_token_name(name: &str) -> String {
    // Strip surrounding quotes if present
    let name = name.trim_matches('"');
    match name {
        "NEWLINE" | "Newline" => "newline".to_string(),
        "INDENT" | "Indent" => "indented block".to_string(),
        "DEDENT" | "Dedent" => "end of block".to_string(),
        "EOF" | "Eof" => "end of file".to_string(),
        "NAME" | "Name" => "identifier".to_string(),
        "INT" | "Int" => "integer".to_string(),
        "FLOAT" | "Float" => "number".to_string(),
        "STRING" | "String" => "string literal".to_string(),
        "VARIABLE" | "Variable" => "variable (e.g., ?x)".to_string(),
        "COLON" | "Colon" | ":" => "':'".to_string(),
        "COMMA" | "Comma" | "," => "','".to_string(),
        "DOT" | "Dot" | "." => "'.'".to_string(),
        "ARROW" | "Arrow" | "->" => "'->'".to_string(),
        "PIPE" | "Pipe" | "|" => "'|'".to_string(),
        "LBRACKET" | "LBracket" | "[" => "'['".to_string(),
        "RBRACKET" | "RBracket" | "]" => "']'".to_string(),
        "STAR" | "Star" | "*" => "'*'".to_string(),
        "namespace" | "Namespace" => "'namespace'".to_string(),
        "import" | "Import" => "'import'".to_string(),
        "ontology" | "Ontology" => "'ontology'".to_string(),
        "concept" | "Concept" => "'concept'".to_string(),
        "property" | "Property" => "'property'".to_string(),
        "enum" | "Enum" => "'enum'".to_string(),
        "rule" | "Rule" => "'rule'".to_string(),
        "match" | "Match" => "'match'".to_string(),
        "then" | "Then" => "'then'".to_string(),
        "sub" | "Sub" => "'sub'".to_string(),
        "has" | "Has" => "'has'".to_string(),
        "is" | "Is" => "'is'".to_string(),
        "as" | "As" => "'as'".to_string(),
        other => format!("'{}'", other),
    }
}

/// Format a diagnostic for display with optional source context.
fn format_diagnostic(diag: &Diagnostic, source: Option<&str>, filename: Option<&str>) -> String {
    let mut out = String::new();

    // Header: severity[code]: message
    out.push_str(&format!(
        "{}[{}]: {}",
        diag.severity,
        diag.code.code(),
        diag.message,
    ));

    // Location line
    if let Some(span) = &diag.span {
        let file = filename.unwrap_or("<input>");
        out.push_str(&format!("\n  --> {}:{}", file, span.start));
    }

    // Source context with underline
    if let (Some(source), Some(span)) = (source, &diag.span)
        && let Some(context) = format_source_context(source, span)
    {
        out.push('\n');
        out.push_str(&context);
    }

    // Secondary labels
    if let Some(source) = source {
        for label in &diag.labels {
            out.push_str(&format!("\n  --> {}", label.span.start));
            if let Some(context) = format_source_context(source, &label.span) {
                out.push('\n');
                out.push_str(&context);
            }
            out.push_str(&format!("\n  = {}", label.message));
        }
    }

    // Help text
    if let Some(help) = &diag.help {
        out.push_str(&format!("\n  = help: {}", help));
    }

    out
}

/// Render source context with underline markers for a given span.
///
/// Example output:
/// ```text
///    |
///  5 |   concept Person
///    |           ^^^^^^
/// ```
fn format_source_context(source: &str, span: &Span) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = span.start.line.checked_sub(1)?;

    if line_idx >= lines.len() {
        return None;
    }

    let line = lines[line_idx];
    let line_num = span.start.line;
    let gutter_width = format!("{}", line_num).len();

    let mut out = String::new();

    // Blank gutter line
    out.push_str(&format!("{:>width$} |\n", "", width = gutter_width));

    // Source line
    out.push_str(&format!(
        "{:>width$} | {}\n",
        line_num,
        line,
        width = gutter_width
    ));

    // Underline
    let col_start = span.start.column.saturating_sub(1);
    let col_end = if span.start.line == span.end.line {
        span.end.column.saturating_sub(1).max(col_start + 1)
    } else {
        line.len()
    };
    let underline_len = col_end.saturating_sub(col_start).max(1);

    out.push_str(&format!(
        "{:>width$} | {:>pad$}{}",
        "",
        "",
        "^".repeat(underline_len),
        width = gutter_width,
        pad = col_start,
    ));

    Some(out)
}

// ============================================================================
// CONVERSIONS → dolfin_diagnostic
// ============================================================================

impl From<Location> for dolfin_diagnostic::Location {
    fn from(l: Location) -> Self {
        dolfin_diagnostic::Location {
            line: l.line,
            column: l.column,
            offset: l.offset,
        }
    }
}

impl From<dolfin_diagnostic::Location> for Location {
    fn from(l: dolfin_diagnostic::Location) -> Self {
        Location {
            line: l.line,
            column: l.column,
            offset: l.offset,
        }
    }
}

impl From<Span> for dolfin_diagnostic::Span {
    fn from(s: Span) -> Self {
        dolfin_diagnostic::Span {
            start: s.start.into(),
            end: s.end.into(),
        }
    }
}

impl From<dolfin_diagnostic::Span> for Span {
    fn from(s: dolfin_diagnostic::Span) -> Self {
        Span {
            start: s.start.into(),
            end: s.end.into(),
        }
    }
}

impl From<Severity> for dolfin_diagnostic::Severity {
    fn from(s: Severity) -> Self {
        match s {
            Severity::HardError => dolfin_diagnostic::Severity::Error,
            Severity::Warning => dolfin_diagnostic::Severity::Warning,
            Severity::Hint => dolfin_diagnostic::Severity::Hint,
        }
    }
}

impl From<&Diagnostic> for dolfin_diagnostic::Diagnostic {
    fn from(d: &Diagnostic) -> Self {
        dolfin_diagnostic::DiagnosticBuilder::new(
            d.severity.into(),
            dolfin_diagnostic::DiagnosticCode::Parse(d.code as u16),
            d.message.clone(),
        )
        .span_opt(d.span.map(Into::into))
        .build()
    }
}

impl ParseResult {
    /// Return all diagnostics as unified `dolfin_diagnostic::Diagnostic` objects.
    ///
    /// Use this in Rust-only code (LSP, raft) to get a homogeneous type.
    /// The Python-facing `diagnostics` field retains its original type.
    pub fn unified_diagnostics(&self) -> Vec<dolfin_diagnostic::Diagnostic> {
        self.diagnostics.iter().map(Into::into).collect()
    }
}

// ============================================================================
// PYTHON EXCEPTION INTEGRATION
// ============================================================================

/// A Python-friendly parse exception that wraps all diagnostics.
///
/// This is raised when calling `parse()` and errors are found,
/// allowing Python code to inspect both the formatted message
/// and structured diagnostic data.
#[cfg(feature = "python")]
#[pyclass(extends=pyo3::exceptions::PyException, from_py_object)]
#[derive(Debug, Clone)]
pub struct DolfinParseError {
    /// All diagnostics from parsing
    #[pyo3(get)]
    pub diagnostics: Vec<Diagnostic>,
    /// Source code (if available)
    #[pyo3(get)]
    pub source: Option<String>,
    /// Filename (if available)
    #[pyo3(get)]
    pub filename: Option<String>,
}

#[cfg(not(feature = "python"))]
#[derive(Debug, Clone)]
pub struct DolfinParseError {
    /// All diagnostics from parsing
    pub diagnostics: Vec<Diagnostic>,
    /// Source code (if available)
    pub source: Option<String>,
    /// Filename (if available)
    pub filename: Option<String>,
}

impl_python! {
#[pymethods]
impl DolfinParseError {
    #[new]
    #[pyo3(signature = (diagnostics, source=None, filename=None))]
    fn new(diagnostics: Vec<Diagnostic>, source: Option<String>, filename: Option<String>) -> Self {
        Self {
            diagnostics,
            source,
            filename,
        }
    }

    /// Number of errors.
    #[getter]
    fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::HardError)
            .count()
    }

    /// Format all diagnostics for display.
    fn format(&self) -> String {
        self.diagnostics
            .iter()
            .map(|d| format_diagnostic(d, self.source.as_deref(), self.filename.as_deref()))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn __str__(&self) -> String {
        self.format()
    }
}
}
// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_display() {
        let loc = Location::new(5, 10, 42);
        assert_eq!(format!("{}", loc), "5:10");
    }

    #[test]
    fn test_span_display_single_line() {
        let span = Span::new(Location::new(5, 3, 20), Location::new(5, 10, 27));
        assert_eq!(format!("{}", span), "5:3-10");
    }

    #[test]
    fn test_span_display_multi_line() {
        let span = Span::new(Location::new(5, 3, 20), Location::new(7, 1, 50));
        assert_eq!(format!("{}", span), "5:3-7:1");
    }

    #[test]
    fn test_span_merge() {
        let a = Span::new(Location::new(1, 5, 4), Location::new(1, 10, 9));
        let b = Span::new(Location::new(1, 1, 0), Location::new(1, 7, 6));
        let merged = a.merge(&b);
        assert_eq!(merged.start.offset, 0);
        assert_eq!(merged.end.offset, 9);
    }

    #[test]
    fn test_diagnostic_builder() {
        let diag = DiagnosticBuilder::error(ErrorCode::UnexpectedToken, "Expected ':'")
            .span(Span::new(
                Location::new(3, 15, 30),
                Location::new(3, 20, 35),
            ))
            .help("Add a colon after the concept name")
            .build();

        assert_eq!(diag.severity, Severity::HardError);
        assert_eq!(diag.code, ErrorCode::UnexpectedToken);
        assert!(diag.span.is_some());
        assert!(diag.help.is_some());
    }

    #[test]
    fn test_humanize_expected() {
        assert_eq!(humanize_expected(&[]), "nothing");
        assert_eq!(humanize_expected(&["Colon".into()]), "':'");
        assert_eq!(
            humanize_expected(&["Colon".into(), "Name".into()]),
            "':' or identifier"
        );
        assert_eq!(
            humanize_expected(&["Colon".into(), "Name".into(), "Newline".into()]),
            "':', identifier, or newline"
        );
    }

    #[test]
    fn test_format_source_context() {
        let source = "namespace com.example\nontology Foo:\n  concept Bar\n";
        let span = Span::new(Location::new(3, 11, 36), Location::new(3, 14, 39));
        let ctx = format_source_context(source, &span).unwrap();
        assert!(ctx.contains("concept Bar"));
        assert!(ctx.contains("^^^"));
    }

    #[test]
    fn test_parse_error_to_diagnostic() {
        let err = ParseError::new("Unexpected token 'foo'", ErrorCode::UnexpectedToken)
            .with_location(Location::new(5, 3, 40))
            .with_expected(vec!["Colon".into(), "Name".into()]);

        let diag = err.into_diagnostic();
        assert_eq!(diag.severity, Severity::HardError);
        assert_eq!(diag.code, ErrorCode::UnexpectedToken);
        assert!(diag.help.is_some());
        assert!(diag.help.unwrap().contains("':'"));
    }

    #[test]
    fn test_lexer_error_to_diagnostic() {
        let err = LexerError::new(
            "Unrecognized character: @",
            Location::new(1, 5, 4),
            ErrorCode::InvalidCharacter,
        );
        let diag = err.into_diagnostic();
        assert_eq!(diag.code, ErrorCode::InvalidCharacter);
        assert!(diag.span.is_some());
    }

    #[test]
    fn test_full_diagnostic_formatting() {
        let source = "namespace com.example\nontology Foo:\n  concept Bar\n";
        let diag = DiagnosticBuilder::error(
            ErrorCode::MissingBlock,
            "Expected indented block after 'concept Bar'",
        )
        .span(Span::new(
            Location::new(3, 11, 36),
            Location::new(3, 14, 39),
        ))
        .help("Add declarations like 'has' or 'sub' indented under the concept")
        .build();

        let formatted = format_diagnostic(&diag, Some(source), Some("test.dlf"));
        assert!(formatted.contains("error[E203]"));
        assert!(formatted.contains("test.dlf:3:11"));
        assert!(formatted.contains("concept Bar"));
        assert!(formatted.contains("^^^"));
        assert!(formatted.contains("help:"));
    }

    #[test]
    fn test_parse_result_success() {
        let ontology = crate::ast::OntologyFile {
            iri_name: None,
            prefixes: vec![],
            declarations: vec![],
            locale: None,
            timezone: None,
            span: None,
        };
        let result = ParseResult::success(ontology, vec![]);
        assert!(result.is_ok());
        assert!(!result.has_errors());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_parse_result_failure() {
        let diag = DiagnosticBuilder::error(ErrorCode::UnexpectedToken, "bad token").build();
        let result = ParseResult::failure(vec![diag]);
        assert!(!result.is_ok());
        assert!(result.has_errors());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_error_code_values() {
        assert_eq!(ErrorCode::InvalidCharacter as u16, 100);
        assert_eq!(ErrorCode::UnexpectedToken as u16, 200);
        assert_eq!(ErrorCode::DuplicateDefinition as u16, 300);
    }

    #[test]
    fn test_error_code_string() {
        assert_eq!(ErrorCode::UnexpectedToken.code(), "E200");
        assert_eq!(ErrorCode::InvalidCharacter.code(), "E100");
        assert_eq!(ErrorCode::DuplicateDefinition.code(), "E300");
    }
}

#[cfg(test)]
mod span_tests {
    use crate::parser;

    #[test]
    fn test_concept_span_covers_full_definition() {
        let source = "concept Animal:\n  has name: string\n";
        let result = parser::parse_ontology(source);
        let onto = result.ontology.unwrap();
        let concept = &onto.concepts_as_ref()[0];

        let span = concept.span.expect("concept should have a span");
        // Le span doit commencer au 'c' de 'concept' (offset 0)
        assert_eq!(span.start.offset, 0);
        // Et couvrir jusqu'à la fin du bloc
        assert!(span.end.offset > 0);

        // Vérifier que le slice correspond
        let slice = &source[span.start.offset..span.end.offset];
        assert!(slice.starts_with("concept Animal"));
    }

    #[test]
    fn test_has_declaration_span() {
        let source = "concept Foo:\n  has bar: string\n";
        let result = parser::parse_ontology(source);
        let onto = result.ontology.unwrap();
        let concept = &onto.concepts_as_ref()[0];

        if let Some(has) = &concept.has_declarations.iter().nth(0) {
            let span = has.span.expect("has should have a span");
            let slice = &source[span.start.offset..span.end.offset];
            assert!(slice.contains("has bar: string"));
        } else {
            panic!("Expected Has member");
        }
    }

    #[test]
    fn test_prefix_span() {
        let source = r#"@iri_name "http://example.org/"
prefix <http://foo.org#> as foo

concept X:
  has y: string
"#;
        let result = parser::parse_ontology(source);
        let onto = result.ontology.unwrap();
        let prefix = &onto.prefixes[0];

        let span = prefix.span.expect("prefix should have span");
        let slice = &source[span.start.offset..span.end.offset];
        assert_eq!(slice, "<http://foo.org#> as foo");
    }
}
