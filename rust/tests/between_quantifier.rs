//! `between N, M` quantifier: `between` used to lex as a plain name, so the
//! documented form failed with "Unexpected '1' in match block".
use rowl::ast::{Declaration, Pattern, Quantifier};
use rowl::parser::parse_ontology;

#[test]
fn between_quantifier_parses() {
    let src = "\
rule r:
  match:
    ?c a Customer
    between 1, 3 ?r [ a Rental ]:
      ?r customer ?c
  then:
    ?c a Regular
";
    let parsed = parse_ontology(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let ontology = parsed.ontology.unwrap();
    let Declaration::Rule(rule) = &ontology.declarations[0] else { panic!("not a rule") };
    assert!(matches!(
        &rule.match_block.patterns[1],
        Pattern::Quantified { quantifier: Quantifier::Between { .. }, .. }
    ));
}
