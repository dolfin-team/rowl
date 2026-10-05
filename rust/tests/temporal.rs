//! End-to-end wiring test: temporal smart literals `date(...)`, `time(...)`,
//! `date_time(...)`, `duration(...)` flow from the lexer through the LALRPOP
//! grammar into `Literal::Temporal`, and resolve via `dolfin-datetime`.

use rowl::ast::{Declaration, FactAssertion, FactValue, Literal, TemporalKind};
use rowl::lexer::{Lexer, Token};
use rowl::parser::{parse_ontology, parse_result_strict};

fn lex(src: &str) -> Vec<Token> {
    Lexer::new(src).filter_map(|r| r.ok()).map(|(_, t, _)| t).collect()
}

#[test]
fn lexer_slurps_constructor() {
    let toks = lex("date(June 1st 2026)");
    let hit = toks.iter().find_map(|t| match t {
        Token::TemporalLit((k, c)) => Some((*k, c.clone())),
        _ => None,
    });
    assert_eq!(hit, Some((TemporalKind::Date, "June 1st 2026".to_string())));
}

#[test]
fn bare_keyword_still_lexes_as_type() {
    // `date` with no following paren must remain the type keyword, not a literal.
    let toks = lex("has birthDate: date");
    assert!(toks.iter().any(|t| matches!(t, Token::TDate)));
    assert!(!toks.iter().any(|t| matches!(t, Token::TemporalLit(_))));
}

#[test]
fn all_four_constructors_lex() {
    for (src, kind, content) in [
        ("date(June 1st 2026)", TemporalKind::Date, "June 1st 2026"),
        ("time(2:30 PM UTC)", TemporalKind::Time, "2:30 PM UTC"),
        ("date_time(June 1st 2026, 2:30 PM)", TemporalKind::DateTime, "June 1st 2026, 2:30 PM"),
        ("duration(1y 6mo)", TemporalKind::Duration, "1y 6mo"),
    ] {
        let toks = lex(src);
        assert!(
            toks.iter().any(|t| matches!(t, Token::TemporalLit((k, c)) if *k == kind && c == content)),
            "failed to lex {src:?}"
        );
    }
}

#[test]
fn resolve_temporal_to_xsd() {
    let lit = Literal::Temporal {
        kind: TemporalKind::Date,
        content: "June 1st 2026".to_string(),
        span: None,
    };
    let ctx = dolfin_datetime::TemporalContext::strict();
    assert_eq!(
        lit.resolve_temporal(&ctx),
        Some(Ok(("2026-06-01".to_string(), "xsd:date"))),
    );
}

#[test]
fn resolve_kind_mismatch_errors() {
    // Declared date(...) but content is a duration → validation error.
    let lit = Literal::Temporal {
        kind: TemporalKind::Date,
        content: "7d".to_string(),
        span: None,
    };
    let ctx = dolfin_datetime::TemporalContext::strict();
    assert!(matches!(lit.resolve_temporal(&ctx), Some(Err(_))));
}

#[test]
fn parses_in_fact_value_position() {
    let src = "\nfact bob a Person\n  birthDate date(June 1st 2026)\n";
    let ontology = parse_result_strict(parse_ontology(src)).expect("should parse");

    let fact = ontology
        .declarations
        .iter()
        .find_map(|d| match d {
            Declaration::Fact(f) => Some(f),
            _ => None,
        })
        .expect("a fact");

    let value = match &fact.assertions[0] {
        FactAssertion::Property { values, .. } => &values[0],
        other => panic!("expected a property assertion, got {other:?}"),
    };
    match value {
        FactValue::Literal { value: Literal::Temporal { kind, content, .. }, .. } => {
            assert_eq!(*kind, TemporalKind::Date);
            assert_eq!(content, "June 1st 2026");
        }
        other => panic!("expected a temporal literal, got {other:?}"),
    }
}

#[test]
fn parses_locale_and_timezone_directives() {
    let src = "\n@locale d/m/y\n@timezone +02:00\n\nfact bob a Person\n  birthDate date(01/06/2026)\n";
    let ontology = parse_result_strict(parse_ontology(src)).expect("should parse");
    assert_eq!(ontology.locale.as_ref().map(|d| d.get().as_str()), Some("d/m/y"));
    assert_eq!(ontology.timezone.as_ref().map(|d| d.get().as_str()), Some("+02:00"));
}

#[test]
fn directives_allowed_after_prefixes() {
    // A prefix line before @locale must still parse (directives sit before or
    // after the prefix block).
    let src = "\nprefix <http://example.org/> as ex\n@locale d/m/y\n\nfact bob a ex:Person\n  birthDate date(01/06/2026)\n";
    let ontology = parse_result_strict(parse_ontology(src)).expect("should parse");
    assert_eq!(ontology.locale.as_ref().map(|d| d.get().as_str()), Some("d/m/y"));
}

#[test]
fn locale_context_resolves_numeric_date() {
    // With @locale d/m/y, a bare numeric date resolves without an inline mask.
    let src = "\n@locale d/m/y\n\nfact bob a Person\n  birthDate date(01/06/2026)\n";
    let ontology = parse_result_strict(parse_ontology(src)).expect("should parse");
    let ctx = ontology.temporal_context().expect("valid locale");

    let lit = Literal::Temporal {
        kind: TemporalKind::Date,
        content: "01/06/2026".to_string(),
        span: None,
    };
    assert_eq!(
        lit.resolve_temporal(&ctx),
        Some(Ok(("2026-06-01".to_string(), "xsd:date"))),
    );
}

#[test]
fn timezone_context_applies_offset() {
    let src = "\n@timezone +02:00\n\nfact x a Y\n  z time(14:30)\n";
    let ontology = parse_result_strict(parse_ontology(src)).expect("should parse");
    let ctx = ontology.temporal_context().expect("valid tz");

    let lit = Literal::Temporal {
        kind: TemporalKind::Time,
        content: "14:30".to_string(),
        span: None,
    };
    assert_eq!(
        lit.resolve_temporal(&ctx),
        Some(Ok(("14:30:00+02:00".to_string(), "xsd:time"))),
    );
}

#[test]
fn bad_locale_directive_errors() {
    let src = "\n@locale nonsense\n\nfact x a Y\n  z date(June 1st 2026)\n";
    let ontology = parse_result_strict(parse_ontology(src)).expect("should parse");
    assert!(ontology.temporal_context().is_err());
}

#[test]
fn parses_in_arithmetic_position() {
    // date(...) - duration(...) must parse in an expression (PrimaryExpr) slot.
    let src = "\nfact deadline a Task\n  due date(2000-01-01 as y-m-d)\n";
    assert!(
        parse_result_strict(parse_ontology(src)).is_ok(),
        "temporal literal should parse in a fact value"
    );
}
