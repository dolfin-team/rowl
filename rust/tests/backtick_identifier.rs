//! Backtick-escaped identifiers (`` `date` ``) let a name collide with a
//! reserved keyword; backticks are stripped at lex time so the parser sees
//! an ordinary identifier.
use rowl::ast::Declaration;
use rowl::error::Severity;
use rowl::lexer::{Lexer, RESERVED_WORDS, Token};
use rowl::parser::parse_ontology;

fn errors(src: &str) -> Vec<String> {
    parse_ontology(src)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::HardError)
        .map(|d| d.message)
        .collect()
}

#[test]
fn backtick_escapes_keyword_as_property_name() {
    let src = "\
concept Event:
  has `date`: date
";
    let errs = errors(src);
    assert!(errs.is_empty(), "backtick property name errored: {errs:?}");
    let ontology = parse_ontology(src).ontology.expect("should parse");
    let Declaration::Concept(concept) = &ontology.declarations[0] else {
        panic!("expected a concept declaration");
    };
    assert_eq!(concept.has_declarations[0].name, "date");
}

#[test]
fn bare_keyword_as_property_name_still_errors() {
    let src = "\
concept Event:
  has date: date
";
    let errs = errors(src);
    assert!(
        !errs.is_empty(),
        "expected bare `date` as a property name to still be rejected"
    );
}

/// `RESERVED_WORDS` is a hand-maintained mirror of the `#[token(...)]`
/// literals on `RawToken` (see `rowl/src/lexer.rs`), consumed by the LSP for
/// completion/quick-fix hints. Catch it drifting out of sync: every entry
/// must lex bare as something other than `Token::Name`, and lex as
/// `Token::Name(word)` when backtick-wrapped.
#[test]
fn reserved_words_matches_actual_lexer_keywords() {
    for &word in RESERVED_WORDS {
        let bare: Vec<_> = Lexer::new(word)
            .filter_map(|t| t.ok())
            .map(|(_, tok, _)| tok)
            .collect();
        assert!(
            !matches!(bare.as_slice(), [Token::Name(_), ..]),
            "'{word}' is listed in RESERVED_WORDS but lexes as a plain Name — remove it"
        );

        let escaped = format!("`{word}`");
        let wrapped: Vec<_> = Lexer::new(&escaped)
            .filter_map(|t| t.ok())
            .map(|(_, tok, _)| tok)
            .collect();
        assert!(
            matches!(&wrapped[..], [Token::Name(n), ..] if n == word),
            "backtick-escaped '{word}' should lex as Token::Name(\"{word}\"), got {wrapped:?}"
        );
    }
}
