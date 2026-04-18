//! Core parsing logic for Dolfin.

use lalrpop_util::lalrpop_mod;

// Auto-generated LALRPOP parser for Dolfin language
lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(dead_code)]
    pub dolfin,
    "/dolfin.rs"
);

use crate::PackageError;
use crate::ast::{OntologyFile, PackageFile};
use crate::comment::{Comment, CommentSink};
use crate::error::{
    /* Diagnostic,  */ DiagnosticBuilder, ErrorCode, LexerError, Location, ParseError,
    ParseResult, /*Severity , Span, */
};
use crate::lexer::{Lexer, Token};

/// Result of parsing with comment preservation.
pub struct ParseWithComments {
    pub result: ParseResult,
    pub comments: Vec<Comment>,
}

/// Parse and collect comments for formatting.
pub fn parse_ontology_with_comments(source: &str) -> ParseWithComments {
    let sink = CommentSink::new();
    let lexer = Lexer::with_comment_sink(source, sink.clone());
    let parser = dolfin::OntologyFileParser::new();

    let result = match parser.parse(lexer) {
        Ok(ontology) => ParseResult::success(ontology, vec![]),
        Err(lalrpop_err) => {
            let parse_error = convert_lalrpop_error(lalrpop_err, source);
            let diagnostic = parse_error.into_diagnostic();
            ParseResult::failure(vec![diagnostic])
        }
    };

    // Comments survive because sink has been cloned (Rc)
    ParseWithComments {
        result,
        comments: sink.take(),
    }
}

/// Parse Dolfin source code from a string.
pub fn parse_ontology(source: &str) -> ParseResult {
    let lexer = Lexer::new(source);
    let parser = dolfin::OntologyFileParser::new();

    match parser.parse(lexer) {
        Ok(ontology) => ParseResult::success(ontology, vec![]),
        Err(lalrpop_err) => {
            let parse_error = convert_lalrpop_error(lalrpop_err, source);
            let diagnostic = parse_error.into_diagnostic();
            ParseResult::failure(vec![diagnostic])
        }
    }
}

/// Parse Dolfin source code and return a `Result` (legacy API).
///
/// Returns `Ok(Ontology)` on success, or `Err(ParseError)` on failure.
/// Prefer `parse()` for richer diagnostics.
pub fn parse_result_strict(result: ParseResult) -> Result<OntologyFile, Box<ParseError>> {
    match result.ontology {
        Some(ontology) if !result.has_errors() => Ok(ontology),
        _ => {
            let first_error = result
                .errors()
                .into_iter()
                .next()
                .expect("parse failed but no errors collected");

            Err(Box::new(ParseError {
                message: first_error.message.clone(),
                location: first_error.span.map(|s| s.start),
                end_location: first_error.span.map(|s| s.end),
                code: first_error.code,
                expected: vec![],
                help: first_error.help.clone(),
            }))
        }
    }
}

/// Parse Dolfin source code from a file.
pub fn parse_ontology_file<P: AsRef<std::path::Path>>(path: P) -> ParseResult {
    let path_ref = path.as_ref();
    let source = match std::fs::read_to_string(path_ref) {
        Ok(s) => s,
        Err(e) => {
            let diag = DiagnosticBuilder::error(
                ErrorCode::UnexpectedEof, // closest code; could add a dedicated IO code
                format!("Failed to read file '{}': {}", path_ref.display(), e),
            )
            .build();
            return ParseResult::failure(vec![diag]);
        }
    };
    parse_ontology(&source)
}

/// Parse a Dolfin package manifest (package.dlf).
pub fn parse_package(source: &str) -> Result<PackageFile, Box<ParseError>> {
    let lexer = Lexer::new(source);
    let parser = dolfin::PackageFileParser::new();

    parser
        .parse(lexer)
        .map_err(|e| convert_lalrpop_error(e, source))
}

/// Parse a package manifest from disk.
pub fn parse_package_file<P: AsRef<std::path::Path>>(path: P) -> Result<PackageFile, PackageError> {
    let source = std::fs::read_to_string(path.as_ref()).map_err(|e| PackageError::IoError {
        path: path.as_ref().to_path_buf(),
        source: e,
    })?;
    parse_package(&source).map_err(|e| PackageError::ParseError {
        path: path.as_ref().to_path_buf(),
        source: e,
    })
}

/// Convert a LALRPOP error into our `ParseError` with rich context.
fn convert_lalrpop_error(
    error: lalrpop_util::ParseError<Location, Token, LexerError>,
    source: &str,
) -> Box<ParseError> {
    match error {
        lalrpop_util::ParseError::InvalidToken { location } => Box::new(
            ParseError::new("Invalid token", ErrorCode::UnexpectedToken).with_location(location),
        ),

        lalrpop_util::ParseError::UnrecognizedEof { location, expected } => {
            let help = suggest_for_eof(source, &location);
            let mut err = ParseError::new("Unexpected end of file", ErrorCode::UnexpectedEof)
                .with_location(location)
                .with_expected(expected);
            if let Some(h) = help {
                err = err.with_help(h);
            }
            Box::new(err)
        }

        lalrpop_util::ParseError::UnrecognizedToken {
            token: (start, ref tok, end),
            expected,
        } => {
            let message = format_unexpected_token_message(tok, source, &start);
            let help = suggest_for_unexpected_token(tok, &expected, source, &start);
            let code = classify_unexpected_token(tok);

            let mut err = ParseError::new(message, code)
                .with_span(start, end)
                .with_expected(expected);
            if let Some(h) = help {
                err = err.with_help(h);
            }
            Box::new(err)
        }

        lalrpop_util::ParseError::ExtraToken {
            token: (start, ref tok, end),
        } => Box::new(
            ParseError::new(
                format!("Extra token '{}' after valid input", tok),
                ErrorCode::ExtraToken,
            )
            .with_span(start, end)
            .with_help("Remove this token or check for a missing newline"),
        ),

        lalrpop_util::ParseError::User { error } => Box::new(
            ParseError::new(error.message.clone(), error.code).with_location(error.location),
        ),
    }
}

/// Produce a context-aware message for unexpected tokens.
fn format_unexpected_token_message(tok: &Token, source: &str, loc: &Location) -> String {
    let context = identify_parsing_context(source, loc);
    match context {
        ParsingContext::ConceptBody => {
            format!("Unexpected '{}' in concept body", tok)
        }
        ParsingContext::OntologyBody => {
            format!("Unexpected '{}' in ontology body", tok)
        }
        ParsingContext::EnumBody => {
            format!("Unexpected '{}' in enum definition", tok)
        }
        ParsingContext::MatchBlock => {
            format!("Unexpected '{}' in match block", tok)
        }
        ParsingContext::ThenBlock => {
            format!("Unexpected '{}' in then block", tok)
        }
        ParsingContext::PropertyDef => {
            format!("Unexpected '{}' in property definition", tok)
        }
        ParsingContext::TopLevel => match tok {
            Token::Indent => {
                "Unexpected indentation at top level, did you forgot the ':' ?".to_string()
            }
            _ => format!("Unexpected '{}' at top level", tok),
        },
        ParsingContext::Unknown => {
            format!("Unexpected token '{}'", tok)
        }
    }
}

/// Classify an unexpected token into a more specific error code when possible.
fn classify_unexpected_token(tok: &Token) -> ErrorCode {
    match tok {
        Token::Arrow => ErrorCode::MissingArrow, // arrow in wrong place
        Token::Indent | Token::Dedent => ErrorCode::InvalidIndentation,
        _ => ErrorCode::UnexpectedToken,
    }
}

/// Suggest fixes for unexpected tokens based on context.
fn suggest_for_unexpected_token(
    tok: &Token,
    expected: &[String],
    source: &str,
    loc: &Location,
) -> Option<String> {
    // Check for common mistakes
    match tok {
        Token::Name(_) => {
            // Check if a colon was expected (common: `concept Foo` without `:`)
            if expected
                .iter()
                .any(|e| e.contains("Colon") || e.contains(":"))
            {
                return Some(
                    "Did you forget a ':' ? Blocks require a colon, e.g., 'concept Foo:'"
                        .to_string(),
                );
            }
        }
        Token::Newline => {
            if expected
                .iter()
                .any(|e| e.contains("Indent") || e.contains("INDENT"))
            {
                return Some("Expected an indented block on the next line".to_string());
            }
        }
        Token::Dedent => {
            return Some(
                "Unexpected decrease in indentation. Check that your block is properly indented"
                    .to_string(),
            );
        }
        Token::Arrow => {
            let ctx = identify_parsing_context(source, loc);
            if ctx == ParsingContext::ConceptBody {
                return Some("'->' is used in property definitions, not in concept bodies. Use 'has' for concept attributes".to_string());
            }
        }
        _ => {}
    }

    None
}

/// Suggest fixes for unexpected EOF.
fn suggest_for_eof(source: &str, _loc: &Location) -> Option<String> {
    // Check if there's an unclosed block
    let lines: Vec<&str> = source.lines().collect();
    if let Some(last_line) = lines.last() {
        let trimmed = last_line.trim();
        if trimmed.ends_with(':') {
            return Some(format!(
                "The block starting with '{}' needs an indented body",
                trimmed,
            ));
        }
    }

    // Check indent stack depth heuristic: if source has indented content
    // at the end, maybe a DEDENT is missing
    let trailing = source.trim_end();
    if !trailing.is_empty() {
        let last_line = trailing.lines().last().unwrap_or("");
        let indent = last_line.len() - last_line.trim_start().len();
        if indent > 0 {
            return Some(
                "The file ends inside an indented block. Ensure all blocks are properly closed"
                    .to_string(),
            );
        }
    }

    None
}

// ============================================================================
// PARSING CONTEXT DETECTION
// ============================================================================

/// Rough classification of where in the grammar the parser currently is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsingContext {
    TopLevel,
    OntologyBody,
    ConceptBody,
    EnumBody,
    MatchBlock,
    ThenBlock,
    PropertyDef,
    Unknown,
}

/// Heuristic: look at preceding lines to determine the parsing context.
fn identify_parsing_context(source: &str, loc: &Location) -> ParsingContext {
    let lines: Vec<&str> = source.lines().collect();
    let target_line = loc.line.saturating_sub(1); // 0-based

    // Walk backward from the error line looking for context clues
    let mut current_indent = if target_line < lines.len() {
        let line = lines[target_line];
        line.len() - line.trim_start().len()
    } else {
        0
    };

    for i in (0..=target_line.min(lines.len().saturating_sub(1))).rev() {
        let line = lines[i];
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();

        // Only consider lines at a lesser or equal indent (enclosing blocks)
        if indent < current_indent || i == target_line {
            if trimmed.starts_with("concept ") && trimmed.ends_with(':') {
                return ParsingContext::ConceptBody;
            }
            if trimmed.starts_with("ontology ") && trimmed.ends_with(':') {
                return ParsingContext::OntologyBody;
            }
            if trimmed.starts_with("enum ") && trimmed.ends_with(':') {
                return ParsingContext::EnumBody;
            }
            if trimmed.starts_with("match") && trimmed.ends_with(':') {
                return ParsingContext::MatchBlock;
            }
            if trimmed.starts_with("then") && trimmed.ends_with(':') {
                return ParsingContext::ThenBlock;
            }
            if trimmed.starts_with("property ") {
                return ParsingContext::PropertyDef;
            }
            current_indent = indent;
        }
    }

    // Check if we're at top level (no indentation)
    if current_indent == 0 {
        return ParsingContext::TopLevel;
    }

    ParsingContext::Unknown
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use crate::error::{Severity, Span};

    #[test]
    fn test_parse_empty() {
        let result = parse_ontology("");
        assert!(result.is_ok());
        let onto = result.ontology.unwrap();
        assert!(onto.iri_name.is_none());
        assert!(onto.prefixes.is_empty());
        assert!(onto.declarations.is_empty());
    }

    #[test]
    fn test_parse_error_has_diagnostic() {
        let result = parse_ontology("namespace\n"); // missing qualified name
        assert!(!result.is_ok());
        assert!(result.has_errors());
        assert_eq!(result.error_count(), 1);

        let diag = &result.diagnostics[0];
        assert_eq!(diag.severity, Severity::HardError);
        assert!(diag.span.is_some());
    }

    #[test]
    fn test_parse_error_shows_source_context() {
        let source = "concept Bar:\n    has name string\n";
        let result = parse_ontology(source);

        if result.has_errors() {
            for diag in &result.diagnostics {
                let formatted = diag.display(Some(source), Some("test.dlf"));
                // Should contain file reference
                assert!(
                    formatted.contains("test.dlf"),
                    "Missing filename in: {}",
                    formatted
                );
                // Should contain error code
                assert!(
                    formatted.contains("[E"),
                    "Missing error code in: {}",
                    formatted
                );
            }
        }
    }

    #[test]
    fn test_parse_iri_name() {
        let source = r#"@iri_name "Mammifère"

concept Mammal:
  has name: string
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.iri_name, Some("Mammifère".to_string()));
        assert_eq!(onto.declarations.len(), 1);
    }

    #[test]
    fn test_parse_prefixes() {
        let source = r#"prefix com.example.common
prefix com.example.other as other

concept Thing:
  has name: string
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.prefixes.len(), 2);
        assert_eq!(onto.prefixes[0].alias, "common");
        assert_eq!(onto.prefixes[1].alias, "other");
    }

    #[test]
    fn test_parse_package() {
        let source = r#"package com.example.biology:
  dolfin_version "1"
  version "1.0.0"
  author "Jane Doe"
  description "Biology ontology"
"#;
        let result = parse_package(source);
        assert!(result.is_ok(), "Error: {:?}", result.err());
        let pkg = result.unwrap();
        assert_eq!(pkg.name.full(), "com.example.biology");
        assert_eq!(pkg.dolfin_version, "1");
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.author, Some("Jane Doe".to_string()));
    }

    #[test]
    fn test_parse_error_expected_colon() {
        // Missing colon after ontology name
        let source = "concept Bar\n    has name: string\n";
        let result = parse_ontology(source);
        assert!(!result.is_ok());

        let diag = &result.diagnostics[0];
        let formatted = diag.display(Some(source), None);
        // The help should mention the colon
        assert!(
            formatted.contains("':'"),
            "expected a colon -> <{}>",
            formatted
        );
    }

    #[test]
    fn test_parse_file_not_found() {
        let result = parse_ontology_file("/nonexistent/file.dlf");
        assert!(!result.is_ok());
        assert!(result.has_errors());
    }

    #[test]
    fn test_parse_simple_ontology() {
        let source = "concept Bar:\n  has name: string\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
    }

    #[test]
    fn test_parse_concept_with_inheritance() {
        let source = r#"concept Employee:
  sub Person
  has employeeId: string
  has salary: optional int
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
    }

    #[test]
    fn test_parse_property_def() {
        let source = r#"property worksFor: Person -> Organization
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
    }

    #[test]
    fn test_parse_enum_def() {
        let source = r#"concept Status:
  one of:
    Active
    Inactive
    Pending
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
    }

    #[test]
    fn test_parse_rule() {
        let source = r#"

rule EmployeeAccess:
  match:
    ?emp a Employee
    ?emp worksFor ?company
  then:
    ?emp hasAccess true
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());

        let declarations = result.ontology.unwrap().declarations;
        assert_eq!(declarations.len(), 1, "Error: Exactly one rule expected");

        let declaration = declarations.first().unwrap();
        assert!(
            matches!(declaration, crate::Declaration::Rule { .. }),
            "Error: rule expected"
        );

        let crate::Declaration::Rule(rule) = declaration else {
            panic!()
        };
        assert_eq!(rule.name, "EmployeeAccess");
        let match_patterns = rule.match_block.patterns.clone();
        assert_eq!(match_patterns.len(), 2);
        let first = match_patterns
            .first()
            .expect("Match block must have a first pattern");
        assert!(matches!(first, crate::Pattern::Type { .. }));
    }

        #[test]
    fn test_parse_two_rule() {
        let source = r#"
rule flag_unvaccinated:
  match:
    ?animal a Animal
  then:
    ?animal a UnvaccinatedAnimal

# End of rules

rule flag_intern_emergency:
  match:
    ?appt animal [ treatedBy [ a Intern ] ]
  then:
    ?appt a UnsafeAssignment

"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());

        let declarations = result.ontology.unwrap().declarations;
        assert_eq!(declarations.len(), 2, "Error: Exactly two rule expected");

        let declaration = declarations.first().unwrap();
        assert!(
            matches!(declaration, crate::Declaration::Rule { .. }),
            "Error: rule expected"
        );

        let crate::Declaration::Rule(rule) = declaration else {
            panic!()
        };
        assert_eq!(rule.name, "flag_unvaccinated");
        let match_patterns = rule.match_block.patterns.clone();
        assert_eq!(match_patterns.len(), 1);
        let first = match_patterns
            .first()
            .expect("Match block must have a first pattern");
        assert!(matches!(first, crate::Pattern::Type { .. }));
      }

    #[test]
    fn test_parse_with_comments() {
        let source = r#"
# comment in ontology
concept A:
    has x: string  # property comment
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
    }

    #[test]
    fn test_parse_complete_ontology() {
        let source = r#"

concept Department:
  one of:
    engineering
    sales
    marketing
    hr

concept Person:
  has firstName: string
  has lastName: string

concept Employee:
  sub Person
  has employeeId: string
  has salary: float

property worksFor: Employee -> Organization
"#;
        let result = parse_ontology(source);
        assert!(
            result.is_ok(),
            "Errors: {}",
            result.format_diagnostics(Some(source), None)
        );
    }

    #[test]
    fn test_parse_result_format_diagnostics() {
        let source = "concept\n"; // incomplete
        let result = parse_ontology(source);
        if result.has_errors() {
            let formatted = result.format_diagnostics(Some(source), Some("broken.dlf"));
            assert!(!formatted.is_empty());
            assert!(formatted.contains("Expected identifier"));
        }
    }

    #[test]
    fn test_parsing_context_detection() {
        let source = "ontology Foo:\n  concept Bar:\n    has name: string\n";
        let loc = Location::new(3, 5, 30);
        let ctx = identify_parsing_context(source, &loc);
        assert_eq!(ctx, ParsingContext::ConceptBody);
    }

    #[test]
    fn test_parsing_context_top_level() {
        let source = "namespace com.example\n";
        let loc = Location::new(1, 1, 0);
        let ctx = identify_parsing_context(source, &loc);
        assert_eq!(ctx, ParsingContext::TopLevel);
    }

    #[test]
    fn test_parsing_context_enum_body() {
        let source = "ontology Foo:\n  enum Status:\n    active\n";
        let loc = Location::new(3, 5, 30);
        let ctx = identify_parsing_context(source, &loc);
        assert_eq!(ctx, ParsingContext::EnumBody);
    }

    #[test]
    fn test_parsing_context_match_block() {
        let source = "ontology Foo:\n  rule R:\n    match:\n      ?x is Person\n";
        let loc = Location::new(4, 7, 45);
        let ctx = identify_parsing_context(source, &loc);
        assert_eq!(ctx, ParsingContext::MatchBlock);
    }

    #[test]
    fn test_diagnostic_builder_fluent() {
        let span = Span::new(Location::new(1, 5, 4), Location::new(1, 10, 9));
        let diag = DiagnosticBuilder::error(ErrorCode::UnexpectedToken, "Expected ':'")
            .span(span)
            .help("Add a colon after the name")
            .label(span, "here")
            .build();

        assert_eq!(diag.severity, Severity::HardError);
        assert_eq!(diag.code, ErrorCode::UnexpectedToken);
        assert!(diag.help.is_some());
        assert_eq!(diag.labels.len(), 1);
    }

    #[test]
    fn test_span_merge() {
        let s1 = Span::new(Location::new(1, 1, 0), Location::new(1, 5, 4));
        let s2 = Span::new(Location::new(1, 10, 9), Location::new(1, 15, 14));
        let merged = s1.merge(&s2);
        assert_eq!(merged.start.offset, 0);
        assert_eq!(merged.end.offset, 14);
    }

    #[test]
    fn test_parse_result_unwrap_failure() {
        let result = ParseResult::failure(vec![
            DiagnosticBuilder::error(ErrorCode::UnexpectedToken, "bad token").build(),
        ]);
        // Can't call unwrap_program directly in Rust (it needs Python),
        // but we can check the fields:
        assert!(result.ontology.is_none());
        assert!(result.has_errors());
        assert!(!result.is_ok());
    }
}
