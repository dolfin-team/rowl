//! Core parsing logic for Dolfin.

use lalrpop_util::lalrpop_mod;

// Auto-generated LALRPOP parser for Dolfin language
lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(dead_code)]
    #[allow(unused_variables)]
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
        assert_eq!(onto.iri_name, Some(crate::ast::IriNameValue::LocalSegment("Mammifère".to_string())));
        assert_eq!(onto.declarations.len(), 1);
    }

    #[test]
    fn test_parse_unit_family() {
        let source = "unitdef family vegetables\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.declarations.len(), 1);
        match &onto.declarations[0] {
            crate::ast::Declaration::Unit(u) => {
                assert_eq!(u.name.get().as_str(), "vegetables");
                assert_eq!(u.kind, crate::ast::UnitKind::Family());
            }
            other => panic!("expected Declaration::Unit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_unit_nominal() {
        let source = "unitdef bunch_of_carrots: nominal of vegetables scale 2\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        match &onto.declarations[0] {
            crate::ast::Declaration::Unit(u) => {
                assert_eq!(u.name.get().as_str(), "bunch_of_carrots");
                match &u.kind {
                    crate::ast::UnitKind::Nominal { family, scale } => {
                        assert_eq!(family.full(), "vegetables");
                        assert_eq!(*scale, 2.0);
                    }
                    other => panic!("expected Nominal, got {:?}", other),
                }
            }
            other => panic!("expected Declaration::Unit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_unit_derived() {
        let source = "unitdef USD: scale 0.92 EUR\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        match &onto.declarations[0] {
            crate::ast::Declaration::Unit(u) => {
                assert_eq!(u.name.get().as_str(), "USD");
                match &u.kind {
                    crate::ast::UnitKind::Derived { scale, reference } => {
                        assert_eq!(*scale, 0.92);
                        assert_eq!(reference, "EUR");
                    }
                    other => panic!("expected Derived, got {:?}", other),
                }
            }
            other => panic!("expected Declaration::Unit, got {:?}", other),
        }
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
        // Failure result: no ontology, errors present.
        assert!(result.ontology.is_none());
        assert!(result.has_errors());
        assert!(!result.is_ok());
    }

    #[test]
    fn test_parse_fact_simple() {
        let source = "fact rex a Dog\n  name \"Rex\"\n  weight 45.0\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.facts().len(), 1);
        let fact = &onto.facts()[0];
        assert_eq!(fact.id, "rex");
        assert_eq!(fact.types.len(), 1);
        assert_eq!(fact.types[0].full(), "Dog");
        assert_eq!(fact.assertions.len(), 2);
    }

    #[test]
    fn test_parse_fact_multiple_types() {
        let source = "fact felix a Cat, NeverVaccinated\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let fact = &onto.facts()[0];
        assert_eq!(fact.types.len(), 2);
        assert_eq!(fact.types[0].full(), "Cat");
        assert_eq!(fact.types[1].full(), "NeverVaccinated");
    }

    #[test]
    fn test_parse_fact_with_reference() {
        let source = "fact rex a Dog\n  owner :John\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let fact = &onto.facts()[0];
        assert_eq!(fact.assertions.len(), 1);
        match &fact.assertions[0] {
            crate::ast::FactAssertion::Property { property, values, .. } => {
                assert_eq!(property.to_string(), "owner");
                assert_eq!(values.len(), 1);
                match &values[0] {
                    crate::ast::FactValue::Reference { qualifier, name, .. } => {
                        assert!(qualifier.is_none());
                        assert_eq!(name, "John");
                    }
                    _ => panic!("Expected Reference"),
                }
            }
            _ => panic!("Expected Property assertion"),
        }
    }

    #[test]
    fn test_parse_fact_anonymous_block() {
        let source = "fact rex a Dog\n  vaccination [\n    vaccine_name \"Davies\"\n    date_administered \"2024-04-23\"\n  ]\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let fact = &onto.facts()[0];
        match &fact.assertions[0] {
            crate::ast::FactAssertion::Property { property, values, .. } => {
                assert_eq!(property.to_string(), "vaccination");
                assert_eq!(values.len(), 1);
                match &values[0] {
                    crate::ast::FactValue::Block { assertions, .. } => {
                        assert_eq!(assertions.len(), 2);
                    }
                    _ => panic!("Expected Block"),
                }
            }
            _ => panic!("Expected Property assertion"),
        }
    }

    #[test]
    fn test_parse_full_fact_example() {
        let source = "fact rex a Dog\n  name \"Rex\"\n  weight 45.0\n  neutered true\n  owner :John\n  vaccinations [\n    vaccine_name \"Davies\"\n    date_administered \"2024-04-23\"\n  ]\n\nfact felix a Cat\n  name \"Felix\"\n  weight 5.2\n  indoor true\n  owner :John\n\nfact John a Owner\n  first_name \"John\"\n  last_name \"Smith\"\n  phone_numbers \"555-1234\"\n  preferred_vet :DrPortbridge\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Errors: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.facts().len(), 3);
    }

    #[test]
    fn test_parse_fact_inverse() {
        let source = "fact john a Person\n  is spouse of :mary\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let fact = &onto.facts()[0];
        match &fact.assertions[0] {
            crate::ast::FactAssertion::Inverse { property, .. } => {
                assert_eq!(property.full(), "spouse");
            }
            _ => panic!("Expected Inverse assertion"),
        }
    }

    #[test]
    fn test_parse_fact_type_hint_in_block() {
        let source = "fact rex a Dog\n  vaccination [\n    a VaccinationRecord\n    name \"Davies\"\n  ]\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let fact = &onto.facts()[0];
        match &fact.assertions[0] {
            crate::ast::FactAssertion::Property { property, values, .. } => {
                assert_eq!(property.to_string(), "vaccination");
                match &values[0] {
                    crate::ast::FactValue::Block { assertions, .. } => {
                        assert!(
                            assertions.iter().any(|a| matches!(a, crate::ast::FactAssertion::TypeHint { .. })),
                            "expected TypeHint in block assertions"
                        );
                    }
                    _ => panic!("expected Block value"),
                }
            }
            _ => panic!("expected Property assertion"),
        }
    }

    #[test]
    fn test_parse_fact_multivalue_list() {
        let source = "fact john a Person\n  phone \"555-1234\", \"555-5678\"\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let fact = &onto.facts()[0];
        match &fact.assertions[0] {
            crate::ast::FactAssertion::Property { property, values, .. } => {
                assert_eq!(property.to_string(), "phone");
                assert_eq!(values.len(), 2, "expected 2 values in comma-separated list");
            }
            _ => panic!("expected Property assertion"),
        }
    }

    #[test]
    fn test_parse_fact_bare() {
        let source = "fact rex a Dog\n";
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let fact = &onto.facts()[0];
        assert_eq!(fact.id, "rex");
        assert_eq!(fact.types.len(), 1);
        assert!(fact.assertions.is_empty(), "bare fact should have no assertions");
    }

    #[test]
    fn test_property_axioms_standalone() {
        let source = r#"property loves: People -> People
  symmetric
  reflexive
  sub friend_of
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let props = onto.properties();
        assert_eq!(props.len(), 1);
        let p = &props[0];
        assert_eq!(p.axioms.len(), 3);
        assert!(matches!(p.axioms[0], crate::ast::PropertyAxiom::Symmetric { .. }));
        assert!(matches!(p.axioms[1], crate::ast::PropertyAxiom::Reflexive { .. }));
        assert!(matches!(p.axioms[2], crate::ast::PropertyAxiom::Sub { .. }));
    }

    #[test]
    fn test_property_axioms_in_has_declaration() {
        let source = r#"concept Appointment:
  has organizer: one People
    inverse of organizes
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let concepts = onto.concepts();
        assert_eq!(concepts.len(), 1);
        let has = &concepts[0].has_declarations[0];
        assert_eq!(has.axioms.len(), 1);
        match &has.axioms[0] {
            crate::ast::PropertyAxiom::InverseOf { property, .. } => {
                assert_eq!(property.full(), "organizes");
            }
            _ => panic!("Expected InverseOf axiom"),
        }
    }

    #[test]
    fn test_property_axioms_transitive_equivalent() {
        let source = r#"property friend_of: People -> People
  transitive
  equivalent to knows
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let props = onto.properties();
        assert_eq!(props[0].axioms.len(), 2);
        assert!(matches!(props[0].axioms[0], crate::ast::PropertyAxiom::Transitive { .. }));
        match &props[0].axioms[1] {
            crate::ast::PropertyAxiom::EquivalentTo { path, .. } => {
                match path {
                    crate::ast::PropertyPath::Name { name, .. } => assert_eq!(name.full(), "knows"),
                    _ => panic!("Expected simple name path"),
                }
            }
            _ => panic!("Expected EquivalentTo axiom"),
        }
    }

    #[test]
    fn test_property_path_sequence() {
        let source = r#"property grand_parent: Human -> Human
  equivalent to (parent / parent)
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let p = &onto.properties()[0];
        match &p.axioms[0] {
            crate::ast::PropertyAxiom::EquivalentTo { path, .. } => {
                assert!(matches!(path, crate::ast::PropertyPath::Sequence { steps, .. } if steps.len() == 2));
            }
            _ => panic!("Expected EquivalentTo"),
        }
    }

    #[test]
    fn test_property_path_inverse_in_sequence() {
        let source = r#"property sibling: Human -> Human
  equivalent to (parent / ^parent)
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        match &onto.properties()[0].axioms[0] {
            crate::ast::PropertyAxiom::EquivalentTo { path, .. } => {
                match path {
                    crate::ast::PropertyPath::Sequence { steps, .. } => {
                        assert_eq!(steps.len(), 2);
                        assert!(matches!(&steps[1], crate::ast::PropertyPath::Inverse { .. }));
                    }
                    _ => panic!("Expected sequence"),
                }
            }
            _ => panic!("Expected EquivalentTo"),
        }
    }

    #[test]
    fn test_property_path_one_or_more() {
        let source = r#"property ancestor: Human -> Human
  equivalent to parent+
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        match &onto.properties()[0].axioms[0] {
            crate::ast::PropertyAxiom::EquivalentTo { path, .. } => {
                assert!(matches!(path, crate::ast::PropertyPath::OneOrMore { .. }));
            }
            _ => panic!("Expected EquivalentTo"),
        }
    }

    #[test]
    fn test_parse_prefixed_name_in_type_ref() {
        let source = r#"prefix people.Person as P

concept Employee:
  sub P:Person
  has manager: P:Person
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let concept = &onto.concepts()[0];
        // sub P:Person → parents: [TypeRef::Named { name: ["P", "Person"] }]
        assert_eq!(concept.parents.len(), 1);
        match &concept.parents[0] {
            crate::ast::TypeRef::Named { name, .. } => assert_eq!(name.parts, vec!["P", "Person"]),
            _ => panic!("expected Named TypeRef"),
        }
    }

    #[test]
    fn test_parse_uri_prefix_with_prefixed_name_sub() {
        let source = r#"prefix <http://example.com/onto/> as ee

concept Machin:
  sub ee:Truc
"#;
        let result = parse_ontology(source);
        assert!(result.is_ok(), "parse errors: {:?}", result.errors());
        let onto = result.ontology.unwrap();

        // Prefix declaration: path is a URI literal, alias is "ee"
        assert_eq!(onto.prefixes.len(), 1);
        let prefix = &onto.prefixes[0];
        assert_eq!(prefix.alias, "ee");
        assert_eq!(prefix.path.parts, vec!["http://example.com/onto/"]);
        assert!(!prefix.path.is_prefixed);

        // sub ee:Truc → TypeRef::Named with is_prefixed=true, parts=["ee","Truc"]
        let concept = &onto.concepts()[0];
        assert_eq!(concept.parents.len(), 1);
        match &concept.parents[0] {
            crate::ast::TypeRef::Named { name, .. } => {
                assert_eq!(name.parts, vec!["ee", "Truc"]);
                assert!(name.is_prefixed, "ee:Truc should have is_prefixed=true");
            }
            _ => panic!("expected Named TypeRef"),
        }
    }

    #[test]
    fn test_example_queries_file_parses() {
        let src = include_str!("../../public/examples/happypaws/queries.dlf");
        let result = parse_ontology(src);
        assert!(result.is_ok(), "queries.dlf failed: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.queries().len(), 3, "expected 3 queries");
        assert_eq!(onto.rules().len(), 2, "expected 2 rules");
    }

    // ── §13 Query language parse tests ─────────────────────────────────────

    #[test]
    fn test_query_13_1_simple_typed_subject() {
        let src = concat!(
            "query movies_released_year:\n",
            "  a ex:Movie\n",
            "    ex:title ?title\n",
            "    ex:year ?year\n",
            "    ex:rating ?rating\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.1 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        assert_eq!(q.name, "movies_released_year");
        assert_eq!(q.body.clauses.len(), 1);
        match &q.body.clauses[0] {
            crate::ast::QueryClause::SubjectPattern(sp) => {
                assert!(sp.subject.is_none());
                assert!(sp.type_ref.is_some());
                assert_eq!(sp.properties.len(), 3);
            }
            other => panic!("expected SubjectPattern, got {:?}", other),
        }
    }

    #[test]
    fn test_query_13_2_inline_filters() {
        let src = concat!(
            "query good_movies_after_2010:\n",
            "  a ex:Movie\n",
            "    ex:title ?title\n",
            "    ex:year [?year > 2010]\n",
            "    ex:rating [?rating > 7.5]\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.2 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        assert_eq!(q.name, "good_movies_after_2010");
        assert_eq!(q.body.clauses.len(), 1);
        match &q.body.clauses[0] {
            crate::ast::QueryClause::SubjectPattern(sp) => {
                assert_eq!(sp.properties.len(), 3);
                match &sp.properties[1] {
                    crate::ast::PropertyPattern::Constrained { block, .. } => {
                        assert_eq!(block.constraints.len(), 1);
                        match &block.constraints[0] {
                            crate::ast::Constraint::Comparison { binding, .. } => {
                                assert!(binding.is_some());
                            }
                            other => panic!("expected Comparison constraint, got {:?}", other),
                        }
                    }
                    other => panic!("expected Constrained property, got {:?}", other),
                }
            }
            other => panic!("expected SubjectPattern, got {:?}", other),
        }
    }

    #[test]
    fn test_query_13_3_return_with_order() {
        let src = concat!(
            "query sorted_movies:\n",
            "  a ex:Movie\n",
            "    ex:title ?title\n",
            "    ex:rating ?rating\n",
            "  return\n",
            "    title ?title\n",
            "    rating ?rating\n",
            "      order desc\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.3 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        assert_eq!(q.name, "sorted_movies");
        let ret = q.body.return_block.as_ref().expect("return block");
        assert_eq!(ret.columns.len(), 2);
        assert_eq!(ret.columns[0].alias.as_deref(), Some("title"));
        assert_eq!(ret.columns[1].order, Some(crate::ast::OrderDir::Desc));
    }

    #[test]
    fn test_query_13_4_return_with_limit() {
        let src = concat!(
            "query top_10_movies:\n",
            "  a ex:Movie\n",
            "    ex:title ?title\n",
            "    ex:rating ?rating\n",
            "  return\n",
            "    ?title\n",
            "    ?rating\n",
            "      order desc\n",
            "    limit 10\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.4 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        let ret = q.body.return_block.as_ref().expect("return block");
        assert_eq!(ret.columns.len(), 2);
        assert_eq!(ret.limit, Some(10));
    }

    #[test]
    fn test_query_13_5_nested_subject_block_with_optional() {
        // spec uses @optional; implementation uses bare `optional`
        let src = concat!(
            "query directors_with_name_and_year:\n",
            "  ?_ ex:director [\n",
            "    ex:name ?directorName\n",
            "    optional ex:birthYear ?birthYear\n",
            "  ]\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.5 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        assert_eq!(q.name, "directors_with_name_and_year");
        assert_eq!(q.body.clauses.len(), 1);
        match &q.body.clauses[0] {
            crate::ast::QueryClause::SubjectPattern(sp) => {
                assert_eq!(sp.properties.len(), 1);
                match &sp.properties[0] {
                    crate::ast::PropertyPattern::Nested { block, .. } => {
                        assert_eq!(block.properties.len(), 2);
                        match &block.properties[1] {
                            crate::ast::PropertyPattern::Optional { .. } => {}
                            other => panic!("expected Optional, got {:?}", other),
                        }
                    }
                    other => panic!("expected Nested, got {:?}", other),
                }
            }
            other => panic!("expected SubjectPattern, got {:?}", other),
        }
    }

    #[test]
    fn test_query_13_6_distinct() {
        let src = concat!(
            "query distinct_genres:\n",
            "  a ex:Movie\n",
            "    ex:genre ?genre\n",
            "  return\n",
            "    ?genre\n",
            "      distinct\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.6 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        let ret = q.body.return_block.as_ref().expect("return block");
        assert_eq!(ret.columns.len(), 1);
        assert!(ret.columns[0].distinct);
    }

    #[test]
    fn test_query_13_7_existence_and_inverse() {
        let src = concat!(
            "query directors_not_horror:\n",
            "  ?director a ex:Director\n",
            "    ex:name ?name\n",
            "  some:\n",
            "    is ex:director of ?_movie\n",
            "  none:\n",
            "    ?director is ex:director of [ex:genre \"Horror\"]\n",
            "  return\n",
            "    ?director\n",
            "    ?name\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.7 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        assert_eq!(q.name, "directors_not_horror");
        // clauses: SubjectPattern, ExistenceBlock(negated=false), ExistenceBlock(negated=true)
        assert_eq!(q.body.clauses.len(), 3);
        match &q.body.clauses[1] {
            crate::ast::QueryClause::ExistenceBlock(eb) => assert!(!eb.negated),
            other => panic!("expected ExistenceBlock(some), got {:?}", other),
        }
        match &q.body.clauses[2] {
            crate::ast::QueryClause::ExistenceBlock(eb) => assert!(eb.negated),
            other => panic!("expected ExistenceBlock(none), got {:?}", other),
        }
    }

    #[test]
    fn test_query_13_8_group_by_aggregation() {
        let src = concat!(
            "query movies_per_genre_count:\n",
            "  ?movie a ex:Movie\n",
            "    ex:genre ?genre\n",
            "  group by ?genre\n",
            "    count ?movie as ?movieCount\n",
            "  return\n",
            "    ?genre\n",
            "    ?movieCount\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.8 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        let gb = q.body.group_by.as_ref().expect("group_by block");
        assert_eq!(gb.var, "?genre");
        assert_eq!(gb.specs.len(), 1);
        assert_eq!(gb.specs[0].kind, crate::ast::AggKind::Count);
    }

    #[test]
    fn test_query_13_9_group_by_having() {
        let src = concat!(
            "query popular_genres:\n",
            "  ?movie a ex:Movie\n",
            "    ex:genre ?genre\n",
            "    ex:rating ?rating\n",
            "  group by ?genre\n",
            "    count ?movie as ?movieCount\n",
            "    average ?rating as ?avgRating\n",
            "    ?movieCount >= 10\n",
            "    ?avgRating > 7.0\n",
            "  return\n",
            "    ?genre\n",
            "    ?movieCount\n",
            "    ?avgRating\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.9 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        let q = &onto.queries()[0];
        let gb = q.body.group_by.as_ref().expect("group_by block");
        assert_eq!(gb.specs.len(), 2);
        assert_eq!(gb.having.len(), 2);
    }

    #[test]
    fn test_query_13_10_body_level_aggregation() {
        let src = concat!(
            "query average_per_director:\n",
            "  ?director is ex:director of ?_\n",
            "  average ?rating as ?directorAvg\n",
            "    ?director is ex:director of [ex:rating ?rating]\n",
            "\n",
            "query global_average:\n",
            "  average ?_r as ?globalAvg\n",
            "    ?_r is ex:rating of [a ex:Movie]\n",
            "\n",
            "query above_average_directors:\n",
            "  average_per_director as [\n",
            "    director ?director\n",
            "    directorAvg ?directorAvg\n",
            "  ]\n",
            "  global_average as ?globalAvg\n",
            "  ?directorAvg > ?globalAvg\n",
            "  ?director ex:name ?directorName\n",
            "  return\n",
            "    ?directorName\n",
            "    ?directorAvg\n",
            "      order desc\n",
            "    ?globalAvg\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.10 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.queries().len(), 3);
        let q0 = &onto.queries()[0];
        assert_eq!(q0.name, "average_per_director");
        // clause 0: SubjectPattern (inverse), clause 1: AggregationQuery
        assert_eq!(q0.body.clauses.len(), 2);
        match &q0.body.clauses[1] {
            crate::ast::QueryClause::AggregationQuery(aq) => {
                assert_eq!(aq.kind, crate::ast::AggKind::Average);
                assert_eq!(aq.result_var, "?directorAvg");
            }
            other => panic!("expected AggregationQuery, got {:?}", other),
        }
        let q2 = &onto.queries()[2];
        assert_eq!(q2.name, "above_average_directors");
        // Composition(Named), Composition(Scalar), BoolFilter, SubjectPattern
        assert_eq!(q2.body.clauses.len(), 4);
    }

    #[test]
    fn test_query_13_11_combined() {
        let src = concat!(
            "query movie_count_avg_rating_per_director:\n",
            "  ?movie ex:director ?director\n",
            "    ex:releaseYear [?year > 2000]\n",
            "    ex:rating ?rating\n",
            "  group by ?director\n",
            "    count ?movie as ?movieCount\n",
            "    average ?rating as ?avgRating\n",
            "    ?movieCount >= 3\n",
            "    ?avgRating > 7.5\n",
            "  return\n",
            "    ?director\n",
            "    ?movieCount\n",
            "    ?avgRating\n",
            "\n",
            "query combine_everything:\n",
            "  movie_count_avg_rating_per_director as [\n",
            "    director ?director\n",
            "    movieCount ?movieCount\n",
            "    avgRating ?avgRating\n",
            "  ]\n",
            "  ?director ex:name ?directorName\n",
            "  some:\n",
            "    ?director is ex:director of [ex:won ?_award]\n",
            "  none:\n",
            "    ?director is ex:director of [ex:genre \"Horror\"]\n",
            "  return\n",
            "    ?directorName\n",
            "    ?movieCount\n",
            "    ?avgRating\n",
        );
        let result = parse_ontology(src);
        assert!(result.is_ok(), "§13.11 parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.queries().len(), 2);
        let q0 = &onto.queries()[0];
        // inline value + continuation block: one SubjectPattern with 3 properties
        assert_eq!(q0.body.clauses.len(), 1);
        match &q0.body.clauses[0] {
            crate::ast::QueryClause::SubjectPattern(sp) => {
                assert_eq!(sp.properties.len(), 3);
            }
            other => panic!("expected SubjectPattern, got {:?}", other),
        }
        let gb = q0.body.group_by.as_ref().expect("group_by");
        assert_eq!(gb.specs.len(), 2);
        assert_eq!(gb.having.len(), 2);
        let q1 = &onto.queries()[1];
        // Composition(Named), SubjectPattern, ExistenceBlock(some), ExistenceBlock(none)
        assert_eq!(q1.body.clauses.len(), 4);
    }

    #[test]
    fn test_parse_query_call_in_rule() {
        let source = concat!(
            "rule use_query:\n",
            "  match:\n",
            "    ?x a Cat\n",
            "    find_at_risk\n",
            "      ?x\n",
            "      ?vet=people.Marcel\n",
            "  then:\n",
            "    ?x a RiskyAnimal\n",
        );
        let result = parse_ontology(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.errors());
        let onto = result.ontology.unwrap();
        assert_eq!(onto.rules().len(), 1);
        let rule = &onto.rules()[0];
        assert_eq!(rule.match_block.patterns.len(), 2);
        match &rule.match_block.patterns[1] {
            crate::ast::Pattern::QueryCall { name, args, .. } => {
                assert_eq!(name.last(), "find_at_risk");
                assert_eq!(args.len(), 2);
                match &args[0] {
                    crate::ast::QueryArg::Var { name, .. } => assert_eq!(name, "?x"),
                    _ => panic!("expected Var arg"),
                }
                match &args[1] {
                    crate::ast::QueryArg::Binding { param, .. } => assert_eq!(param, "?vet"),
                    _ => panic!("expected Binding arg"),
                }
            }
            _ => panic!("expected QueryCall pattern"),
        }
    }
}
