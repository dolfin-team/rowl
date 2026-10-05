//! End-to-end wiring test: physical-quantity smart literals `quantity(...)`
//! flow from the lexer through the LALRPOP grammar into `Literal::Quantity`,
//! and resolve via `dolfin-units`.

use rowl::ast::{Declaration, FactAssertion, FactValue, Literal};
use rowl::lexer::{Lexer, Token};
use rowl::parser::{parse_ontology, parse_result_strict};

fn lex(src: &str) -> Vec<Token> {
    Lexer::new(src).filter_map(|r| r.ok()).map(|(_, t, _)| t).collect()
}

fn quantity_content(toks: &[Token]) -> Option<String> {
    toks.iter().find_map(|t| match t {
        Token::QuantityLit(c) => Some(c.clone()),
        _ => None,
    })
}

#[test]
fn lexer_slurps_constructor() {
    assert_eq!(quantity_content(&lex("quantity(42 km/h)")), Some("42 km/h".to_string()));
}

#[test]
fn regex_captures_nested_paren_exponent() {
    // The exponent form `s^(-2)` contains parens; the slurp regex must capture
    // the whole literal including the outer close paren, not stop at the first.
    assert_eq!(
        quantity_content(&lex("quantity(9.81 m.s^(-2))")),
        Some("9.81 m.s^(-2)".to_string()),
    );
    assert_eq!(
        quantity_content(&lex("quantity(1.609 km.s^(-1))")),
        Some("1.609 km.s^(-1)".to_string()),
    );
}

#[test]
fn conversion_directive_is_captured_in_content() {
    // The trailing `as <unit>` stays inside the slurped content; dolfin-units
    // handles the conversion, not the Dolfin grammar.
    assert_eq!(
        quantity_content(&lex("quantity(42 km/h as m/s)")),
        Some("42 km/h as m/s".to_string()),
    );
}

#[test]
fn bare_keyword_lexes_as_name() {
    // `quantity` with no following paren must remain a plain name, not a literal.
    let toks = lex("has quantity: int");
    assert!(toks.iter().any(|t| matches!(t, Token::Name(n) if n == "quantity")));
    assert!(!toks.iter().any(|t| matches!(t, Token::QuantityLit(_))));
}

#[test]
fn resolve_quantity_computes_si_value() {
    let lit = Literal::Quantity { content: "42 km/h".to_string(), span: None };
    let q = lit.resolve_quantity().expect("is a quantity").expect("valid");
    // 42 km/h = 42 * 1000 / 3600 m/s ≈ 11.667 m/s
    assert!((q.si_value() - 11.6667).abs() < 1e-3, "si_value was {}", q.si_value());
}

#[test]
fn resolve_quantity_applies_as_conversion() {
    let lit = Literal::Quantity { content: "42 km/h as m/s".to_string(), span: None };
    let q = lit.resolve_quantity().expect("is a quantity").expect("valid");
    assert_eq!(q.display_unit, "m/s");
    assert!((q.qty - 11.6667).abs() < 1e-3, "qty was {}", q.qty);
}

#[test]
fn resolve_quantity_unknown_unit_errors() {
    let lit = Literal::Quantity { content: "42 zonks".to_string(), span: None };
    assert!(matches!(lit.resolve_quantity(), Some(Err(_))));
}

#[test]
fn resolve_quantity_none_for_other_literals() {
    let lit = Literal::Int { value: 3, span: None };
    assert!(lit.resolve_quantity().is_none());
}

#[test]
fn parses_in_fact_value_position() {
    let src = "\nfact car a Vehicle\n  topSpeed quantity(42 km/h)\n";
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
        FactValue::Literal { value: Literal::Quantity { content, .. }, .. } => {
            assert_eq!(content, "42 km/h");
        }
        other => panic!("expected a quantity literal, got {other:?}"),
    }
}

#[test]
fn parses_exponent_form_in_fact_value() {
    let src = "\nfact ball a Object\n  accel quantity(9.81 m.s^(-2))\n";
    assert!(
        parse_result_strict(parse_ontology(src)).is_ok(),
        "quantity literal with exponent should parse in a fact value"
    );
}
