//! `at least` / `at most` (space form) parse as both cardinality and quantifier;
//! the `at_least` / `at_most` underscore spellings remain valid aliases.
use rowl::error::Severity;
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
fn space_cardinality_parses() {
    let src = "\
concept Owner:
  has phone_numbers: at least 1 string
  has nicknames: at most 3 string
";
    let errs = errors(src);
    assert!(errs.is_empty(), "space cardinality errored: {errs:?}");
    assert!(parse_ontology(src).ontology.is_some());
}

#[test]
fn underscore_cardinality_still_parses() {
    let src = "\
concept Owner:
  has phone_numbers: at_least 1 string
";
    let errs = errors(src);
    assert!(errs.is_empty(), "underscore cardinality errored: {errs:?}");
}

#[test]
fn space_quantifier_parses() {
    let src = "\
rule r:
  match:
    ?d a Dog
    at least 2 ?v:
      ?d vaccinations ?v
  then:
    ?d a WellVaccinated
";
    let errs = errors(src);
    assert!(errs.is_empty(), "space quantifier errored: {errs:?}");
}
